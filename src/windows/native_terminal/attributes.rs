//! 原生终端窗口专用属性配置（DIY 空间）
//! Native terminal window exclusive attributes configuration (DIY area).
//!
//! 在此自由调整原生终端窗口的 DWM 材质、样式与逃生通道。
//! 窗口使用 no-frame: false（WS_OVERLAPPEDWINDOW），DWM 原生绘制标题栏按钮，
//! 通过 WM_NCCALCSIZE 子类化隐藏标准标题栏，仅保留 DWM 原生按钮。
//!
//! Freely adjust native terminal window DWM material, styles, and escape hatches.
//! Window uses no-frame: false (WS_OVERLAPPEDWINDOW), DWM draws caption buttons natively,
//! and standard titlebar is hidden via WM_NCCALCSIZE subclassing to preserve only native caption buttons.

use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取原生终端窗口的个性化属性配置
/// Retrieve personalized window attributes for the native terminal window.
pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica()
        .with_dark_mode(is_system_dark_mode())
        .with_corner(CornerPreference::Round)
        // no-frame: false 已提供 WS_OVERLAPPEDWINDOW（含 CAPTION/SYSMENU/MINIMIZEBOX/MAXIMIZEBOX），
        // DWM 在窗口创建时即分配标题栏按钮，无需额外添加样式
        // no-frame: false already provides WS_OVERLAPPEDWINDOW (includes CAPTION/SYSMENU/MINIMIZEBOX/MAXIMIZEBOX),
        // DWM allocates caption buttons at window creation, no extra styles needed
}
