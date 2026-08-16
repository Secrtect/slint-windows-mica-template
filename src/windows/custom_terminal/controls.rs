//! 自绘终端窗口专用标题栏适配器
//!
//! 提供与主窗口 AppWindow 一致的 Win32 非客户区按钮命中测试，
//! 从而获得 Windows 原生系统气泡提示（tooltip）和 Win11 Snap Layouts 贴靠布局。
//!
//! Custom titlebar adapter for the custom terminal window.
//! Provides the same Win32 non-client area hit-testing as the main window,
//! enabling native system tooltips and Win11 Snap Layouts.

use crate::CustomTerminalWindow;
use crate::platform::borderless::{TitlebarAdapter, TitlebarMetrics};
use crate::platform::controls::TitlebarButtons;

/// 终端窗口专用标题栏适配器
///
/// 与主窗口的 GlobalWindowControlsAdapter 不同，此适配器：
/// - 使用硬编码的按钮尺寸（与 Slint 中 TerminalTitlebar 的布局一致）
/// - 不依赖 global WindowControls 单例，而是直接设置 CustomTerminalWindow 的
///   titlebar-*-hover / titlebar-*-pressed 属性来驱动按钮视觉状态
/// - 点击事件使用 trait 默认实现委托给 WindowFrame（最小化/最大化/关闭）
pub struct TerminalTitlebarAdapter {
    pub buttons: TitlebarButtons,
}

impl TerminalTitlebarAdapter {
    pub fn new(buttons: TitlebarButtons) -> Self {
        Self { buttons }
    }
}

impl TitlebarAdapter<CustomTerminalWindow> for TerminalTitlebarAdapter {
    fn metrics(&self, _component: &CustomTerminalWindow) -> TitlebarMetrics {
        // 按钮尺寸与 ui/custom-terminal-window/titlebar.slint 中的
        // TerminalTitlebar 布局保持一致：
        //   height: 36px, 按钮宽度: 46px
        TitlebarMetrics {
            titlebar_height: 36.0,
            close_width: 46.0,
            maximize_width: 46.0,
            minimize_width: 46.0,
            show_minimize: self.buttons.show_minimize,
            show_maximize: self.buttons.show_maximize,
            show_close: self.buttons.show_close,
        }
    }

    /// 由 Win32 子类化 `WM_NCACTIVATE` 消息驱动，实现失去焦点时标题栏文字/按钮变淡
    /// Driven by Win32 subclassing WM_NCACTIVATE message,
    /// implements the fade effect when the window loses focus
    fn set_active(&self, component: &CustomTerminalWindow, is_active: bool) {
        if component.get_titlebar_is_active() != is_active {
            component.set_titlebar_is_active(is_active);
        }
    }

    /// 由 Win32 子类化 `WM_NCMOUSEHOVER` 等消息驱动，更新标题栏按钮的 hover 视觉状态
    /// Driven by Win32 subclassing messages (WM_NCMOUSEHOVER etc.),
    /// updates the titlebar button hover visual state
    fn set_hover(&self, component: &CustomTerminalWindow, min: bool, max: bool, close: bool) {
        if component.get_titlebar_min_hover() != min {
            component.set_titlebar_min_hover(min);
        }
        if component.get_titlebar_max_hover() != max {
            component.set_titlebar_max_hover(max);
        }
        if component.get_titlebar_close_hover() != close {
            component.set_titlebar_close_hover(close);
        }
    }

    /// 由 Win32 子类化 `WM_NCLBUTTONDOWN` 等消息驱动，更新标题栏按钮的 pressed 视觉状态
    /// Driven by Win32 subclassing messages (WM_NCLBUTTONDOWN etc.),
    /// updates the titlebar button pressed visual state
    fn set_pressed(&self, component: &CustomTerminalWindow, min: bool, max: bool, close: bool) {
        if component.get_titlebar_min_pressed() != min {
            component.set_titlebar_min_pressed(min);
        }
        if component.get_titlebar_max_pressed() != max {
            component.set_titlebar_max_pressed(max);
        }
        if component.get_titlebar_close_pressed() != close {
            component.set_titlebar_close_pressed(close);
        }
    }
}