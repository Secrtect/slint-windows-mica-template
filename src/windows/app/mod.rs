//! app 模块：主窗口专属生命周期与业务逻辑
//! App module: main window exclusive lifecycle and business logic.
//!
//! 与 ui/app-window/ 一一对应，底层无边框与属性能力复用 crate::platform 基础设施。
//! Corresponds directly to ui/app-window/; reuses crate::platform infrastructure for borderless and DWM styling.

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
/// Configure the main window (center positioning, Mica effect, control bindings)
pub fn setup(app: &AppWindow, frame: &WindowFrame<AppWindow>, hook_ok: bool) {
    // 1. 定位到当前活动屏幕居中 / 1. Center window on active monitor
    center_component_on_active_monitor(app, app.get_init_width(), app.get_init_height());

    // 2. 应用 Mica 视觉特效 / 2. Apply Mica visual effect
    apply_mica_effect(app, |a| a.set_is_mica_active(true), hook_ok);

    // 3. 绑定窗口控制回调 / 3. Bind window control callbacks
    let buttons = TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    };
    controls::setup_window_controls(app, frame.clone(), buttons);
}