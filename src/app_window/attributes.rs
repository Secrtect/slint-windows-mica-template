//! DWM 属性预设模块：声明式配置 DWM 视觉属性（Mica / 暗色模式 / 圆角）
//!
//! DWM attribute preset module: declaratively configure DWM visual attributes
//! (Mica / dark mode / rounded corners).
//!
//! 从 `cbt_hook` 中抽离，使 DWM 属性管理与 Hook 机制解耦。
//! Extracted from `cbt_hook` to decouple DWM attribute management from the hook mechanism.

#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod inner {
    use std::mem::size_of;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DWMWCP_ROUND, DWMWCP_ROUNDSMALL,
    };
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_CURRENT_USER, KEY_READ, REG_DWORD,
    };

    /// 检测当前系统是否处于深色模式
    ///
    /// 通过读取注册表键值判断：
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme`
    /// - 值为 0 → 深色模式
    /// - 值为 1 → 浅色模式
    /// - 读取失败 → 默认返回 `true`（深色模式）
    ///
    /// Detect whether the system is currently in dark mode.
    ///
    /// Reads the registry key:
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme`
    /// - Value 0 → dark mode
    /// - Value 1 → light mode
    /// - On failure → defaults to `true` (dark mode)
    pub fn is_system_dark_mode() -> bool {
        // 子键路径（UTF-16 + null 终止符）
        // Subkey path (UTF-16 + null terminator)
        let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize\0"
            .encode_utf16()
            .collect();
        let value_name: Vec<u16> = "AppsUseLightTheme\0".encode_utf16().collect();

        let mut hkey: windows_sys::Win32::System::Registry::HKEY = 0;
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            )
        };

        if status != 0 {
            println!("[DwmPreset] 无法打开注册表键，默认使用深色模式 (error: {})", status);
            // Failed to open registry key, defaulting to dark mode
            return true;
        }

        let mut data: u32 = 1; // 默认浅色 / default light
        let mut data_size: u32 = size_of::<u32>() as u32;
        let mut data_type: u32 = 0;
        let query_status = unsafe {
            RegQueryValueExW(
                hkey,
                value_name.as_ptr(),
                std::ptr::null(),
                &mut data_type,
                &mut data as *mut u32 as *mut u8,
                &mut data_size,
            )
        };

        unsafe { RegCloseKey(hkey) };

        if query_status != 0 || data_type != REG_DWORD {
            println!("[DwmPreset] 无法读取 AppsUseLightTheme，默认使用深色模式 (error: {})", query_status);
            // Failed to read AppsUseLightTheme, defaulting to dark mode
            return true;
        }

        let is_dark = data == 0;
        println!(
            "[DwmPreset] 系统主题检测: AppsUseLightTheme={}, is_dark={}",
            data, is_dark
        );
        // System theme detected
        is_dark
    }

    // ────────────────────────────────────────────────────────────────────────
    // DWM Backdrop 类型常量（windows-sys 0.52 未导出这些值，手动定义）
    // DWM Backdrop type constants (not exported by windows-sys 0.52)
    // ────────────────────────────────────────────────────────────────────────
    const DWMSBT_MAINWINDOW: u32 = 2; // Mica
    const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic
    const DWMSBT_TABBEDWINDOW: u32 = 4; // Tabbed

    /// 未文档化的 DWMWA_MICA_EFFECT 属性（适用于旧版 Win11 22000-22522）
    /// Undocumented DWMWA_MICA_EFFECT attribute (for older Win11 builds 22000-22522)
    const DWMWA_MICA_EFFECT: u32 = 1029;

    // ────────────────────────────────────────────────────────────────────────
    // DwmPreset：声明式 DWM 属性预设
    // DwmPreset: declarative DWM attribute preset
    // ────────────────────────────────────────────────────────────────────────

    /// DWM 背景材质类型
    /// DWM backdrop material type
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DwmBackdrop {
        /// Mica 效果（Windows 11 推荐主窗口材质）
        /// Mica effect (recommended for main windows on Windows 11)
        Mica,
        /// Acrylic 效果
        /// Acrylic effect
        Acrylic,
        /// Tabbed 效果（用于标签页窗口）
        /// Tabbed effect (for tabbed windows)
        Tabbed,
    }

    /// 圆角偏好
    /// Corner preference
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CornerPreference {
        /// 圆角 / Rounded
        Round,
        /// 小圆角 / Small rounded
        RoundSmall,
        /// 不圆角 / Not rounded
        DoNotRound,
    }

    /// 声明式 DWM 属性预设，通过 Builder 模式配置
    /// Declarative DWM attribute preset, configured via Builder pattern
    #[derive(Debug, Clone, Default)]
    pub struct DwmPreset {
        backdrop: Option<DwmBackdrop>,
        dark_mode: Option<bool>,
        corner: Option<CornerPreference>,
    }

    impl DwmPreset {
        /// 创建空预设
        /// Create an empty preset
        pub fn new() -> Self {
            Self::default()
        }

        /// 启用 Mica 背景
        /// Enable Mica backdrop
        pub fn with_mica(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Mica);
            self
        }

        /// 启用 Acrylic 背景
        /// Enable Acrylic backdrop
        pub fn with_acrylic(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Acrylic);
            self
        }

        /// 启用 Tabbed 背景
        /// Enable Tabbed backdrop
        pub fn with_tabbed(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Tabbed);
            self
        }

        /// 设置暗色模式
        /// Set dark mode
        pub fn with_dark_mode(mut self, enabled: bool) -> Self {
            self.dark_mode = Some(enabled);
            self
        }

        /// 设置圆角偏好
        /// Set corner preference
        pub fn with_corner(mut self, pref: CornerPreference) -> Self {
            self.corner = Some(pref);
            self
        }

        /// 将预设的 DWM 属性注入到指定 HWND
        /// Apply the preset DWM attributes to the given HWND
        pub fn apply(&self, hwnd: HWND) {
            // 1. Backdrop (Mica / Acrylic / Tabbed)
            if let Some(backdrop) = self.backdrop {
                let backdrop_type: u32 = match backdrop {
                    DwmBackdrop::Mica => DWMSBT_MAINWINDOW,
                    DwmBackdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
                    DwmBackdrop::Tabbed => DWMSBT_TABBEDWINDOW,
                };
                let result = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_SYSTEMBACKDROP_TYPE as u32,
                        &backdrop_type as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
                if result != 0 && matches!(backdrop, DwmBackdrop::Mica) {
                    // 在旧版 Win11（22000-22522）上回退到未文档化的 MICA_EFFECT
                    // Fall back to undocumented MICA_EFFECT on older Win11 builds
                    let enabled: u32 = 1;
                    let _ = unsafe {
                        DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_MICA_EFFECT,
                            &enabled as *const _ as *const _,
                            size_of::<u32>() as u32,
                        )
                    };
                }
                println!("[DwmPreset] 注入 Backdrop({:?}): HWND({:?})", backdrop, hwnd);
            }

            // 2. 暗色模式 / Dark mode
            if let Some(dark) = self.dark_mode {
                let value: u32 = if dark { 1 } else { 0 };
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
                        &value as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
                println!("[DwmPreset] 注入 DarkMode({}): HWND({:?})", dark, hwnd);
            }

            // 3. 圆角偏好 / Corner preference
            if let Some(corner) = self.corner {
                let pref: u32 = match corner {
                    CornerPreference::Round => DWMWCP_ROUND as u32,
                    CornerPreference::RoundSmall => DWMWCP_ROUNDSMALL as u32,
                    CornerPreference::DoNotRound => DWMWCP_DONOTROUND as u32,
                };
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                        &pref as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
                println!("[DwmPreset] 注入 Corner({:?}): HWND({:?})", corner, hwnd);
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────────
// 跨平台导出（非 Windows 平台提供空实现）
// Cross-platform exports (no-op on non-Windows platforms)
// ────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
pub use inner::*;

/// 非 Windows 平台的空实现
/// No-op implementation for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub mod fallback {
    #[derive(Debug, Clone, Default)]
    pub struct DwmPreset;

    pub fn is_system_dark_mode() -> bool {
        false
    }

    impl DwmPreset {
        pub fn new() -> Self {
            Self
        }
        pub fn with_mica(self) -> Self {
            self
        }
        pub fn with_acrylic(self) -> Self {
            self
        }
        pub fn with_tabbed(self) -> Self {
            self
        }
        pub fn with_dark_mode(self, _enabled: bool) -> Self {
            self
        }
        pub fn apply(&self, _hwnd: ()) {}
    }
}

#[cfg(not(target_os = "windows"))]
pub use fallback::*;
