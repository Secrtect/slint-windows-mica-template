//! 自绘终端窗口专用属性配置（DIY 空间）
//!
//! 在此自由调整自绘终端窗口的 DWM 材质（如 Mica/Acrylic）、工具窗样式、置顶属性等。

use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取自绘终端窗口的个性化属性配置
pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica() // 亦可换为 .with_acrylic()
        .with_dark_mode(is_system_dark_mode())
        .with_corner(CornerPreference::Round) // 圆角偏好
        // 示例：如需使终端变为轻量工具悬浮窗，只需取消下面这行注释：
        // .with_tool_window(true)
        // 示例：如需终端置顶显示：
        // .with_always_on_top(true)
}
