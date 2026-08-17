//! 通用 Windows 平台窗口基础设施模块
//! Common Windows Platform Window Infrastructure Module.
//!
//! 提供跨窗口复用的 Win32 无边框框架、Snap Layouts 贴靠布局、DWM 属性 (Mica/Acrylic)、
//! 窗口样式与逃生通道、CBT Hook 零闪烁捕获、屏幕居中及视觉特效。
//!
//! Provides reusable Win32 borderless frame, Win11 Snap Layouts, DWM attributes (Mica/Acrylic),
//! window styles and escape hatches, CBT Hook zero-flicker capture, monitor centering, and visual effects.

pub mod attributes;
pub mod borderless;
pub mod controls;
pub mod display;
pub mod effects;
pub mod hook;

pub use attributes::{CornerPreference, DwmBackdrop, DwmPreset, WindowAttributes, is_system_dark_mode};
pub use borderless::{SimpleBorderlessAdapter, TitlebarAdapter, TitlebarMetrics, TitlebarSetup, WindowFrame};
pub use controls::{GlobalWindowControlsAdapter, TitlebarButtons, setup_global_window_controls};
pub use display::{center_component_on_active_monitor, center_on_active_monitor};
pub use effects::apply_mica_effect;
pub use hook::CbtHookGuard;
