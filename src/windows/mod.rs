//! 业务与示例窗口模块集合
//! Application and sample window modules collection.
//!
//! 包含主窗口、自绘终端窗口与 DWM 原生终端窗口的具体业务实现。
//! Contains specific implementations for main window, custom terminal window, and DWM native terminal window.

pub mod app;
pub mod custom_terminal;
pub mod native_terminal;

/// 窗口关闭行为 - Window close behavior
///
/// 控制二级窗口（终端窗口）在被关闭时的行为：
/// - `Hide`: 隐藏窗口，保留组件实例，可稍后重新显示（适合需要频繁切换显示/隐藏的场景）
/// - `Destroy`: 销毁窗口，释放组件及所有资源（适合用完即弃的场景，默认行为）
///
/// Controls the behavior of secondary windows (terminal windows) when closed:
/// - `Hide`: Hide the window, keep the component instance alive for later re-display
///   (suitable for frequent show/hide toggling)
/// - `Destroy`: Destroy the window, release the component and all resources
///   (suitable for one-shot scenarios, default behavior)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Hide 为其他开发者预留的选项，当前默认使用 Destroy / Hide is reserved for developers; Destroy is used by default
pub enum CloseBehavior {
    /// 关闭时隐藏窗口（可重新显示）- Hide window on close (can be re-shown later)
    Hide,
    /// 关闭时销毁窗口（释放资源）- Destroy window on close (release resources)
    Destroy,
}
