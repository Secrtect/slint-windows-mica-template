//! custom_terminal_window 子模块：与 ui/custom-terminal-window/ 一一对应
//!
//! 提供自绘终端窗口的创建、CBT Hook 防闪烁注入、回调绑定与 Mica 视觉效果。
//!
//! custom_terminal_window submodule: corresponds 1:1 with ui/custom-terminal-window/
//!
//! Provides custom terminal window creation, CBT Hook flicker-free injection,
//! callback binding, and Mica visual effects.

use crate::CustomTerminalWindow;
use crate::app_window::attributes;
use crate::cbt_hook;
use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;

/// 终端窗口标题（必须与 .slint 文件中 `title` 属性完全一致，用于 CBT Hook 匹配）
/// Terminal window title (must exactly match the `title` property in the .slint file for CBT Hook matching)
const TERMINAL_TITLE: &str = "自绘终端日志 (Console Output)";

/// 创建并显示自绘终端窗口（含 CBT Hook 防闪烁）
///
/// 1. 创建窗口实例并绑定回调
/// 2. 安装 CBT Hook（匹配 TERMINAL_TITLE）
/// 3. show() 触发底层 CreateWindowExW → Hook 捕获 HWND 注入 DWM 属性并自卸载
/// 4. 在 Hook 回调中将 Mica 激活与阴影恢复推入 event loop
pub fn open() -> Result<CustomTerminalWindow, slint::PlatformError> {
    // ── 1. 创建 CustomTerminalWindow 实例 ──
    let terminal = CustomTerminalWindow::new()?;

    // ── 2. 绑定 UI 回调 ──

    // 2a. 关闭按钮 → 隐藏窗口
    let weak = terminal.as_weak();
    terminal.on_close_requested(move || {
        if let Some(t) = weak.upgrade() {
            let _ = t.window().hide();
            println!("[CustomTerminal] 窗口已隐藏");
        }
    });

    // 2b. 标题栏拖拽 → winit 原生拖拽
    let weak = terminal.as_weak();
    terminal.on_drag(move || {
        if let Some(t) = weak.upgrade() {
            t.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    });

    // 2c. 复制日志按钮（预留）
    terminal.on_copy_log(|| {
        println!("[CustomTerminal] 复制日志（功能预留）");
    });

    // ── 3. 安装 CBT Hook（在 show() 触发 CreateWindowExW 瞬间注入 DWM 属性） ──
    let weak_for_hook = terminal.as_weak();
    let hook_installed = cbt_hook::CbtHookGuard::install(
        Some(TERMINAL_TITLE.to_string()),
        move |hwnd| {
            // 3a. 在窗口创建/激活瞬间注入 DWM 属性（杜绝初次闪烁）
            let is_dark = attributes::is_system_dark_mode();
            attributes::DwmPreset::new()
                .with_mica()
                .with_dark_mode(is_dark)
                .with_corner(attributes::CornerPreference::Round)
                .apply(hwnd);
            println!("[CustomTerminal] 🎯 CBT Hook 成功捕获窗口并注入 DWM Mica 属性");

            // 3b. DWM 属性注入完成后，在事件循环中激活 UI 透明背景并恢复阴影
            let _ = weak_for_hook.upgrade_in_event_loop(|t| {
                if crate::sys_info::is_win11() {
                    t.set_is_mica_active(true);
                    println!("[CustomTerminal] Mica UI 透明标志已激活");
                }

                // 恢复无边框窗口阴影
                t.window().with_winit_window(|window| {
                    use winit::platform::windows::WindowExtWindows;
                    window.set_undecorated_shadow(true);

                    // Win10 阴影修复
                    if !crate::sys_info::is_win11() {
                        use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
                        use windows::Win32::UI::Controls::MARGINS;
                        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

                        if let Ok(handle) = window.window_handle() {
                            if let RawWindowHandle::Win32(h) = handle.as_raw() {
                                let hwnd = windows::Win32::Foundation::HWND(
                                    h.hwnd.get() as *mut std::ffi::c_void,
                                );
                                let margins = MARGINS {
                                    cxLeftWidth: 0,
                                    cxRightWidth: 0,
                                    cyTopHeight: 0,
                                    cyBottomHeight: 1,
                                };
                                unsafe {
                                    let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
                                }
                            }
                        }
                    }
                });
            });
        },
    );

    if let Ok(guard) = hook_installed {
        println!("[CustomTerminal] CBT Hook 已安装，等待 show() 捕获窗口");
        // 注意：CbtHook 在 HCBT_ACTIVATE 触发时会自动调用 UnhookWindowsHookEx 自卸载。
        // 这里使用 std::mem::forget 防止 guard 在 open() 函数返回时被提前 Drop 卸载。
        std::mem::forget(guard);
    } else {
        println!("[CustomTerminal] CBT Hook 安装失败");
    }

    // ── 4. 请求显示窗口（触发 CreateWindowExW） ──
    terminal.show()?;

    Ok(terminal)
}


