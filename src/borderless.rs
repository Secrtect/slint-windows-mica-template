#![allow(dead_code)]

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
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTLEFT, HTRIGHT, HTTOP,
    HTTOPLEFT, HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS, SIZE_MAXIMIZED, SIZE_RESTORED,
    WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST, WM_SIZE,
};

type MaximizeCallback<T> = Arc<dyn Fn(&T, bool) + Send + Sync + 'static>;

struct FrameState<T: ComponentHandle + 'static> {
    weak: slint::Weak<T>,
    on_maximized: Mutex<Option<MaximizeCallback<T>>>,
}

pub struct WindowFrame<T: ComponentHandle + 'static> {
    state: Arc<FrameState<T>>,
}

impl<T: ComponentHandle + 'static> Clone for WindowFrame<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<T: ComponentHandle + 'static> WindowFrame<T> {
    const BORDER_WIDTH: i32 = 8;
    const SUBCLASS_ID: usize = 1;

    fn new(component: &T) -> Self {
        Self {
            state: Arc::new(FrameState {
                weak: component.as_weak(),
                on_maximized: Mutex::new(None),
            }),
        }
    }

    /// 注册底层窗口最大化/还原状态改变时的通知回调
    pub fn on_maximized_changed<F>(&self, callback: F)
    where
        F: Fn(&T, bool) + Send + Sync + 'static,
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

    fn install_custom_frame(hwnd: HWND, state: Arc<FrameState<T>>) {
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
                let _ = Arc::from_raw(ref_data as *const FrameState<T>);
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
        match msg {
            // 监听窗口尺寸变化
            WM_SIZE => {
                let size_type = wparam.0 as u32;
                let is_maximized = match size_type {
                    SIZE_MAXIMIZED => Some(true),
                    SIZE_RESTORED => Some(false),
                    _ => None,
                };

                if let Some(is_max) = is_maximized {
                    let state_ptr = ref_data as *const FrameState<T>;
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

            // 窗口销毁时清理 Subclass 与 Raw 指针，防止内存泄漏
            WM_NCDESTROY => {
                let state_ptr = ref_data as *const FrameState<T>;
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
                if unsafe { IsZoomed(hwnd) }.as_bool() {
                    return LRESULT(HTCLIENT as isize);
                }

                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                }

                let left = x - rect.left < Self::BORDER_WIDTH;
                let right = rect.right - x <= Self::BORDER_WIDTH;
                let top = y - rect.top < Self::BORDER_WIDTH;
                let bottom = rect.bottom - y <= Self::BORDER_WIDTH;

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
                } else if right {
                    HTRIGHT
                } else {
                    HTCLIENT
                };

                LRESULT(hit as isize)
            }

            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}

pub trait TitlebarSetup<T: ComponentHandle> {
    fn setup_borderless(&self) -> Result<WindowFrame<T>, slint::PlatformError>;
}

impl<T: ComponentHandle + 'static> TitlebarSetup<T> for slint::Weak<T> {
    fn setup_borderless(&self) -> Result<WindowFrame<T>, slint::PlatformError> {
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
