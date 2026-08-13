//! 轻量级 CBT Hook 模块：在窗口创建瞬间注入 DWM 属性，杜绝多窗口闪烁
//!
//! Lightweight CBT Hook module: injects DWM attributes at window creation to prevent
//! multi-window flickering.
//!
//! # 设计原则 / Design Principles
//!
//! - **只注入 DWM 属性**（Mica / 暗色模式 / 圆角），不修改窗口样式 (`WS_STYLE`)
//! - **不干预 borderless 逻辑**，自绘标题栏按钮的 hover/click 功能完全保留
//! - Hook 在注入完成后**立即自卸载**，不长期驻留
//! - 使用 `windows-sys` crate（与项目现有依赖一致）

#[cfg(target_os = "windows")]
#[allow(dead_code, unused_imports)]
mod inner {
    use std::cell::RefCell;
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND, DWMWCP_ROUNDSMALL,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetClassNameW, GetParent, GetWindow, SetWindowsHookExW,
        UnhookWindowsHookEx, CBT_CREATEWNDW, GW_OWNER, HCBT_ACTIVATE, HCBT_CREATEWND,
        HHOOK, WH_CBT,
    };

    // ────────────────────────────────────────────────────────────────────────
    // DWM Backdrop 类型常量（windows-sys 0.52 未导出这些值，手动定义）
    // DWM Backdrop type constants (not exported by windows-sys 0.52)
    // ────────────────────────────────────────────────────────────────────────
    const DWMSBT_NONE: u32 = 1;
    const DWMSBT_MAINWINDOW: u32 = 2; // Mica
    const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic
    const DWMSBT_TABBEDWINDOW: u32 = 4; // Tabbed

    /// 未文档化的 DWMWA_MICA_EFFECT 属性（适用于旧版 Win11 22000-22522）
    /// Undocumented DWMWA_MICA_EFFECT attribute (for older Win11 builds 22000-22522)
    const DWMWA_MICA_EFFECT: u32 = 1029;

    // ────────────────────────────────────────────────────────────────────────
    // DwmPreset：声明式 DWM 属性预设
    // DwmPreset: declarative DWM attribute preset
    // ────────────────────────────────────────────────────────────────────────

    /// DWM 背景材质类型
    /// DWM backdrop material type
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DwmBackdrop {
        /// Mica 效果（Windows 11 推荐主窗口材质）
        /// Mica effect (recommended for main windows on Windows 11)
        Mica,
        /// Acrylic 效果
        /// Acrylic effect
        Acrylic,
        /// Tabbed 效果（用于标签页窗口）
        /// Tabbed effect (for tabbed windows)
        Tabbed,
    }

    /// 圆角偏好
    /// Corner preference
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CornerPreference {
        /// 圆角 / Rounded
        Round,
        /// 小圆角 / Small rounded
        RoundSmall,
        /// 不圆角 / Not rounded
        DoNotRound,
    }

    /// 声明式 DWM 属性预设，通过 Builder 模式配置
    /// Declarative DWM attribute preset, configured via Builder pattern
    #[derive(Debug, Clone, Default)]
    pub struct DwmPreset {
        backdrop: Option<DwmBackdrop>,
        dark_mode: Option<bool>,
        corner: Option<CornerPreference>,
    }

    impl DwmPreset {
        /// 创建空预设
        /// Create an empty preset
        pub fn new() -> Self {
            Self::default()
        }

        /// 启用 Mica 背景
        /// Enable Mica backdrop
        pub fn with_mica(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Mica);
            self
        }

        /// 启用 Acrylic 背景
        /// Enable Acrylic backdrop
        pub fn with_acrylic(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Acrylic);
            self
        }

        /// 启用 Tabbed 背景
        /// Enable Tabbed backdrop
        pub fn with_tabbed(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Tabbed);
            self
        }

        /// 设置暗色模式
        /// Set dark mode
        pub fn with_dark_mode(mut self, enabled: bool) -> Self {
            self.dark_mode = Some(enabled);
            self
        }

        /// 设置圆角偏好
        /// Set corner preference
        pub fn with_corner(mut self, pref: CornerPreference) -> Self {
            self.corner = Some(pref);
            self
        }

        /// 将预设的 DWM 属性注入到指定 HWND
        /// Apply the preset DWM attributes to the given HWND
        fn apply(&self, hwnd: HWND) {
            // 1. Backdrop (Mica / Acrylic / Tabbed)
            if let Some(backdrop) = self.backdrop {
                let backdrop_type: u32 = match backdrop {
                    DwmBackdrop::Mica => DWMSBT_MAINWINDOW,
                    DwmBackdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
                    DwmBackdrop::Tabbed => DWMSBT_TABBEDWINDOW,
                };
                let result = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_SYSTEMBACKDROP_TYPE as u32,
                        &backdrop_type as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
                if result != 0 && matches!(backdrop, DwmBackdrop::Mica) {
                    // 在旧版 Win11（22000-22522）上回退到未文档化的 MICA_EFFECT
                    // Fall back to undocumented MICA_EFFECT on older Win11 builds
                    let enabled: u32 = 1;
                    let _ = unsafe {
                        DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_MICA_EFFECT,
                            &enabled as *const _ as *const _,
                            size_of::<u32>() as u32,
                        )
                    };
                }
                println!("[CbtHook] 注入 Backdrop({:?}): HWND({:?})", backdrop, hwnd);
            }

            // 2. 暗色模式 / Dark mode
            if let Some(dark) = self.dark_mode {
                let value: u32 = if dark { 1 } else { 0 };
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                        &value as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
                println!("[CbtHook] 注入 DarkMode({}): HWND({:?})", dark, hwnd);
            }

            // 3. 圆角偏好 / Corner preference
            if let Some(corner) = self.corner {
                let pref: u32 = match corner {
                    CornerPreference::Round => DWMWCP_ROUND as u32,
                    CornerPreference::RoundSmall => DWMWCP_ROUNDSMALL as u32,
                    CornerPreference::DoNotRound => DWMWCP_DONOTROUND as u32,
                };
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                        &pref as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
                println!("[CbtHook] 注入 Corner({:?}): HWND({:?})", corner, hwnd);
            }
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // Hook 内部上下文（thread-local）
    // Hook internal context (thread-local)
    // ────────────────────────────────────────────────────────────────────────

    struct HookContext {
        preset: DwmPreset,
        hook: HHOOK,
        /// 已识别的目标 UI 窗口 HWND（在 HCBT_CREATEWND 中设置）
        /// Target UI window HWND identified during HCBT_CREATEWND
        target: Option<HWND>,
        /// DWM 属性是否已成功注入（在 HCBT_ACTIVATE 中设置）
        /// Whether DWM attributes have been successfully injected (set during HCBT_ACTIVATE)
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
    /// 安装后在 HCBT_CREATEWND 中自动注入 DWM 属性并卸载；
    /// 若 Drop 时 Hook 仍在（无窗口创建），也会自动卸载。
    ///
    /// RAII guard for the CBT Hook.
    /// After installation, it automatically injects DWM attributes on HCBT_CREATEWND and uninstalls;
    /// if the hook is still active when dropped (no window was created), it also uninstalls.
    pub struct CbtHookGuard {
        hook: Option<HHOOK>,
        applied: bool,
    }

    impl CbtHookGuard {
        /// 在当前线程安装 WH_CBT Hook
        /// Install a WH_CBT hook on the current thread
        ///
        /// 返回 `Ok(guard)` 表示安装成功。如果安装失败，返回 `Err` 但不影响程序运行，
        /// 调用方可降级到原有的 `invoke_from_event_loop` 路径。
        ///
        /// Returns `Ok(guard)` on success. On failure returns `Err`, but the caller can
        /// fall back to the existing `invoke_from_event_loop` path.
        pub fn install(preset: DwmPreset) -> Result<Self, String> {
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
                    preset,
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

        /// Hook 是否已成功注入了 DWM 属性
        /// Whether the hook has successfully applied DWM attributes
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
                                    "[CbtHook] 🎯 识别 Slint UI 顶层窗口 HWND({:?}) (Class: {:?})，等待 ACTIVATE 注入 DWM",
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

            // ── 阶段 2：ACTIVATE 时注入 DWM 属性并自卸载 ──
            // ── Phase 2: inject DWM attributes on ACTIVATE and self-uninstall ──
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
                                "[CbtHook] 🚀 ACTIVATE 阶段：为 HWND({:?}) 注入 DWM 属性",
                                hwnd
                            );
                            context.preset.apply(hwnd);
                            context.applied = true;

                            // 注入完成，立即卸载 Hook
                            // Injection complete, immediately uninstall the hook
                            unsafe { UnhookWindowsHookEx(context.hook) };
                            println!("[CbtHook] Hook 已自卸载（DWM 注入完成）");
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
    #[derive(Debug, Clone, Default)]
    pub struct DwmPreset;

    impl DwmPreset {
        pub fn new() -> Self {
            Self
        }
        pub fn with_mica(self) -> Self {
            self
        }
        pub fn with_acrylic(self) -> Self {
            self
        }
        pub fn with_tabbed(self) -> Self {
            self
        }
        pub fn with_dark_mode(self, _enabled: bool) -> Self {
            self
        }
    }

    pub struct CbtHookGuard;

    impl CbtHookGuard {
        pub fn install(_preset: DwmPreset) -> Result<Self, String> {
            Ok(Self)
        }
        pub fn was_applied(&self) -> bool {
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub use fallback::*;
