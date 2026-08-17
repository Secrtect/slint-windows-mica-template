//! 主窗口专用控件绑定配置（DIY 空间）
//! Main window exclusive control bindings configuration (DIY area).
//!
//! 在此自由调整主窗口标题栏按钮可见性及特定事件。
//! Freely adjust main window titlebar button visibility and specific interaction events.

use crate::AppWindow;
use crate::platform::borderless::WindowFrame;
use crate::platform::controls::{TitlebarButtons, setup_global_window_controls};

/// 绑定主窗口的标题栏与无边框控制
/// Bind titlebar and borderless controls for the main window.
pub fn setup_window_controls(app: &AppWindow, frame: WindowFrame<AppWindow>, buttons: TitlebarButtons) {
    setup_global_window_controls(app, frame, buttons);
}
