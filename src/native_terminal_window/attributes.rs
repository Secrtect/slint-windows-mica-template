//! native_terminal_window 专用窗口属性配置（DIY 空间）
//!
//! 在此自由调整原生终端窗口的 DWM 材质、样式与逃生通道。

use crate::window::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取原生终端窗口的个性化属性配置
pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica()
        .with_dark_mode(is_system_dark_mode())
        .with_corner(CornerPreference::Round)
}
