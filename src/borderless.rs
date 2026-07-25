#![allow(dead_code)]

use crate::AppWindow;
use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use slint::Window;
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use tracing::warn;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWM_WINDOW_CORNER_PREFERENCE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTCLOSE, HTLEFT, HTMAXBUTTON,
    HTMINBUTTON, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS,
    SIZE_MAXIMIZED, SIZE_RESTORED, WM_MOUSEMOVE, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST,
    WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_SIZE,
};

type MaximizeCallback = Arc<dyn Fn(&AppWindow, bool) + Send + Sync + 'static>;

struct FrameState {
    weak: slint::Weak<AppWindow>,
    on_maximized: Mutex<Option<MaximizeCallback>>,
}

pub struct WindowFrame {
    state: Arc<FrameState>,
}

impl Clone for WindowFrame {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl WindowFrame {
    const BORDER_WIDTH: i32 = 8;
    const SUBCLASS_ID: usize = 1;

    fn new(component: &AppWindow) -> Self {
        Self {
            state: Arc::new(FrameState {
                weak: component.as_weak(),
                on_maximized: Mutex::new(None),
            }),
        }
    }

    pub fn on_maximized_changed<F>(&self, callback: F)
    where
        F: Fn(&AppWindow, bool) + Send + Sync + 'static,
    {
        let mut guard = self.state.on_maximized.lock().unwrap();
        *guard = Some(Arc::new(callback));
    }

    fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
        self.state.weak.upgrade().map(|c| f(c.window()))
    }

    pub fn maximize(&self, is_maximized: bool) {
        self.with_window(|w| w.set_maximized(is_maximized));
    }

    pub fn toggle_maximized(&self) {
        self.with_window(|w| w.set_maximized(!w.is_maximized()));
    }

    pub fn minimize(&self) {
        self.with_window(|w| w.set_minimized(true));
    }

    pub fn close(&self) {
        slint::quit_event_loop().expect("Failed to quit event loop");
    }

    pub fn drag(&self) {
        self.with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    fn with_winit_window<R>(&self, f: impl FnOnce(&winit::window::Window) -> R) -> Option<R> {
        self.state
            .weak
            .upgrade()
            .and_then(|c| c.window().with_winit_window(|w| f(w)))
    }

    fn apply(&self) {
        self.with_winit_window(|window| {
            let Some(hwnd) = Self::get_hwnd(window) else {
                warn!("Failed to extract HWND from winit window");
                return;
            };
            Self::apply_rounded_corners(hwnd);
            Self::apply_drop_shadow(hwnd);
            Self::install_custom_frame(hwnd, self.state.clone());
        });
    }

    fn get_hwnd(window: &winit::window::Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
            _ => None,
        }
    }

    fn apply_rounded_corners(hwnd: HWND) {
        let preference = DWMWCP_ROUND;
        unsafe {
            if let Err(e) = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const DWM_WINDOW_CORNER_PREFERENCE as *const c_void,
                size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            ) {
                warn!("DwmSetWindowAttribute (rounded corners) failed: {e}");
            }
        }
    }

    fn apply_drop_shadow(hwnd: HWND) {
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 1,
        };
        unsafe {
            if let Err(e) = DwmExtendFrameIntoClientArea(hwnd, &margins) {
                warn!("DwmExtendFrameIntoClientArea (drop shadow) failed: {e}");
            }
        }
    }

    fn install_custom_frame(hwnd: HWND, state: Arc<FrameState>) {
        let ref_data = Arc::into_raw(state) as usize;
        unsafe {
            if !SetWindowSubclass(
                hwnd,
                Some(Self::custom_frame_proc),
                Self::SUBCLASS_ID,
                ref_data,
            )
            .as_bool()
            {
                warn!("SetWindowSubclass (custom frame) failed");
                let _ = Arc::from_raw(ref_data as *const FrameState);
            }
        }
    }

    unsafe extern "system" fn custom_frame_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        uid_subclass: usize,
        ref_data: usize,
    ) -> LRESULT {
        let state_ptr = ref_data as *const FrameState;

        match msg {
            WM_SIZE => {
                let size_type = wparam.0 as u32;
                let is_maximized = match size_type {
                    SIZE_MAXIMIZED => Some(true),
                    SIZE_RESTORED => Some(false),
                    _ => None,
                };

                if let Some(is_max) = is_maximized {
                    if !state_ptr.is_null() {
                        let state = unsafe { &*state_ptr };
                        let callback = state.on_maximized.lock().unwrap().clone();
                        if let Some(cb) = callback {
                            let weak = state.weak.clone();
                            let _ = weak.upgrade_in_event_loop(move |app| {
                                cb(&app, is_max);
                            });
                        }
                    }
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            WM_NCDESTROY => {
                if !state_ptr.is_null() {
                    let _ = unsafe { Arc::from_raw(state_ptr) };
                }
                unsafe {
                    let _ = RemoveWindowSubclass(hwnd, Some(Self::custom_frame_proc), uid_subclass);
                    DefSubclassProc(hwnd, msg, wparam, lparam)
                }
            }

            WM_NCCALCSIZE if wparam.0 != 0 => {
                if unsafe { IsZoomed(hwnd) }.as_bool() {
                    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
                    let mut monitor_info = MONITORINFO {
                        cbSize: size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
                        let params = unsafe { &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS) };
                        params.rgrc[0] = monitor_info.rcWork;
                    }
                }
                LRESULT(0)
            }

            WM_NCHITTEST => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                }

                // 非最大化时判定拉伸边缘
                let is_zoomed = unsafe { IsZoomed(hwnd) }.as_bool();
                if !is_zoomed {
                    let left = x - rect.left < Self::BORDER_WIDTH;
                    let right = rect.right - x <= Self::BORDER_WIDTH;
                    let top = y - rect.top < Self::BORDER_WIDTH;
                    let bottom = rect.bottom - y <= Self::BORDER_WIDTH;

                    if top || bottom || left || right {
                        let hit = if top && left {
                            HTTOPLEFT
                        } else if top && right {
                            HTTOPRIGHT
                        } else if bottom && left {
                            HTBOTTOMLEFT
                        } else if bottom && right {
                            HTBOTTOMRIGHT
                        } else if top {
                            HTTOP
                        } else if bottom {
                            HTBOTTOM
                        } else if left {
                            HTLEFT
                        } else {
                            HTRIGHT
                        };
                        return LRESULT(hit as isize);
                    }
                }

                // 判定三大标题栏控制按钮（从右至左：关闭 -> 最大化 -> 最小化）
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        let dpi = unsafe { GetDpiForWindow(hwnd) };
                        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };

                        let title_h = (controls.get_titlebar_height() * scale) as i32;
                        let close_w = (controls.get_close_width() * scale) as i32;
                        let max_w = (controls.get_maximize_width() * scale) as i32;
                        let min_w = (controls.get_minimize_width() * scale) as i32;

                        let btn_top = rect.top;
                        let btn_bottom = rect.top + title_h;

                        if y >= btn_top && y < btn_bottom {
                            // 1. 关闭按钮判断
                            let close_left = rect.right - close_w;
                            let close_right = rect.right;
                            if x >= close_left && x < close_right {
                                return LRESULT(HTCLOSE as isize);
                            }

                            // 2. 最大化按钮判断
                            let max_left = close_left - max_w;
                            let max_right = close_left;
                            if controls.get_show_maximize() && x >= max_left && x < max_right {
                                return LRESULT(HTMAXBUTTON as isize);
                            }

                            // 3. 最小化按钮判断
                            let min_left = if controls.get_show_maximize() {
                                max_left - min_w
                            } else {
                                close_left - min_w
                            };
                            let min_right = if controls.get_show_maximize() {
                                max_left
                            } else {
                                close_left
                            };
                            if x >= min_left && x < min_right {
                                return LRESULT(HTMINBUTTON as isize);
                            }
                        }
                    }
                }

                LRESULT(HTCLIENT as isize)
            }

            // 实时处理非客户区 Hover 变色联动
            WM_NCMOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        let hit = wparam.0 as usize;

                        let is_min = hit == HTMINBUTTON as usize;
                        let is_max = hit == HTMAXBUTTON as usize;
                        let is_close = hit == HTCLOSE as usize;

                        if controls.get_min_hover() != is_min {
                            controls.set_min_hover(is_min);
                        }
                        if controls.get_max_hover() != is_max {
                            controls.set_max_hover(is_max);
                        }
                        if controls.get_close_hover() != is_close {
                            controls.set_close_hover(is_close);
                        }
                    }
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 移出非客户区重置状态
            WM_NCMOUSELEAVE | WM_MOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        if controls.get_min_hover() {
                            controls.set_min_hover(false);
                        }
                        if controls.get_max_hover() {
                            controls.set_max_hover(false);
                        }
                        if controls.get_close_hover() {
                            controls.set_close_hover(false);
                        }
                    }
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 拦截 Win32 默认点击按压重绘
            WM_NCLBUTTONDOWN
                if wparam.0 == HTMINBUTTON as usize
                    || wparam.0 == HTMAXBUTTON as usize
                    || wparam.0 == HTCLOSE as usize =>
            {
                LRESULT(0)
            }

            // 响应三个按钮的逻辑点击
            WM_NCLBUTTONUP => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        match wparam.0 as usize {
                            x if x == HTMINBUTTON as usize => controls.invoke_minimize(),
                            x if x == HTMAXBUTTON as usize => controls.invoke_maximize(),
                            x if x == HTCLOSE as usize => controls.invoke_close(),
                            _ => {}
                        }
                    }
                }
                LRESULT(0)
            }

            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}

pub trait TitlebarSetup {
    fn setup_borderless(&self) -> Result<WindowFrame, slint::PlatformError>;
}

impl TitlebarSetup for slint::Weak<AppWindow> {
    fn setup_borderless(&self) -> Result<WindowFrame, slint::PlatformError> {
        let component = self.upgrade().ok_or_else(|| {
            slint::PlatformError::Other("Failed to upgrade component handle".to_string())
        })?;
        let frame = WindowFrame::new(&component);

        self.upgrade_in_event_loop({
            let frame = frame.clone();
            move |_| {
                frame.apply();
            }
        })
        .expect("Failed to upgrade window");

        Ok(frame)
    }
}
