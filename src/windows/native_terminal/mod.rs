//! native_terminal 子模块：与 ui/native-terminal-window/ 一一对应
//! Native terminal sub-module: corresponds directly to ui/native-terminal-window/.
//!
//! 提供原生终端窗口的创建、CBT Hook 防闪烁注入、回调绑定与 Mica 视觉效果。
//! 标题栏按钮由 DWM 系统原生绘制（非 Slint 自绘），Slint 仅负责拖拽区。
//! 窗口属性在 ./attributes.rs 中自由 DIY。
//!
//! Provides creation, CBT Hook zero-flicker injection, callback binding, and Mica visual effects for native terminal window.
//! Titlebar caption buttons are natively rendered by Windows DWM (not Slint custom-drawn); Slint only handles drag region.
//! Window attributes can be freely customized in ./attributes.rs.

pub mod attributes;
mod borderless;

use crate::NativeTerminalWindow;
use crate::platform::{
    CbtHookGuard, apply_mica_effect, center_component_on_active_monitor,
};
use crate::windows::CloseBehavior;
use borderless::NativeCaptionFrame;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ::windows::Win32::Foundation::HWND;

/// 终端窗口标题（必须与 .slint 文件中 `title` 属性完全一致，用于 CBT Hook 匹配）
/// Terminal window title (must match `title` property in .slint file for CBT Hook title matching)
const NATIVE_TERMINAL_TITLE: &str = "原生终端日志 (Console Output)";

/// 创建并显示原生终端窗口（含 CBT Hook 防闪烁 + DWM 原生按钮子类化）
/// Create and display native terminal window (with CBT Hook zero-flicker injection + DWM native caption subclassing)
///
/// # 参数 - Parameters
/// * `handle` - 外部传入的窗口句柄持有器，用于在关闭时销毁窗口组件
///   External handle holder for destroying the window component on close
/// * `close_behavior` - 窗口关闭行为（隐藏 vs 销毁）
///   Window close behavior (hide vs destroy)
pub fn open(
    handle: Rc<RefCell<Option<NativeTerminalWindow>>>,
    close_behavior: CloseBehavior,
) -> Result<(), slint::PlatformError> {
    // ── 共享 Frame 持有器：CBT Hook 回调需要访问 NativeCaptionFrame 来安装子类化 ──
    // ── Shared Frame holder: CBT Hook callback needs access to NativeCaptionFrame to install subclassing ──
    let frame_holder: Arc<Mutex<Option<NativeCaptionFrame>>> =
        Arc::new(Mutex::new(None));

    // 如果是 Hide 模式，且之前已经创建过实例，则直接重新显示已存在的窗口
    // If in Hide mode and an instance already exists, re-show the existing window directly
    if close_behavior == CloseBehavior::Hide
        && let Some(existing) = handle.borrow().as_ref()
    {
        existing.show()?;
        println!("[NativeTerminal] 复用已有窗口实例并重新显示 (Hide 模式) / Reusing existing instance to re-show (Hide mode)");
        return Ok(());
    }

    // ── 1. 安装 CBT Hook（按 NATIVE_TERMINAL_TITLE 匹配并在 CreateWindowExW 瞬间注入属性 + 子类化） ──
    // ── 1. Install CBT Hook (matches NATIVE_TERMINAL_TITLE and injects attributes + subclassing at CreateWindowExW) ──
    let hook_installed = CbtHookGuard::install(
        Some(NATIVE_TERMINAL_TITLE.to_string()),
        {
            let frame_holder = Arc::clone(&frame_holder);
            move |hwnd_isize| {
                // attrs.apply() 使用 windows_sys::HWND (isize)
                // 注入 DWM 属性（Mica、暗色、圆角）以及 WS_CAPTION 样式
                // attrs.apply() uses windows_sys::HWND (isize)
                // Injects DWM attributes (Mica, dark mode, rounded corners) and WS_CAPTION styles
                let attrs = attributes::get_attributes();
                attrs.apply(hwnd_isize);

                // 调试：打印实际窗口样式，验证 WS_POPUP 是否已移除
                // Debug: Print actual window styles to verify WS_POPUP removal
                #[cfg(debug_assertions)]
                {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongPtrW, GWL_STYLE, GWL_EXSTYLE,
                    };
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        WS_POPUP, WS_CAPTION, WS_CHILD,
                    };
                    let style = unsafe { GetWindowLongPtrW(hwnd_isize, GWL_STYLE) } as u32;
                    let ex_style = unsafe { GetWindowLongPtrW(hwnd_isize, GWL_EXSTYLE) } as u32;
                    let has_popup = (style & WS_POPUP) != 0;
                    let has_caption = (style & WS_CAPTION) != 0;
                    let has_overlapped = (style & (WS_POPUP | WS_CHILD)) == 0;
                    // WS_EX_TOOLWINDOW = 0x00000080, WS_EX_APPWINDOW = 0x00040000
                    let has_toolwindow = (ex_style & 0x00000080) != 0;
                    let has_appwindow = (ex_style & 0x00040000) != 0;
                    println!("[NativeTerminal] 🎯 CBT Hook 窗口样式 / Window styles: 0x{:08X}, EX: 0x{:08X}", style, ex_style);
                    println!("[NativeTerminal]    WS_POPUP={}, WS_CAPTION={}, WS_OVERLAPPED={}",
                        has_popup, has_caption, has_overlapped);
                    println!("[NativeTerminal]    WS_EX_TOOLWINDOW={}, WS_EX_APPWINDOW={}",
                        has_toolwindow, has_appwindow);
                }

                println!("[NativeTerminal] 🎯 CBT Hook 成功捕获窗口并注入 DWM 属性 + WS_CAPTION / Captured window and injected DWM attributes + WS_CAPTION");

                // frame.apply_to_hwnd() 使用 windows::HWND (struct)，需转换
                // frame.apply_to_hwnd() uses windows::HWND (struct), conversion required
                let hwnd = HWND(hwnd_isize as *mut std::ffi::c_void);
                if let Some(ref frame) = *frame_holder.lock().unwrap() {
                    frame.apply_to_hwnd(hwnd);
                    println!("[NativeTerminal] 🎯 CBT Hook 已通过 HWND 安装 DWM 原生按钮子类化 / Installed DWM native button subclassing via HWND");
                } else {
                    println!("[NativeTerminal] ⚠️ CBT Hook 回调时 frame 尚未就绪 / Frame was not ready at CBT Hook callback");
                }
            }
        },
    );
    let hook_ok = hook_installed.is_ok();

    if let Ok(guard) = hook_installed {
        println!("[NativeTerminal] CBT Hook 已安装，等待 show() 捕获窗口 / CBT Hook installed, waiting for show() to capture");
        std::mem::forget(guard);
    } else {
        println!("[NativeTerminal] CBT Hook 安装失败，将以 fallback 方式运行 / CBT Hook installation failed, running in fallback mode");
    }

    // ── 2. 创建 NativeTerminalWindow 实例 / 2. Create NativeTerminalWindow instance ──
    let terminal = NativeTerminalWindow::new()?;

    // ── 3. 创建 DWM 原生按钮框架（子类化由 CBT Hook 在 HCBT_ACTIVATE 时安装） ──
    // ── 3. Create DWM native button frame (subclassing installed by CBT Hook at HCBT_ACTIVATE) ──
    let frame = NativeCaptionFrame::new(&terminal);
    *frame_holder.lock().unwrap() = Some(frame.clone());

    // ── 4. 活动屏幕居中定位与防超屏处理 / 4. Center positioning on active monitor with overflow prevention ──
    center_component_on_active_monitor(
        &terminal,
        terminal.get_init_width(),
        terminal.get_init_height(),
    );

    // ── 5. 应用 Mica 特效联动 / 5. Apply Mica visual effect ──
    apply_mica_effect(&terminal, |t| t.set_is_mica_active(true), hook_ok);

    // ── 6. 绑定 UI 回调 / 6. Bind UI callbacks ──
    // 6a. 关闭按钮（底部 Slint 按钮）
    //     CloseBehavior::Destroy → 调用 hide() 并清空 handle 句柄持有器
    //     CloseBehavior::Hide → 调用 hide() 并保留 handle 中的组件实例
    // 6a. Close button (dialog bottom button)
    //     CloseBehavior::Destroy → Call hide() and clear handle holder
    //     CloseBehavior::Hide → Call hide() and retain component instance in handle
    let weak = terminal.as_weak();
    let handle_for_close = handle.clone();
    terminal.on_close_requested(move || {
        match close_behavior {
            CloseBehavior::Hide => {
                if let Some(t) = weak.upgrade() {
                    let _ = t.window().hide();
                    println!("[NativeTerminal] 窗口已隐藏 (Hide) / Window hidden (Hide)");
                }
            }
            CloseBehavior::Destroy => {
                if let Some(t) = weak.upgrade() {
                    let _ = t.window().hide();
                }
                *handle_for_close.borrow_mut() = None;
                println!("[NativeTerminal] 窗口已关闭并释放资源 (Destroy) / Window closed and destroyed (Destroy)");
            }
        }
    });

    // 6b. 标题栏拖拽 / 6b. Titlebar dragging
    let frame_drag = frame.clone();
    terminal.on_drag(move || {
        frame_drag.drag();
    });

    // 6c. 双击标题栏切换最大化/还原 / 6c. Double-click titlebar to toggle maximize/restore
    let frame_dbl = frame.clone();
    terminal.on_double_click(move || {
        frame_dbl.toggle_maximized();
    });

    // 6d. 复制日志按钮 / 6d. Copy log button
    let weak_log = terminal.as_weak();
    terminal.on_copy_log(move || {
        if let Some(_t) = weak_log.upgrade() {
            println!("[NativeTerminal] 复制日志（功能预留） / Copy log (feature reserved)");
        }
    });

    // ── 7. 显示窗口（触发 CreateWindowExW → CBT Hook → apply_to_hwnd）
    //    apply_to_hwnd 中已安装子类化（WM_NCCALCSIZE + WM_NCHITTEST）和阴影，
    //    DWM 原生按钮在窗口首帧前就已就绪 ──
    // ── 7. Show window (triggers CreateWindowExW -> CBT Hook -> apply_to_hwnd)
    //    Subclassing (WM_NCCALCSIZE + WM_NCHITTEST) and shadow are installed in apply_to_hwnd,
    //    so DWM native caption buttons are ready before the first frame renders ──
    terminal.show()?;

    // ── 8. 将窗口组件存入外部句柄持有器
    // ── 8. Store the window component into the external handle holder ──
    *handle.borrow_mut() = Some(terminal);

    Ok(())
}