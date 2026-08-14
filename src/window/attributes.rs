//! 窗口属性与样式配置模块：声明式配置 DWM 视觉属性（Mica / Acrylic / 暗色模式 / 圆角）
//! 与 Win32 窗口样式（工具窗口、置顶、穿透、扩展样式）及逃生通道。
//!
//! Window attributes and styles module: declaratively configure DWM visual attributes
//! (Mica / Acrylic / dark mode / rounded corners) and Win32 window styles
//! (tool window, always on top, click-through, extended styles) with escape hatches.

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
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    };

    /// 检测当前系统是否处于深色模式
    ///
    /// 通过读取注册表键值判断：
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme`
    /// - 值为 0 → 深色模式 (true)
    /// - 值为 1 → 浅色模式 (false)
    /// - 读取失败 → 默认返回 `true`（深色模式）
    pub fn is_system_dark_mode() -> bool {
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
            println!("[WindowAttributes] 无法打开注册表键，默认使用深色模式 (error: {})", status);
            return true;
        }

        let mut data: u32 = 1;
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
            println!("[WindowAttributes] 无法读取 AppsUseLightTheme，默认使用深色模式 (error: {})", query_status);
            return true;
        }

        let is_dark = data == 0;
        println!(
            "[WindowAttributes] 系统主题检测: AppsUseLightTheme={}, is_dark={}",
            data, is_dark
        );
        is_dark
    }

    // ────────────────────────────────────────────────────────────────────────
    // DWM 常量定义
    // ────────────────────────────────────────────────────────────────────────
    const DWMSBT_AUTO: u32 = 0;
    const DWMSBT_NONE: u32 = 1;
    const DWMSBT_MAINWINDOW: u32 = 2; // Mica
    const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic
    const DWMSBT_TABBEDWINDOW: u32 = 4; // Tabbed

    /// 未文档化的 DWMWA_MICA_EFFECT 属性（适用于旧版 Win11 22000-22522）
    const DWMWA_MICA_EFFECT: u32 = 1029;
    /// DWMWA_BORDER_COLOR (Win11 build 22000+)
    const DWMWA_BORDER_COLOR: u32 = 34;
    /// DWMWA_CAPTION_COLOR (Win11 build 22000+)
    const DWMWA_CAPTION_COLOR: u32 = 35;
    /// DWMWA_TEXT_COLOR (Win11 build 22000+)
    const DWMWA_TEXT_COLOR: u32 = 36;

    /// DWM 背景材质类型
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DwmBackdrop {
        /// Mica 材质（Windows 11 推荐主窗口材质）
        Mica,
        /// Acrylic 亚克力材质（半透明模糊，适合轻量窗口/弹出层）
        Acrylic,
        /// Tabbed 标签页材质
        Tabbed,
        /// 无 DWM 材质
        None,
    }

    /// 圆角偏好
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CornerPreference {
        /// 圆角 / Rounded
        Round,
        /// 小圆角 / Small rounded
        RoundSmall,
        /// 不圆角 / Not rounded
        DoNotRound,
    }

    /// 原始 DWM 属性数据包装
    #[derive(Clone)]
    struct RawDwmAttr {
        attr: u32,
        data: Vec<u8>,
    }

    /// 声明式窗口属性配置器
    ///
    /// 涵盖：
    /// 1. DWM 视觉材质（Mica / Acrylic / Tabbed）、暗色模式、圆角偏好、边框颜色；
    /// 2. Win32 窗口样式（工具窗口 WS_EX_TOOLWINDOW、置顶 WS_EX_TOPMOST、穿透等）；
    /// 3. 多级逃生通道（自定义样式位、自定义 DWM 属性、裸 HWND 回调闭包）。
    #[derive(Default)]
    pub struct WindowAttributes {
        backdrop: Option<DwmBackdrop>,
        dark_mode: Option<bool>,
        corner: Option<CornerPreference>,
        border_color: Option<u32>,
        caption_color: Option<u32>,
        text_color: Option<u32>,

        // Win32 Styles & ExStyles
        style_add: u32,
        style_remove: u32,
        ex_style_add: u32,
        ex_style_remove: u32,

        // 逃生通道
        raw_dwm_attrs: Vec<RawDwmAttr>,
        custom_actions: Vec<Box<dyn FnOnce(HWND) + 'static>>,
    }

    impl WindowAttributes {
        /// 创建空属性配置
        pub fn new() -> Self {
            Self::default()
        }

        /// 启用 Mica 材质
        pub fn with_mica(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Mica);
            self
        }

        /// 启用 Acrylic 亚克力材质
        pub fn with_acrylic(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Acrylic);
            self
        }

        /// 启用 Tabbed 材质
        pub fn with_tabbed(mut self) -> Self {
            self.backdrop = Some(DwmBackdrop::Tabbed);
            self
        }

        /// 设置 DWM 背景材质类型
        pub fn with_backdrop(mut self, backdrop: DwmBackdrop) -> Self {
            self.backdrop = Some(backdrop);
            self
        }

        /// 设置深浅色模式（true: 深色, false: 浅色）
        pub fn with_dark_mode(mut self, enabled: bool) -> Self {
            self.dark_mode = Some(enabled);
            self
        }

        /// 设置圆角偏好（Round / RoundSmall / DoNotRound）
        pub fn with_corner(mut self, pref: CornerPreference) -> Self {
            self.corner = Some(pref);
            self
        }

        /// 设置 DWM 边框颜色（0x00BBGGRR 格式，Win11 适用）
        pub fn with_border_color(mut self, color_bgr: u32) -> Self {
            self.border_color = Some(color_bgr);
            self
        }

        /// 设置 DWM 标题栏颜色（0x00BBGGRR 格式，Win11 适用）
        pub fn with_caption_color(mut self, color_bgr: u32) -> Self {
            self.caption_color = Some(color_bgr);
            self
        }

        /// 设置 DWM 标题文本颜色（0x00BBGGRR 格式，Win11 适用）
        pub fn with_text_color(mut self, color_bgr: u32) -> Self {
            self.text_color = Some(color_bgr);
            self
        }

        // ────────────────────────────────────────────────────────────────────
        // Win32 窗口样式配置（含冷门属性）
        // ────────────────────────────────────────────────────────────────────

        /// 设置为工具窗口（WS_EX_TOOLWINDOW）
        ///
        /// 效果：不在 Alt+Tab 列表及任务栏显示，适合悬浮工具栏、辅助调试面板等。
        pub fn with_tool_window(mut self, enabled: bool) -> Self {
            if enabled {
                self.ex_style_add |= WS_EX_TOOLWINDOW;
                self.ex_style_remove |= WS_EX_APPWINDOW;
            } else {
                self.ex_style_remove |= WS_EX_TOOLWINDOW;
            }
            self
        }

        /// 强制在任务栏显示（WS_EX_APPWINDOW）
        pub fn with_app_window(mut self, enabled: bool) -> Self {
            if enabled {
                self.ex_style_add |= WS_EX_APPWINDOW;
                self.ex_style_remove |= WS_EX_TOOLWINDOW;
            } else {
                self.ex_style_remove |= WS_EX_APPWINDOW;
            }
            self
        }

        /// 设置窗口置顶（WS_EX_TOPMOST）
        pub fn with_always_on_top(mut self, enabled: bool) -> Self {
            if enabled {
                self.ex_style_add |= WS_EX_TOPMOST;
            } else {
                self.ex_style_remove |= WS_EX_TOPMOST;
            }
            self
        }

        /// 设置鼠标点击穿透（WS_EX_TRANSPARENT）
        pub fn with_click_through(mut self, enabled: bool) -> Self {
            if enabled {
                self.ex_style_add |= WS_EX_TRANSPARENT;
            } else {
                self.ex_style_remove |= WS_EX_TRANSPARENT;
            }
            self
        }

        /// 设置点击不激活抢焦点（WS_EX_NOACTIVATE）
        pub fn with_no_activate(mut self, enabled: bool) -> Self {
            if enabled {
                self.ex_style_add |= WS_EX_NOACTIVATE;
            } else {
                self.ex_style_remove |= WS_EX_NOACTIVATE;
            }
            self
        }

        /// 修改原始 Win32 GWL_STYLE 标志位
        pub fn with_raw_style(mut self, add: u32, remove: u32) -> Self {
            self.style_add |= add;
            self.style_remove |= remove;
            self
        }

        /// 修改原始 Win32 GWL_EXSTYLE 标志位
        pub fn with_raw_ex_style(mut self, add: u32, remove: u32) -> Self {
            self.ex_style_add |= add;
            self.ex_style_remove |= remove;
            self
        }

        // ────────────────────────────────────────────────────────────────────
        // 逃生通道（Escape Hatches）
        // ────────────────────────────────────────────────────────────────────

        /// 逃生通道 1：直接向 HWND 注入任意 DWM 属性（DwmSetWindowAttribute）
        pub fn with_raw_dwm_attribute<T: Copy>(mut self, attr: u32, val: T) -> Self {
            let size = size_of::<T>();
            let mut data = Vec::with_capacity(size);
            unsafe {
                let ptr = &val as *const T as *const u8;
                data.extend_from_slice(std::slice::from_raw_parts(ptr, size));
            }
            self.raw_dwm_attrs.push(RawDwmAttr { attr, data });
            self
        }

        /// 逃生通道 2：在 apply(hwnd) 时执行任意自定义 Win32 / DWM 调用闭包
        pub fn with_custom_action(mut self, action: impl FnOnce(HWND) + 'static) -> Self {
            self.custom_actions.push(Box::new(action));
            self
        }

        /// 将配置好的属性与样式应用到目标 HWND
        pub fn apply(self, hwnd: HWND) {
            // 1. 应用 Win32 GWL_STYLE
            if self.style_add != 0 || self.style_remove != 0 {
                let current = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
                let new_style = (current | self.style_add) & !self.style_remove;
                if new_style != current {
                    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize) };
                }
            }

            // 2. 应用 Win32 GWL_EXSTYLE
            if self.ex_style_add != 0 || self.ex_style_remove != 0 {
                let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 };
                let new_ex = (current | self.ex_style_add) & !self.ex_style_remove;
                if new_ex != current {
                    unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize) };
                    // 刷新窗口框架以使样式生效
                    unsafe {
                        SetWindowPos(
                            hwnd,
                            0 as _,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                        );
                    }
                }
            }

            // 3. DWM Backdrop (Mica / Acrylic / Tabbed)
            if let Some(backdrop) = self.backdrop {
                let backdrop_type: u32 = match backdrop {
                    DwmBackdrop::Mica => DWMSBT_MAINWINDOW,
                    DwmBackdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
                    DwmBackdrop::Tabbed => DWMSBT_TABBEDWINDOW,
                    DwmBackdrop::None => DWMSBT_NONE,
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
                println!("[WindowAttributes] 注入 Backdrop({:?}): HWND({:?})", backdrop, hwnd);
            }

            // 4. 暗色模式 / Dark mode
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
                println!("[WindowAttributes] 注入 DarkMode({}): HWND({:?})", dark, hwnd);
            }

            // 5. 圆角偏好 / Corner preference
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
                println!("[WindowAttributes] 注入 Corner({:?}): HWND({:?})", corner, hwnd);
            }

            // 6. 边框/标题/文字颜色
            if let Some(color) = self.border_color {
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_BORDER_COLOR,
                        &color as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
            }
            if let Some(color) = self.caption_color {
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_CAPTION_COLOR,
                        &color as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
            }
            if let Some(color) = self.text_color {
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_TEXT_COLOR,
                        &color as *const _ as *const _,
                        size_of::<u32>() as u32,
                    )
                };
            }

            // 7. 自定义 DWM 属性注入 (逃生通道 1)
            for raw in self.raw_dwm_attrs {
                let _ = unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        raw.attr,
                        raw.data.as_ptr() as *const _,
                        raw.data.len() as u32,
                    )
                };
            }

            // 8. 自定义闭包执行 (逃生通道 2)
            for action in self.custom_actions {
                action(hwnd);
            }
        }
    }

    /// 向后兼容的别名
    pub type DwmPreset = WindowAttributes;
}

#[cfg(target_os = "windows")]
pub use inner::*;

#[cfg(not(target_os = "windows"))]
pub mod fallback {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DwmBackdrop {
        Mica,
        Acrylic,
        Tabbed,
        None,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CornerPreference {
        Round,
        RoundSmall,
        DoNotRound,
    }

    #[derive(Default)]
    pub struct WindowAttributes;

    pub fn is_system_dark_mode() -> bool {
        false
    }

    impl WindowAttributes {
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
        pub fn with_backdrop(self, _backdrop: DwmBackdrop) -> Self {
            self
        }
        pub fn with_dark_mode(self, _enabled: bool) -> Self {
            self
        }
        pub fn with_corner(self, _pref: CornerPreference) -> Self {
            self
        }
        pub fn with_border_color(self, _color_bgr: u32) -> Self {
            self
        }
        pub fn with_caption_color(self, _color_bgr: u32) -> Self {
            self
        }
        pub fn with_text_color(self, _color_bgr: u32) -> Self {
            self
        }
        pub fn with_tool_window(self, _enabled: bool) -> Self {
            self
        }
        pub fn with_app_window(self, _enabled: bool) -> Self {
            self
        }
        pub fn with_always_on_top(self, _enabled: bool) -> Self {
            self
        }
        pub fn with_click_through(self, _enabled: bool) -> Self {
            self
        }
        pub fn with_no_activate(self, _enabled: bool) -> Self {
            self
        }
        pub fn with_raw_style(self, _add: u32, _remove: u32) -> Self {
            self
        }
        pub fn with_raw_ex_style(self, _add: u32, _remove: u32) -> Self {
            self
        }
        pub fn with_raw_dwm_attribute<T: Copy>(self, _attr: u32, _val: T) -> Self {
            self
        }
        pub fn with_custom_action(self, _action: impl FnOnce(()) + 'static) -> Self {
            self
        }
        pub fn apply(self, _hwnd: ()) {}
    }

    pub type DwmPreset = WindowAttributes;
}

#[cfg(not(target_os = "windows"))]
pub use fallback::*;
