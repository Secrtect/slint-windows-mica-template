//! app_window 专用控件绑定配置（DIY 空间）
//!
//! 在此自由调整主窗口标题栏按钮可见性及特定事件。

use crate::AppWindow;
use crate::window::borderless::WindowFrame;
use crate::window::controls::{TitlebarButtons, setup_global_window_controls};

/// 绑定主窗口的标题栏与无边框控制
pub fn setup_window_controls(app: &AppWindow, frame: WindowFrame<AppWindow>, buttons: TitlebarButtons) {
    setup_global_window_controls(app, frame, buttons);
}
