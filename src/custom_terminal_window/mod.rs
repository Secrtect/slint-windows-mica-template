//! custom_terminal_window 子模块：与 ui/custom-terminal-window/ 一一对应
//!
//! 提供自绘终端窗口的创建、CBT Hook 防闪烁注入、回调绑定与 Mica 视觉效果。
//! 窗口属性在 ./attributes.rs 中自由 DIY。

pub mod attributes;
pub mod controls;

use crate::CustomTerminalWindow;
use crate::window::{
    CbtHookGuard, apply_mica_effect, center_component_on_active_monitor,
};
use crate::window::borderless::WindowFrame;
use crate::window::controls::TitlebarButtons;
use controls::TerminalTitlebarAdapter;
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::HWND;

/// 终端窗口标题（必须与 .slint 文件中 `title` 属性完全一致，用于 CBT Hook 匹配）
const TERMINAL_TITLE: &str = "自绘终端日志 (Console Output)";

/// 创建并显示自绘终端窗口（含 CBT Hook 防闪烁 + 子类化安装）
pub fn open() -> Result<CustomTerminalWindow, slint::PlatformError> {
    // ── 共享 Frame 持有器：CBT Hook 回调需要访问 WindowFrame 来安装子类化
    //    因为 CBT Hook 在 HCBT_ACTIVATE 时已经有 HWND，但 winit 窗口尚未就绪，
    //    所以不能用 frame.apply()（需要 winit），而是用 frame.apply_to_hwnd(hwnd) ──
    let frame_holder: Arc<Mutex<Option<WindowFrame<CustomTerminalWindow>>>> =
        Arc::new(Mutex::new(None));

    // ── 1. 安装 CBT Hook（按 TERMINAL_TITLE 匹配并在 CreateWindowExW 瞬间注入属性 + 子类化） ──
    let hook_installed = CbtHookGuard::install(
        Some(TERMINAL_TITLE.to_string()),
        {
            let frame_holder = Arc::clone(&frame_holder);
            move |hwnd_isize| {
                // attrs.apply() 使用 windows_sys::HWND (isize)
                let attrs = attributes::get_attributes();
                attrs.apply(hwnd_isize);
                println!("[CustomTerminal] 🎯 CBT Hook 成功捕获窗口并注入 DWM 属性");

                // frame.apply_to_hwnd() 使用 windows::HWND (struct)，需转换
                let hwnd = HWND(hwnd_isize as *mut std::ffi::c_void);
                if let Some(ref frame) = *frame_holder.lock().unwrap() {
                    frame.apply_to_hwnd(hwnd);
                    println!("[CustomTerminal] 🎯 CBT Hook 已通过 HWND 安装 Win32 子类化");
                } else {
                    println!("[CustomTerminal] ⚠️ CBT Hook 回调时 frame 尚未就绪");
                }
            }
        },
    );
    let hook_ok = hook_installed.is_ok();

    if let Ok(guard) = hook_installed {
        println!("[CustomTerminal] CBT Hook 已安装，等待 show() 捕获窗口");
        // CbtHook 在 HCBT_ACTIVATE 触发时会自动自卸载
        std::mem::forget(guard);
    } else {
        println!("[CustomTerminal] CBT Hook 安装失败，将以 fallback 方式运行");
    }

    // ── 2. 创建 CustomTerminalWindow 实例 ──
    let terminal = CustomTerminalWindow::new()?;

    // ── 3. 手动创建无边框框架（子类化由 CBT Hook 在 HCBT_ACTIVATE 时安装） ──
    let buttons = TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    };
    let frame = WindowFrame::new(&terminal, Arc::new(TerminalTitlebarAdapter::new(buttons)));

    // 将 frame 存入共享持有器，供 CBT Hook 回调使用
    *frame_holder.lock().unwrap() = Some(frame.clone());

    // ── 4. 活动屏幕居中定位与防超屏处理 ──
    center_component_on_active_monitor(
        &terminal,
        terminal.get_init_width(),
        terminal.get_init_height(),
    );

    // ── 5. 应用 Mica 特效联动 ──
    apply_mica_effect(&terminal, |t| t.set_is_mica_active(true), hook_ok);

    // ── 6. 绑定 UI 回调 ──
    // 6a. 关闭按钮（⚠️ 标题栏按钮由 Win32 子类化接管，此回调作为 fallback）
    let weak = terminal.as_weak();
    terminal.on_close_requested(move || {
        if let Some(t) = weak.upgrade() {
            let _ = t.window().hide();
            println!("[CustomTerminal] 窗口已隐藏（via Slint callback fallback）");
        }
    });

    // 6b. 标题栏拖拽
    let frame_drag = frame.clone();
    terminal.on_drag(move || {
        frame_drag.drag();
    });

    // 6c. 复制日志按钮
    let weak_log = terminal.as_weak();
    terminal.on_copy_log(move || {
        if let Some(_t) = weak_log.upgrade() {
            println!("[CustomTerminal] 复制日志（功能预留）");
        }
    });

    // ── 7. 显示窗口（触发 CreateWindowExW → CBT Hook → apply_to_hwnd）
    //    apply_to_hwnd 中已通过 DwmExtendFrameIntoClientArea 启用阴影，
    //    阴影在窗口显示前就已就绪，无需额外调度 ──
    terminal.show()?;

    Ok(terminal)
}