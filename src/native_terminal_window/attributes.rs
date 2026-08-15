//! native_terminal_window 专用窗口属性配置（DIY 空间）
//!
//! 在此自由调整原生终端窗口的 DWM 材质、样式与逃生通道。
//! 窗口使用 no-frame: false（WS_OVERLAPPEDWINDOW），DWM 原生绘制标题栏按钮，
//! 通过 WM_NCCALCSIZE 子类化隐藏标准标题栏，仅保留 DWM 原生按钮。

use crate::window::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

/// 获取原生终端窗口的个性化属性配置
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
