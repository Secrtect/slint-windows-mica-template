//! 轻量级 CBT Hook 模块：在窗口创建瞬间提供 HWND，由调用方决定如何处理
//!
//! Lightweight CBT Hook module: provides HWND at window creation,
//! letting the caller decide what to do with it.
//!
//! # 设计原则 / Design Principles
//!
//! - **只负责 Hook + 识别窗口 + 交付 HWND**，不包含任何 DWM 属性逻辑
//! - **不干预 borderless 逻辑**，自绘标题栏按钮的 hover/click 功能完全保留
//! - Hook 在回调完成后**立即自卸载**，不长期驻留
//! - 使用 `windows-sys` crate（与项目现有依赖一致）

#[cfg(target_os = "windows")]
#[allow(dead_code, unused_imports)]
mod inner {
    use std::cell::RefCell;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetParent, GetWindow, SetWindowsHookExW,
        UnhookWindowsHookEx, CBT_CREATEWNDW, GW_OWNER, HCBT_ACTIVATE, HCBT_CREATEWND,
        HHOOK, WH_CBT,
    };

    // ────────────────────────────────────────────────────────────────────────
    // Hook 内部上下文（thread-local）
    // Hook internal context (thread-local)
    // ────────────────────────────────────────────────────────────────────────

    struct HookContext {
        /// 捕获到 UI HWND 后执行的回调
        /// Callback to execute when UI HWND is captured
        on_hwnd_ready: Option<Box<dyn FnOnce(HWND)>>,
        hook: HHOOK,
        /// 已识别的目标 UI 窗口 HWND（在 HCBT_CREATEWND 中设置）
        /// Target UI window HWND identified during HCBT_CREATEWND
        target: Option<HWND>,
        /// 回调是否已执行（在 HCBT_ACTIVATE 中设置）
        /// Whether the callback has been executed (set during HCBT_ACTIVATE)
        applied: bool,
    }

    thread_local! {
        static ACTIVE_CONTEXT: RefCell<Option<HookContext>> = const { RefCell::new(None) };
    }

    // ────────────────────────────────────────────────────────────────────────
    // CbtHookGuard：RAII 封装
    // CbtHookGuard: RAII wrapper
    // ────────────────────────────────────────────────────────────────────────

    /// CBT Hook 的 RAII 守卫
    /// 安装后在 HCBT_ACTIVATE 中自动回调并卸载；
    /// 若 Drop 时 Hook 仍在（无窗口创建），也会自动卸载。
    ///
    /// RAII guard for the CBT Hook.
    /// After installation, it automatically calls back on HCBT_ACTIVATE and uninstalls;
    /// if the hook is still active when dropped (no window was created), it also uninstalls.
    pub struct CbtHookGuard {
        hook: Option<HHOOK>,
        applied: bool,
    }

    impl CbtHookGuard {
        /// 在当前线程安装 WH_CBT Hook
        /// Install a WH_CBT hook on the current thread
        ///
        /// `on_hwnd_ready`：Hook 捕获到 Slint UI 窗口的 HWND 后执行的回调。
        /// 调用方可在此闭包中注入 DWM 属性或执行其他初始化操作。
        ///
        /// `on_hwnd_ready`: Callback executed when the hook captures the Slint UI window's HWND.
        /// The caller can inject DWM attributes or perform other initialization in this closure.
        ///
        /// 返回 `Ok(guard)` 表示安装成功。如果安装失败，返回 `Err` 但不影响程序运行，
        /// 调用方可降级到原有的 `invoke_from_event_loop` 路径。
        ///
        /// Returns `Ok(guard)` on success. On failure returns `Err`, but the caller can
        /// fall back to the existing `invoke_from_event_loop` path.
        pub fn install(on_hwnd_ready: impl FnOnce(HWND) + 'static) -> Result<Self, String> {
            // 清理可能残留的上下文
            // Clean up any leftover context
            ACTIVE_CONTEXT.with(|ctx| {
                *ctx.borrow_mut() = None;
            });

            let hook = unsafe {
                SetWindowsHookExW(WH_CBT, Some(cbt_proc), 0 as _, GetCurrentThreadId())
            };

            if hook == 0 as _ {
                return Err("SetWindowsHookExW(WH_CBT) failed".to_string());
            }

            ACTIVE_CONTEXT.with(|ctx| {
                *ctx.borrow_mut() = Some(HookContext {
                    on_hwnd_ready: Some(Box::new(on_hwnd_ready)),
                    hook,
                    target: None,
                    applied: false,
                });
            });

            println!(
                "[CbtHook] WH_CBT Hook 安装成功 (Thread ID: {})",
                unsafe { GetCurrentThreadId() }
            );

            Ok(Self {
                hook: Some(hook),
                applied: false,
            })
        }

        /// Hook 是否已成功执行了回调
        /// Whether the hook has successfully executed the callback
        pub fn was_applied(&self) -> bool {
            // 从 thread-local 同步最新状态（hook 回调可能已更新）
            // Sync latest state from thread-local (the hook callback may have updated it)
            ACTIVE_CONTEXT.with(|ctx| {
                ctx.borrow()
                    .as_ref()
                    .map(|c| c.applied)
                    .unwrap_or(self.applied)
            })
        }

        /// 手动卸载 Hook
        /// Manually uninstall the hook
        fn uninstall(&mut self) {
            // 同步 applied 状态
            // Sync applied state
            self.applied = ACTIVE_CONTEXT.with(|ctx| {
                ctx.borrow()
                    .as_ref()
                    .map(|c| c.applied)
                    .unwrap_or(self.applied)
            });

            ACTIVE_CONTEXT.with(|ctx| {
                *ctx.borrow_mut() = None;
            });

            if let Some(hook) = self.hook.take() {
                unsafe {
                    UnhookWindowsHookEx(hook);
                }
                println!("[CbtHook] WH_CBT Hook 已卸载 (手动/Drop)");
            }
        }
    }

    impl Drop for CbtHookGuard {
        fn drop(&mut self) {
            self.uninstall();
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // 窗口过滤逻辑
    // Window filtering logic
    // ────────────────────────────────────────────────────────────────────────

    /// 判断 HWND 是否为顶层窗口（无父窗口、无拥有者窗口）
    /// Determine whether an HWND is a top-level window (no parent and no owner)
    fn is_top_level(hwnd: HWND) -> bool {
        unsafe {
            let parent = GetParent(hwnd);
            let owner = GetWindow(hwnd, GW_OWNER);
            (parent == 0 as _) && (owner == 0 as _)
        }
    }

    /// 判断类名是否属于系统/框架内部的辅助窗口
    /// Determine whether a class name belongs to a system/framework helper window
    fn is_system_or_helper_class(class_name: &str, is_atom: bool) -> bool {
        if is_atom {
            return true;
        }
        let lower = class_name.to_lowercase();
        lower.contains("winit thread event target")
            || lower.contains("ime")
            || lower.contains("olemainthread")
            || lower.contains("cicmarshal")
            || lower == "message"
    }

    /// 从 CREATESTRUCTW 中提取类名
    /// Extract class name from CREATESTRUCTW
    unsafe fn extract_class_name(cbt_create: *const CBT_CREATEWNDW) -> (String, bool) {
        if cbt_create.is_null() {
            return (String::new(), false);
        }
        let cs_ptr = unsafe { (*cbt_create).lpcs };
        if cs_ptr.is_null() {
            return (String::new(), false);
        }
        let cs = unsafe { &*cs_ptr };
        if cs.lpszClass.is_null() {
            return (String::new(), false);
        }
        // 判断是否为 Atom（低 16 位指针）
        // Check if it's an Atom (low 16-bit pointer)
        if (cs.lpszClass as usize) <= 0xFFFF {
            return (format!("Atom(#{})", cs.lpszClass as usize), true);
        }
        let mut len = 0;
        while unsafe { *cs.lpszClass.add(len) } != 0 {
            len += 1;
        }
        (
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(cs.lpszClass, len) }),
            false,
        )
    }

    // ────────────────────────────────────────────────────────────────────────
    // WH_CBT 回调
    // WH_CBT callback
    // ────────────────────────────────────────────────────────────────────────

    unsafe extern "system" fn cbt_proc(
        n_code: i32,
        wparam: usize,
        lparam: isize,
    ) -> isize {
        if n_code < 0 {
            return unsafe { CallNextHookEx(0 as _, n_code, wparam, lparam) };
        }

        let hwnd = wparam as HWND;

        match n_code as u32 {
            // ── 阶段 1：CREATEWND 仅识别并记录目标 HWND ──
            // ── Phase 1: CREATEWND only identifies and records target HWND ──
            HCBT_CREATEWND if lparam != 0 => {
                let cbt_create = lparam as *const CBT_CREATEWNDW;
                let (class_name, is_atom) = unsafe { extract_class_name(cbt_create) };

                if !is_system_or_helper_class(&class_name, is_atom)
                    && is_top_level(hwnd)
                    && class_name == "Window Class"
                {
                    ACTIVE_CONTEXT.with(|ctx| {
                        let mut ctx = ctx.borrow_mut();
                        if let Some(ref mut context) = *ctx {
                            if context.target.is_none() {
                                println!(
                                    "[CbtHook] 🎯 识别 Slint UI 顶层窗口 HWND({:?}) (Class: {:?})，等待 ACTIVATE 回调",
                                    hwnd, class_name
                                );
                                context.target = Some(hwnd);
                            }
                        }
                    });
                } else {
                    println!(
                        "[CbtHook] 忽略窗口 HWND({:?}) (Class: {:?}, top_level: {}, is_atom: {})",
                        hwnd, class_name, is_top_level(hwnd), is_atom
                    );
                }
            }

            // ── 阶段 2：ACTIVATE 时执行回调并自卸载 ──
            // ── Phase 2: execute callback on ACTIVATE and self-uninstall ──
            HCBT_ACTIVATE => {
                let should_apply = ACTIVE_CONTEXT.with(|ctx| {
                    let ctx = ctx.borrow();
                    ctx.as_ref()
                        .map(|c| c.target == Some(hwnd) && !c.applied)
                        .unwrap_or(false)
                });

                if should_apply {
                    ACTIVE_CONTEXT.with(|ctx| {
                        let mut ctx = ctx.borrow_mut();
                        if let Some(ref mut context) = *ctx {
                            println!(
                                "[CbtHook] 🚀 ACTIVATE 阶段：为 HWND({:?}) 执行回调",
                                hwnd
                            );

                            // 取出并执行回调
                            // Take and execute the callback
                            if let Some(callback) = context.on_hwnd_ready.take() {
                                callback(hwnd);
                            }
                            context.applied = true;

                            // 回调完成，立即卸载 Hook
                            // Callback complete, immediately uninstall the hook
                            unsafe { UnhookWindowsHookEx(context.hook) };
                            println!("[CbtHook] Hook 已自卸载（回调执行完成）");
                        }
                    });
                }
            }

            _ => {}
        }

        unsafe { CallNextHookEx(0 as _, n_code, wparam, lparam) }
    }
}

// ────────────────────────────────────────────────────────────────────────
// 跨平台导出（非 Windows 平台提供空实现）
// Cross-platform exports (no-op on non-Windows platforms)
// ────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub use inner::*;

/// 非 Windows 平台的空实现
/// No-op implementation for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub mod fallback {
    pub struct CbtHookGuard;

    impl CbtHookGuard {
        pub fn install(_on_hwnd_ready: impl FnOnce(()) + 'static) -> Result<Self, String> {
            Ok(Self)
        }
        pub fn was_applied(&self) -> bool {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub use fallback::*;
