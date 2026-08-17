//! 自绘终端窗口专用属性配置（DIY 空间）
//! Custom terminal window exclusive attributes configuration (DIY area).
//!
//! 在此自由调整自绘终端窗口的 DWM 材质（如 Mica/Acrylic）、工具窗样式、置顶属性等。
//! Freely adjust custom terminal window DWM material (Mica/Acrylic), tool window style, always-on-top, etc.

use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取自绘终端窗口的个性化属性配置
/// Retrieve personalized window attributes for the custom terminal window.
pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica() // 亦可换为 .with_acrylic() / Can also be changed to .with_acrylic()
        .with_dark_mode(is_system_dark_mode()) // 跟随系统主题 / Follow system dark/light theme
        .with_corner(CornerPreference::Round) // 圆角偏好 / Corner preference
        // 示例：如需使终端变为轻量工具悬浮窗，只需取消下面这行注释：
        // Example: To make the terminal a lightweight tool window, uncomment the line below:
        // .with_tool_window(true)
        // 示例：如需终端置顶显示：
        // Example: To make the terminal always on top:
        // .with_always_on_top(true)
}
