//! 轻量级 CBT Hook 模块：在窗口创建瞬间提供 HWND，由调用方决定如何处理
//! Lightweight CBT Hook module: provides HWND at window creation,
//! letting the caller decide what to do with it.
//!
//! # 设计原则 / Design Principles
//!
//! - **只负责 Hook + 识别窗口 + 交付 HWND**，不强行绑定特定属性逻辑
//! - **提供 100% 开放的逃生通道**：闭包直接接收原始 Win32 HWND
//! - **不干预 borderless 逻辑**，自绘标题栏按钮的 hover/click 功能完全保留
//! - Hook 在回调完成后**立即自卸载**，不长期驻留
//!
//! - **Focused on Hook + Target Matching + HWND Delivery**: Does not enforce specific attribute logic.
//! - **100% Open Escape Hatch**: Directly provides raw Win32 HWND to user closure.
//! - **Zero interference with borderless logic**: Preserves titlebar hover and hit-testing untouched.
//! - **Instant unhook**: Unregisters immediately after callback finishes; never lingers.

#[cfg(target_os = "windows")]
#[allow(dead_code, unused_imports)]
mod inner {
    use std::cell::RefCell;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetParent, GetWindow, GetWindowTextW, SetWindowsHookExW,
        UnhookWindowsHookEx, CBT_CREATEWNDW, GW_OWNER, HCBT_ACTIVATE, HCBT_CREATEWND,
        HHOOK, WH_CBT,
    };

    struct HookContext {
        /// 捕获到 UI HWND 后执行的回调（逃生通道）
        /// Callback executed when UI HWND is captured (escape hatch)
        on_hwnd_ready: Option<Box<dyn FnOnce(HWND)>>,
        hook: HHOOK,
        /// 已识别的目标 UI 窗口 HWND（在 HCBT_CREATEWND 中设置）
        /// Identified target UI window HWND (set in HCBT_CREATEWND)
        target: Option<HWND>,
        /// 目标窗口的标题（可选，如果传入则匹配标题）
        /// Target window title (optional, matches title if provided)
        target_title: Option<String>,
        /// 回调是否已执行（在 HCBT_ACTIVATE 中设置）
        /// Whether callback has been executed (set in HCBT_ACTIVATE)
        applied: bool,
    }

    thread_local! {
        static ACTIVE_CONTEXT: RefCell<Option<HookContext>> = const { RefCell::new(None) };
    }

    /// CBT Hook 的 RAII 守卫
    /// 安装后在 HCBT_ACTIVATE 中自动回调并卸载；
    /// 若 Drop 时 Hook 仍在（无窗口创建），也会自动卸载。
    ///
    /// RAII guard for CBT Hook.
    /// Automatically invokes callback and unhooks in HCBT_ACTIVATE;
    /// unhooks on drop if no window was activated.
    pub struct CbtHookGuard {
        hook: Option<HHOOK>,
        applied: bool,
    }

    impl CbtHookGuard {
        /// 在当前线程安装 WH_CBT Hook
        /// Install WH_CBT Hook on current thread.
        ///
        /// `target_title`：可选窗口标题，若提供则仅对匹配标题的顶层窗口触发
        /// `on_hwnd_ready`：Hook 捕获到 Slint UI 窗口的 HWND 后执行的回调（开放原始 HWND 句柄）。
        ///
        /// `target_title`: Optional title to match against top-level windows.
        /// `on_hwnd_ready`: Closure executed when HWND is captured.
        pub fn install(
            target_title: Option<String>,
            on_hwnd_ready: impl FnOnce(HWND) + 'static,
        ) -> Result<Self, String> {
            // 清理可能残留的上下文 / Clear any residual thread-local context
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
                    target_title,
                    applied: false,
                });
            });

            Ok(Self {
                hook: Some(hook),
                applied: false,
            })
        }

        /// 手动提前卸载 Hook
        /// Manually unhook early
        pub fn uninstall(mut self) {
            self.do_uninstall();
        }

        fn do_uninstall(&mut self) {
            if let Some(h) = self.hook.take() {
                unsafe {
                    UnhookWindowsHookEx(h);
                }
                ACTIVE_CONTEXT.with(|ctx| {
                    *ctx.borrow_mut() = None;
                });
            }
        }
    }

    impl Drop for CbtHookGuard {
        fn drop(&mut self) {
            self.do_uninstall();
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // CBT Hook 回调过程 / CBT Hook Callback Procedure
    // ────────────────────────────────────────────────────────────────────────

    unsafe extern "system" fn cbt_proc(code: i32, wparam: usize, lparam: isize) -> isize {
        if code < 0 {
            return unsafe { CallNextHookEx(0 as _, code, wparam, lparam) };
        }

        match code as u32 {
            HCBT_CREATEWND => {
                let hwnd = wparam as HWND;
                let (is_top_level, window_name) = unsafe {
                    let create_struct = &*(lparam as *const CBT_CREATEWNDW);
                    let cs = &*create_struct.lpcs;

                    let is_child = (cs.style as u32 & windows_sys::Win32::UI::WindowsAndMessaging::WS_CHILD) != 0;
                    let parent = GetParent(hwnd);
                    let owner = GetWindow(hwnd, GW_OWNER);
                    let is_top = !is_child && parent == 0 as _ && owner == 0 as _;

                    let name = if !cs.lpszName.is_null() {
                        let mut len = 0;
                        while *cs.lpszName.add(len) != 0 {
                            len += 1;
                        }
                        String::from_utf16_lossy(std::slice::from_raw_parts(cs.lpszName, len))
                    } else {
                        String::new()
                    };
                    (is_top, name)
                };

                if is_top_level {
                    let matches_target = ACTIVE_CONTEXT.with(|ctx| {
                        let mut borrow = ctx.borrow_mut();
                        if let Some(ref mut c) = *borrow {
                            if let Some(ref title) = c.target_title {
                                if window_name.contains(title.as_str()) || title.contains(&window_name) {
                                    c.target = Some(hwnd);
                                    true
                                } else {
                                    false
                                }
                            } else {
                                c.target = Some(hwnd);
                                true
                            }
                        } else {
                            false
                        }
                    });

                    if matches_target {
                        println!(
                            "[CbtHook] 🎯 HCBT_CREATEWND 捕获到目标 UI 窗口 / Captured target UI window: HWND({:?}), Title: '{}'",
                            hwnd, window_name
                        );
                    }
                }
            }

            HCBT_ACTIVATE => {
                let hwnd = wparam as HWND;

                let should_apply = ACTIVE_CONTEXT.with(|ctx| {
                    let mut borrow = ctx.borrow_mut();
                    if let Some(ref mut c) = *borrow {
                        if !c.applied && (c.target == Some(hwnd) || c.target.is_none()) {
                            c.applied = true;
                            let callback = c.on_hwnd_ready.take();
                            let hook = c.hook;
                            (callback, hook)
                        } else {
                            (None, 0 as _)
                        }
                    } else {
                        (None, 0 as _)
                    }
                });

                if let (Some(callback), hook) = should_apply {
                    println!("[CbtHook] ⚡ HCBT_ACTIVATE 触发，开始执行 HWND 回调... / Triggered, executing HWND callback...");
                    callback(hwnd);
                    println!("[CbtHook] ✅ 回调完成，卸载 CBT Hook / Callback finished, unhooking CBT Hook");

                    if hook != 0 as _ {
                        unsafe {
                            UnhookWindowsHookEx(hook);
                        }
                    }
                    ACTIVE_CONTEXT.with(|ctx| {
                        *ctx.borrow_mut() = None;
                    });
                }
            }

            _ => {}
        }

        unsafe { CallNextHookEx(0 as _, code, wparam, lparam) }
    }
}

#[cfg(target_os = "windows")]
pub use inner::*;

#[cfg(not(target_os = "windows"))]
pub mod fallback {
    pub struct CbtHookGuard;

    impl CbtHookGuard {
        pub fn install<F>(_target_title: Option<String>, _on_hwnd_ready: F) -> Result<Self, String>
        where
            F: FnOnce(()) + 'static,
        {
            Err("CBT Hook is only supported on Windows".to_string())
        }
        pub fn uninstall(self) {}
    }
}

#[cfg(not(target_os = "windows"))]
pub use fallback::*;
