//! native_terminal_window 子模块：与 ui/native-terminal-window/ 一一对应
//!
//! 提供原生终端窗口的创建、CBT Hook 防闪烁注入、回调绑定与 Mica 视觉效果。
//! 窗口属性在 ./attributes.rs 中自由 DIY。

pub mod attributes;

use crate::NativeTerminalWindow;
use crate::window::{
    CbtHookGuard, TitlebarSetup, apply_mica_effect, center_component_on_active_monitor,
};
use slint::ComponentHandle;

/// 终端窗口标题（必须与 .slint 文件中 `title` 属性完全一致，用于 CBT Hook 匹配）
const NATIVE_TERMINAL_TITLE: &str = "原生终端日志 (Console Output)";

/// 创建并显示原生终端窗口（含 CBT Hook 防闪烁）
pub fn open() -> Result<NativeTerminalWindow, slint::PlatformError> {
    // ── 1. 安装 CBT Hook ──
    let hook_installed = CbtHookGuard::install(
        Some(NATIVE_TERMINAL_TITLE.to_string()),
        move |hwnd| {
            let attrs = attributes::get_attributes();
            attrs.apply(hwnd);
            println!("[NativeTerminal] 🎯 CBT Hook 成功捕获窗口并注入属性");
        },
    );
    let hook_ok = hook_installed.is_ok();

    if let Ok(guard) = hook_installed {
        println!("[NativeTerminal] CBT Hook 已安装，等待 show() 捕获窗口");
        std::mem::forget(guard);
    } else {
        println!("[NativeTerminal] CBT Hook 安装失败，将以 fallback 方式运行");
    }

    // ── 2. 创建 NativeTerminalWindow 实例 ──
    let terminal = NativeTerminalWindow::new()?;

    // ── 3. 挂载通用无边框拉伸边框与阴影支持 ──
    let frame = terminal.as_weak().setup_borderless_simple()?;

    // ── 4. 活动屏幕居中定位与防超屏处理 ──
    center_component_on_active_monitor(
        &terminal,
        terminal.get_init_width(),
        terminal.get_init_height(),
    );

    // ── 5. 应用 Mica 特效联动 ──
    apply_mica_effect(&terminal, |t| t.set_is_mica_active(true), hook_ok);

    // ── 6. 绑定 UI 回调 ──
    let weak = terminal.as_weak();
    terminal.on_close_requested(move || {
        if let Some(t) = weak.upgrade() {
            let _ = t.window().hide();
            println!("[NativeTerminal] 窗口已隐藏");
        }
    });

    let frame_drag = frame.clone();
    terminal.on_drag(move || {
        frame_drag.drag();
    });

    let weak_log = terminal.as_weak();
    terminal.on_copy_log(move || {
        if let Some(_t) = weak_log.upgrade() {
            println!("[NativeTerminal] 复制日志（功能预留）");
        }
    });

    // ── 7. 显示窗口 ──
    terminal.show()?;

    Ok(terminal)
}
