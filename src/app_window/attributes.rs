//! app_window 专用窗口属性配置（DIY 空间）
//!
//! 在此自由定义主窗口的 DWM 材质、暗色模式、圆角、任务栏行为与逃生通道。

use crate::window::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取主窗口的个性化属性配置
pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica() // 主窗口使用 Windows 11 Mica 材质
        .with_dark_mode(is_system_dark_mode())
        .with_corner(CornerPreference::Round) // 标准大圆角
        .with_app_window(true) // 确保在任务栏显示图标
}
