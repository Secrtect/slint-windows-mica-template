//! app_window 模块：主窗口专属生命周期与业务逻辑
//!
//! 与 ui/app-window/ 一一对应，底层无边框与属性能力复用 crate::window 基础设施。

pub mod attributes;
pub mod controls;

use crate::AppWindow;
use crate::window::{
    GlobalWindowControlsAdapter, TitlebarButtons, TitlebarSetup, WindowFrame,
    center_component_on_active_monitor, apply_mica_effect,
};
use slint::ComponentHandle;

/// 初始化并配置主窗口
pub fn setup(app: &AppWindow, hook_ok: bool) -> Result<WindowFrame<AppWindow>, slint::PlatformError> {
    // 1. 启动无边框机制（挂载基于 WindowControls 全局单例的标题栏适配器）
    let buttons = TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    };
    let frame = app
        .as_weak()
        .setup_borderless_with_adapter(GlobalWindowControlsAdapter::new(buttons))?;

    // 2. 定位到当前活动屏幕居中
    center_component_on_active_monitor(app, app.get_init_width(), app.get_init_height());

    // 3. 应用 Mica 视觉特效
    apply_mica_effect(app, |a| a.set_is_mica_active(true), hook_ok);

    // 4. 绑定窗口控制回调
    controls::setup_window_controls(app, frame.clone(), buttons);

    Ok(frame)
}
