//! 标题栏控件配置与通用适配器模块
//!
//! Titlebar controls configuration and common adapters module.

use super::borderless::{TitlebarAdapter, TitlebarMetrics, WindowFrame};
use crate::AppWindow;
use slint::ComponentHandle;

/// 标题栏按钮可见性配置
#[derive(Debug, Clone, Copy)]
pub struct TitlebarButtons {
    pub show_minimize: bool,
    pub show_maximize: bool,
    pub show_close: bool,
}

impl Default for TitlebarButtons {
    fn default() -> Self {
        Self {
            show_minimize: true,
            show_maximize: true,
            show_close: true,
        }
    }
}

/// 针对使用 Slint `global WindowControls` 全局单例的窗口（如主窗口 AppWindow）的通用适配器
pub struct GlobalWindowControlsAdapter {
    pub buttons: TitlebarButtons,
}

impl GlobalWindowControlsAdapter {
    pub fn new(buttons: TitlebarButtons) -> Self {
        Self { buttons }
    }
}

impl TitlebarAdapter<AppWindow> for GlobalWindowControlsAdapter {
    fn metrics(&self, component: &AppWindow) -> TitlebarMetrics {
        let controls = component.global::<crate::WindowControls>();
        TitlebarMetrics {
            titlebar_height: controls.get_titlebar_height(),
            close_width: controls.get_close_width(),
            maximize_width: controls.get_maximize_width(),
            minimize_width: controls.get_minimize_width(),
            show_minimize: self.buttons.show_minimize && controls.get_show_minimize(),
            show_maximize: self.buttons.show_maximize && controls.get_show_maximize(),
            show_close: self.buttons.show_close && controls.get_show_close(),
        }
    }

    fn set_active(&self, component: &AppWindow, is_active: bool) {
        let controls = component.global::<crate::WindowControls>();
        controls.set_is_active(is_active);
    }

    fn set_hover(&self, component: &AppWindow, min: bool, max: bool, close: bool) {
        let controls = component.global::<crate::WindowControls>();
        if controls.get_min_hover() != min { controls.set_min_hover(min); }
        if controls.get_max_hover() != max { controls.set_max_hover(max); }
        if controls.get_close_hover() != close { controls.set_close_hover(close); }
    }

    fn set_pressed(&self, component: &AppWindow, min: bool, max: bool, close: bool) {
        let controls = component.global::<crate::WindowControls>();
        if controls.get_min_pressed() != min { controls.set_min_pressed(min); }
        if controls.get_max_pressed() != max { controls.set_max_pressed(max); }
        if controls.get_close_pressed() != close { controls.set_close_pressed(close); }
    }

    fn on_minimize_clicked(&self, component: &AppWindow, _frame: &WindowFrame<AppWindow>) {
        let controls = component.global::<crate::WindowControls>();
        controls.invoke_minimize();
    }

    fn on_maximize_clicked(&self, component: &AppWindow, _frame: &WindowFrame<AppWindow>) {
        let controls = component.global::<crate::WindowControls>();
        controls.invoke_maximize();
    }

    fn on_close_clicked(&self, component: &AppWindow, _frame: &WindowFrame<AppWindow>) {
        let controls = component.global::<crate::WindowControls>();
        controls.invoke_close();
    }
}

/// 快速绑定主窗口 global WindowControls 的回调与同步
pub fn setup_global_window_controls(
    app: &AppWindow,
    frame: WindowFrame<AppWindow>,
    buttons: TitlebarButtons,
) {
    let controls = app.global::<crate::WindowControls>();

    // 1. 初始化按钮可见性与状态
    controls.set_show_minimize(buttons.show_minimize);
    controls.set_show_maximize(buttons.show_maximize);
    controls.set_show_close(buttons.show_close);
    controls.set_maximized(false);

    // 2. 监听最大化/还原状态变化
    frame.on_maximized_changed(|app, is_max| {
        app.global::<crate::WindowControls>().set_maximized(is_max);
    });

    // 3. 绑定 UI 回调到 WindowFrame
    let f_max = frame.clone();
    controls.on_maximize(move || {
        f_max.toggle_maximized();
    });

    let f_dbl = frame.clone();
    controls.on_double_click(move || {
        f_dbl.toggle_maximized();
    });

    let f_close = frame.clone();
    controls.on_close(move || {
        f_close.close();
    });

    let f_drag = frame.clone();
    controls.on_drag(move || {
        f_drag.drag();
    });

    let f_min = frame.clone();
    controls.on_minimize(move || {
        f_min.minimize();
    });
}
