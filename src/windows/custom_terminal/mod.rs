//! custom_terminal 子模块：与 ui/custom-terminal-window/ 一一对应
//! Custom terminal sub-module: corresponds directly to ui/custom-terminal-window/.
//!
//! 提供自绘终端窗口的创建、CBT Hook 防闪烁注入、回调绑定与 Mica 视觉效果。
//! 窗口属性在 ./attributes.rs 中自由 DIY。
//! Provides creation, CBT Hook zero-flicker injection, callback binding, and Mica visual effects for custom terminal window.
//! Window attributes can be freely customized in ./attributes.rs.

pub mod attributes;
pub mod controls;

use crate::CustomTerminalWindow;
use crate::platform::{
    CbtHookGuard, apply_mica_effect, center_component_on_active_monitor,
};
use crate::platform::borderless::WindowFrame;
use crate::platform::controls::TitlebarButtons;
use crate::windows::CloseBehavior;
use controls::TerminalTitlebarAdapter;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ::windows::Win32::Foundation::HWND;

/// 终端窗口标题（必须与 .slint 文件中 `title` 属性完全一致，用于 CBT Hook 匹配）
/// Terminal window title (must match `title` property in .slint file for CBT Hook title matching)
const TERMINAL_TITLE: &str = "自绘终端日志 (Console Output)";

/// 创建并显示自绘终端窗口（含 CBT Hook 防闪烁 + 子类化安装）
/// Create and display custom terminal window (with CBT Hook zero-flicker injection + subclassing)
///
/// # 参数 - Parameters
/// * `handle` - 外部传入的窗口句柄持有器，用于在关闭时销毁窗口组件
///   External handle holder for destroying the window component on close
/// * `close_behavior` - 窗口关闭行为（隐藏 vs 销毁）
///   Window close behavior (hide vs destroy)
pub fn open(
    handle: Rc<RefCell<Option<CustomTerminalWindow>>>,
    close_behavior: CloseBehavior,
) -> Result<(), slint::PlatformError> {
    // ── 共享 Frame 持有器：CBT Hook 回调需要访问 WindowFrame 来安装子类化
    //    因为 CBT Hook 在 HCBT_ACTIVATE 时已经有 HWND，但 winit 窗口尚未就绪，
    //    所以不能用 frame.apply()（需要 winit），而是用 frame.apply_to_hwnd(hwnd) ──
    // ── Shared Frame holder: CBT Hook callback needs access to WindowFrame to install subclassing.
    //    Since CBT Hook has HWND at HCBT_ACTIVATE but winit window is not yet ready,
    //    we use frame.apply_to_hwnd(hwnd) instead of frame.apply() (which requires winit). ──
    let frame_holder: Arc<Mutex<Option<WindowFrame<CustomTerminalWindow>>>> =
        Arc::new(Mutex::new(None));

    // 如果是 Hide 模式，且之前已经创建过实例，则直接重新显示已存在的窗口
    // If in Hide mode and an instance already exists, re-show the existing window directly
    if close_behavior == CloseBehavior::Hide
        && let Some(existing) = handle.borrow().as_ref()
    {
        existing.show()?;
        println!("[CustomTerminal] 复用已有窗口实例并重新显示 (Hide 模式) / Reusing existing instance to re-show (Hide mode)");
        return Ok(());
    }

    // ── 1. 安装 CBT Hook（按 TERMINAL_TITLE 匹配并在 CreateWindowExW 瞬间注入属性 + 子类化） ──
    // ── 1. Install CBT Hook (matches TERMINAL_TITLE and injects attributes + subclassing at CreateWindowExW) ──
    let hook_installed = CbtHookGuard::install(
        Some(TERMINAL_TITLE.to_string()),
        {
            let frame_holder = Arc::clone(&frame_holder);
            move |hwnd_isize| {
                // attrs.apply() 使用 windows_sys::HWND (isize) / attrs.apply() uses windows_sys::HWND (isize)
                let attrs = attributes::get_attributes();
                attrs.apply(hwnd_isize);
                println!("[CustomTerminal] 🎯 CBT Hook 成功捕获窗口并注入 DWM 属性 / Captured window and injected DWM attributes");

                // frame.apply_to_hwnd() 使用 windows::HWND (struct)，需转换
                // frame.apply_to_hwnd() uses windows::HWND (struct), conversion required
                let hwnd = HWND(hwnd_isize as *mut std::ffi::c_void);
                if let Some(ref frame) = *frame_holder.lock().unwrap() {
                    frame.apply_to_hwnd(hwnd);
                    println!("[CustomTerminal] 🎯 CBT Hook 已通过 HWND 安装 Win32 子类化 / Installed Win32 subclassing via HWND");
                } else {
                    println!("[CustomTerminal] ⚠️ CBT Hook 回调时 frame 尚未就绪 / Frame was not ready at CBT Hook callback");
                }
            }
        },
    );
    let hook_ok = hook_installed.is_ok();

    if let Ok(guard) = hook_installed {
        println!("[CustomTerminal] CBT Hook 已安装，等待 show() 捕获窗口 / CBT Hook installed, waiting for show() to capture");
        // CbtHook 在 HCBT_ACTIVATE 触发时会自动自卸载 / CbtHook unhooks itself upon HCBT_ACTIVATE
        std::mem::forget(guard);
    } else {
        println!("[CustomTerminal] CBT Hook 安装失败，将以 fallback 方式运行 / CBT Hook installation failed, running in fallback mode");
    }

    // ── 2. 创建 CustomTerminalWindow 实例 / 2. Create CustomTerminalWindow instance ──
    let terminal = CustomTerminalWindow::new()?;

    // ── 3. 手动创建无边框框架（子类化由 CBT Hook 在 HCBT_ACTIVATE 时安装） ──
    // ── 3. Create borderless frame (subclassing is installed by CBT Hook on HCBT_ACTIVATE) ──
    let buttons = TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    };
    let frame = WindowFrame::new(&terminal, Arc::new(TerminalTitlebarAdapter::new(buttons)));

    // 将 frame 存入共享持有器，供 CBT Hook 回调使用 / Store frame in shared holder for CBT Hook callback
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
    // 6a. 关闭按钮（标题栏按钮与底部按钮统一触发 close-requested）
    //     CloseBehavior::Destroy → 调用 hide() 并清空 handle 句柄持有器
    //     CloseBehavior::Hide → 调用 hide() 并保留 handle 中的组件实例
    // 6a. Close button (titlebar button and dialog button both trigger close-requested)
    //     CloseBehavior::Destroy → Call hide() and clear handle holder
    //     CloseBehavior::Hide → Call hide() and retain component instance in handle
    let weak = terminal.as_weak();
    let handle_for_close = handle.clone();
    terminal.on_close_requested(move || {
        match close_behavior {
            CloseBehavior::Hide => {
                if let Some(t) = weak.upgrade() {
                    let _ = t.window().hide();
                    println!("[CustomTerminal] 窗口已隐藏 (Hide) / Window hidden (Hide)");
                }
            }
            CloseBehavior::Destroy => {
                if let Some(t) = weak.upgrade() {
                    let _ = t.window().hide();
                }
                *handle_for_close.borrow_mut() = None;
                println!("[CustomTerminal] 窗口已关闭并释放资源 (Destroy) / Window closed and destroyed (Destroy)");
            }
        }
    });

    // 6b. 标题栏拖拽 / 6b. Titlebar dragging
    let frame_drag = frame.clone();
    terminal.on_drag(move || {
        frame_drag.drag();
    });

    // 6c. 最大化状态变化监听（切换标题栏最大化/还原图标）
    // 6c. Monitor maximized state changes (toggle titlebar maximize/restore icon)
    frame.on_maximized_changed(move |terminal, is_max| {
        terminal.set_titlebar_maximized(is_max);
    });

    // 6d. 双击标题栏切换最大化/还原
    // 6d. Double-click titlebar to toggle maximize/restore
    let frame_dbl = frame.clone();
    terminal.on_double_click(move || {
        frame_dbl.toggle_maximized();
    });

    // 6e. 复制日志按钮
    // 6e. Copy log button
    let weak_log = terminal.as_weak();
    terminal.on_copy_log(move || {
        if let Some(_t) = weak_log.upgrade() {
            println!("[CustomTerminal] 复制日志（功能预留） / Copy log (feature reserved)");
        }
    });

    // ── 7. 显示窗口（触发 CreateWindowExW → CBT Hook → apply_to_hwnd）
    // ── 7. Show window (triggers CreateWindowExW -> CBT Hook -> apply_to_hwnd) ──
    terminal.show()?;

    // ── 8. 将窗口组件存入外部句柄持有器
    // ── 8. Store the window component into the external handle holder ──
    *handle.borrow_mut() = Some(terminal);

    Ok(())
}