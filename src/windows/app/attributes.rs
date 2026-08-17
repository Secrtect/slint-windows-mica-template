//! 主窗口专用属性配置（DIY 空间）
//! Main window exclusive attributes configuration (DIY area).
//!
//! 在此自由定义主窗口的 DWM 材质、暗色模式、圆角、任务栏行为与逃生通道。
//! Freely configure main window DWM material, dark mode, corner preference, taskbar presence, and escape hatches.

use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取主窗口的个性化属性配置
/// Retrieve personalized window attributes for the main window.
pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica() // 主窗口使用 Windows 11 Mica 材质 / Main window uses Windows 11 Mica material
        .with_dark_mode(is_system_dark_mode()) // 跟随系统深浅色 / Follow system dark/light theme
        .with_corner(CornerPreference::Round) // 标准大圆角 / Standard large rounded corners
        .with_app_window(true) // 确保在任务栏显示图标 / Ensure icon appears in taskbar
}
