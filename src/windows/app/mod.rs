//! app 模块：主窗口专属生命周期与业务逻辑
//!
//! 与 ui/app-window/ 一一对应，底层无边框与属性能力复用 crate::platform 基础设施。

pub mod attributes;
pub mod controls;

use crate::AppWindow;
use crate::platform::{
    GlobalWindowControlsAdapter, TitlebarButtons, WindowFrame,
    center_component_on_active_monitor, apply_mica_effect,
};
use std::sync::Arc;

/// 创建主窗口的无边框框架（不调度 apply()，由 CBT Hook 在 HCBT_ACTIVATE 时同步安装）
/// Create the main window's borderless frame (does NOT schedule apply();
/// apply_to_hwnd is called synchronously from the CBT Hook during HCBT_ACTIVATE)
pub fn create_frame(app: &AppWindow) -> WindowFrame<AppWindow> {
    let buttons = TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    };
    WindowFrame::new(app, Arc::new(GlobalWindowControlsAdapter::new(buttons)))
}

/// 配置主窗口（居中定位、Mica 特效、控件绑定）
/// Configure the main window (center, Mica, control bindings)
pub fn setup(app: &AppWindow, frame: &WindowFrame<AppWindow>, hook_ok: bool) {
    // 1. 定位到当前活动屏幕居中
    center_component_on_active_monitor(app, app.get_init_width(), app.get_init_height());

    // 2. 应用 Mica 视觉特效
    apply_mica_effect(app, |a| a.set_is_mica_active(true), hook_ok);

    // 3. 绑定窗口控制回调
    let buttons = TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    };
    controls::setup_window_controls(app, frame.clone(), buttons);
}