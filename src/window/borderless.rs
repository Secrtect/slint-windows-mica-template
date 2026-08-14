#![allow(dead_code)]

use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use slint::Window;
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::{Arc, Mutex};
use tracing::warn;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TRACKMOUSEEVENT_FLAGS, TrackMouseEvent,
};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTCLOSE, HTLEFT, HTMAXBUTTON,
    HTMINBUTTON, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS,
    SIZE_MAXIMIZED, SIZE_RESTORED, SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SetWindowPos, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WA_INACTIVE, WM_ACTIVATE, WM_CANCELMODE,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCACTIVATE, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST,
    WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_SIZE,
};
use winit::platform::windows::WindowExtWindows;

/// 标题栏度量尺寸配置（逻辑像素单位，会自动随 DPI 缩放）
#[derive(Debug, Clone)]
pub struct TitlebarMetrics {
    pub titlebar_height: f32,
    pub close_width: f32,
    pub maximize_width: f32,
    pub minimize_width: f32,
    pub show_minimize: bool,
    pub show_maximize: bool,
    pub show_close: bool,
}

impl Default for TitlebarMetrics {
    fn default() -> Self {
        Self {
            titlebar_height: 32.0,
            close_width: 46.0,
            maximize_width: 46.0,
            minimize_width: 46.0,
            show_minimize: true,
            show_maximize: true,
            show_close: true,
        }
    }
}

/// 标题栏适配器接口：连接 Win32 消息与特定 Slint 窗口的 UI 状态
pub trait TitlebarAdapter<T: ComponentHandle>: Send + Sync + 'static {
    /// 获取当前窗口的标题栏几何尺寸与按钮可见性
    fn metrics(&self, component: &T) -> TitlebarMetrics;

    /// 窗口激活状态改变（聚焦/失焦）
    fn set_active(&self, component: &T, is_active: bool) {
        let _ = (component, is_active);
    }

    /// 按钮悬停状态更新 (min, max, close)
    fn set_hover(&self, component: &T, min: bool, max: bool, close: bool) {
        let _ = (component, min, max, close);
    }

    /// 按钮按下状态更新 (min, max, close)
    fn set_pressed(&self, component: &T, min: bool, max: bool, close: bool) {
        let _ = (component, min, max, close);
    }

    /// 点击最小化按钮
    fn on_minimize_clicked(&self, component: &T, frame: &WindowFrame<T>) {
        let _ = component;
        frame.minimize();
    }

    /// 点击最大化/还原按钮
    fn on_maximize_clicked(&self, component: &T, frame: &WindowFrame<T>) {
        let _ = component;
        frame.toggle_maximized();
    }

    /// 点击关闭按钮
    fn on_close_clicked(&self, component: &T, frame: &WindowFrame<T>) {
        let _ = component;
        frame.close();
    }
}

/// 默认空适配器（仅提供基础边缘拉伸与原生阴影，不拦截非客户区控制按钮）
pub struct SimpleBorderlessAdapter;
impl<T: ComponentHandle> TitlebarAdapter<T> for SimpleBorderlessAdapter {
    fn metrics(&self, _component: &T) -> TitlebarMetrics {
        TitlebarMetrics {
            show_minimize: false,
            show_maximize: false,
            show_close: false,
            ..Default::default()
        }
    }
}

/// 最大化状态变更回调类型
type MaximizeCallback<T> = Arc<dyn Fn(&T, bool) + Send + Sync + 'static>;

/// 窗口框架内部状态
struct FrameState<T: ComponentHandle + 'static> {
    weak: slint::Weak<T>,
    adapter: Arc<dyn TitlebarAdapter<T>>,
    on_maximized: Mutex<Option<MaximizeCallback<T>>>,
    pressed_hit: Mutex<Option<usize>>,
}

/// 无边框窗口框架，提供窗口控制方法
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
    const SUBCLASS_ID: usize = 1001;

    /// 创建窗口框架实例
    pub fn new(component: &T, adapter: Arc<dyn TitlebarAdapter<T>>) -> Self {
        Self {
            state: Arc::new(FrameState {
                weak: component.as_weak(),
                adapter,
                on_maximized: Mutex::new(None),
                pressed_hit: Mutex::new(None),
            }),
        }
    }

    /// 根据窗口 DPI 动态获取系统标准拉伸边框宽度（SM_CXSIZEFRAME + SM_CXPADDEDBORDER）
    fn get_resize_border_width(hwnd: HWND) -> i32 {
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        if dpi != 0 {
            unsafe {
                GetSystemMetricsForDpi(SM_CXSIZEFRAME, dpi)
                    + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
            }
        } else {
            8
        }
    }

    /// 设置最大化状态变更的回调函数
    pub fn on_maximized_changed<F>(&self, callback: F)
    where
        F: Fn(&T, bool) + Send + Sync + 'static,
    {
        let mut guard = self.state.on_maximized.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(Arc::new(callback));
    }

    /// 在持有窗口实例的情况下执行闭包
    fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
        self.state.weak.upgrade().map(|c| f(c.window()))
    }

    /// 设置窗口最大化状态
    pub fn maximize(&self, is_maximized: bool) {
        self.with_window(|w| w.set_maximized(is_maximized));
    }

    /// 切换窗口最大化状态
    pub fn toggle_maximized(&self) {
        self.with_window(|w| w.set_maximized(!w.is_maximized()));
    }

    /// 最小化窗口
    pub fn minimize(&self) {
        self.with_window(|w| w.set_minimized(true));
    }

    /// 关闭/隐藏当前窗口
    pub fn close(&self) {
        self.with_window(|w| w.hide());
    }

    /// 开始拖拽窗口（调用 winit 的原生拖拽）
    pub fn drag(&self) {
        self.with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    /// 在持有 winit 原生窗口的情况下执行闭包
    fn with_winit_window<R>(&self, f: impl FnOnce(&winit::window::Window) -> R) -> Option<R> {
        self.state
            .weak
            .upgrade()
            .and_then(|c| c.window().with_winit_window(|w| f(w)))
    }

    /// 应用无边框窗口样式与子类化
    pub fn apply(&self) {
        self.with_winit_window(|window| {
            let Some(hwnd) = Self::get_hwnd(window) else {
                warn!("Failed to extract HWND from winit window");
                return;
            };

            // 1. 恢复无边框窗口阴影
            window.set_undecorated_shadow(true);

            // 2. Win10 阴影修复
            if !crate::sys_info::is_win11() {
                use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
                use windows::Win32::UI::Controls::MARGINS;

                let margins = MARGINS {
                    cxLeftWidth: 0,
                    cxRightWidth: 0,
                    cyTopHeight: 0,
                    cyBottomHeight: 1,
                };
                unsafe {
                    if let Err(e) = DwmExtendFrameIntoClientArea(hwnd, &margins) {
                        warn!("Win10 DwmExtendFrameIntoClientArea failed: {e}");
                    }
                }
            }

            // 3. 安装自定义窗口过程（子类化）
            Self::install_custom_frame(hwnd, self.state.clone());

            // 4. 触发 SWP_FRAMECHANGED 通知 DWM 刷新非客户区 metrics 与 Snap Layouts 缓存
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
                );
            }
        });
    }

    /// 从 winit 窗口获取 Windows HWND
    fn get_hwnd(window: &winit::window::Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
            _ => None,
        }
    }

    /// 安装自定义窗口过程
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

    /// 核心 Win32 子类化消息过程
    unsafe extern "system" fn custom_frame_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        uid_subclass: usize,
        ref_data: usize,
    ) -> LRESULT {
        let state_ptr = ref_data as *const FrameState<T>;

        match msg {
            // 窗口激活/失焦
            WM_ACTIVATE => {
                let is_active = (wparam.0 & 0xFFFF) as u16 != WA_INACTIVE as u16;
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let weak = state.weak.clone();
                    let adapter = state.adapter.clone();
                    let _ = weak.upgrade_in_event_loop(move |component| {
                        adapter.set_active(&component, is_active);
                        if !is_active {
                            adapter.set_hover(&component, false, false, false);
                            adapter.set_pressed(&component, false, false, false);
                        }
                    });
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 非客户区激活
            WM_NCACTIVATE => {
                let composition_enabled = unsafe {
                    windows::Win32::Graphics::Dwm::DwmIsCompositionEnabled()
                };
                if composition_enabled.map(|b| b.as_bool()).unwrap_or(false) {
                    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
                } else {
                    LRESULT(1)
                }
            }

            // 窗口大小变更检测（最大化/还原）
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
                        let callback = state.on_maximized.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        let weak = state.weak.clone();
                        let _ = weak.upgrade_in_event_loop(move |component| {
                            if let Some(cb) = callback {
                                cb(&component, is_max);
                            }
                        });
                    }
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 窗口销毁时注销并释放 Arc
            WM_NCDESTROY => {
                unsafe {
                    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                    let _ = RemoveWindowSubclass(hwnd, Some(Self::custom_frame_proc), uid_subclass);
                    if !state_ptr.is_null() {
                        let _ = Arc::from_raw(state_ptr);
                    }
                    result
                }
            }

            // 非客户区大小计算（最大化时排除任务栏）
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

            // 非客户区命中测试（标题栏按钮与边缘拉伸）
            WM_NCHITTEST => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                }

                let is_zoomed = unsafe { IsZoomed(hwnd) }.as_bool();
                if is_zoomed {
                    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
                    let mut monitor_info = MONITORINFO {
                        cbSize: size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
                        rect = monitor_info.rcWork;
                    }
                }

                // 1. 优先判定标题栏控制按钮（使 Win11 Snap Layouts 贴靠布局生效）
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(component) = state.weak.upgrade() {
                        let metrics = state.adapter.metrics(&component);
                        let dpi = unsafe { GetDpiForWindow(hwnd) };
                        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };

                        let title_h = (metrics.titlebar_height * scale) as i32;
                        let close_w = (metrics.close_width * scale) as i32;
                        let max_w = (metrics.maximize_width * scale) as i32;
                        let min_w = (metrics.minimize_width * scale) as i32;

                        let btn_top = rect.top;
                        let btn_bottom = rect.top + title_h;

                        if y >= btn_top && y < btn_bottom {
                            // 关闭按钮
                            if metrics.show_close {
                                let close_left = rect.right - close_w;
                                if x >= close_left && x < rect.right {
                                    return LRESULT(HTCLOSE as isize);
                                }
                            }

                            let effective_close_w = if metrics.show_close { close_w } else { 0 };

                            // 最大化按钮
                            if metrics.show_maximize {
                                let max_right = rect.right - effective_close_w;
                                let max_left = max_right - max_w;
                                if x >= max_left && x < max_right {
                                    return LRESULT(HTMAXBUTTON as isize);
                                }
                            }

                            let effective_max_w = if metrics.show_maximize { max_w } else { 0 };

                            // 最小化按钮
                            if metrics.show_minimize {
                                let min_right = rect.right - effective_close_w - effective_max_w;
                                let min_left = min_right - min_w;
                                if x >= min_left && x < min_right {
                                    return LRESULT(HTMINBUTTON as isize);
                                }
                            }
                        }
                    }
                }

                // 2. 判定窗口边缘拉伸区域
                if !is_zoomed {
                    let border_width = Self::get_resize_border_width(hwnd);
                    let left = x - rect.left < border_width;
                    let right = rect.right - x <= border_width;
                    let top = y - rect.top < border_width;
                    let bottom = rect.bottom - y <= border_width;

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

                // 3. 客户区
                LRESULT(HTCLIENT as isize)
            }

            // 非客户区鼠标移动
            WM_NCMOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let hit = wparam.0 as usize;
                    let pressed = state
                        .pressed_hit
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();

                    let weak = state.weak.clone();
                    let adapter = state.adapter.clone();
                    let _ = weak.upgrade_in_event_loop(move |component| {
                        if let Some(pressed_code) = pressed {
                            if hit == pressed_code {
                                let is_min = pressed_code == HTMINBUTTON as usize;
                                let is_max = pressed_code == HTMAXBUTTON as usize;
                                let is_close = pressed_code == HTCLOSE as usize;
                                adapter.set_pressed(&component, is_min, is_max, is_close);
                                adapter.set_hover(&component, false, false, false);
                            } else {
                                adapter.set_pressed(&component, false, false, false);
                                adapter.set_hover(&component, false, false, false);
                            }
                        } else {
                            let is_min = hit == HTMINBUTTON as usize;
                            let is_max = hit == HTMAXBUTTON as usize;
                            let is_close = hit == HTCLOSE as usize;
                            adapter.set_hover(&component, is_min, is_max, is_close);
                            adapter.set_pressed(&component, false, false, false);
                        }
                    });
                }

                let mut tme = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TRACKMOUSEEVENT_FLAGS(TME_NONCLIENT.0 | TME_LEAVE.0),
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe { let _ = TrackMouseEvent(&mut tme); }

                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 鼠标离开非客户区
            WM_NCMOUSELEAVE | WM_MOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let weak = state.weak.clone();
                    let adapter = state.adapter.clone();
                    let _ = weak.upgrade_in_event_loop(move |component| {
                        adapter.set_hover(&component, false, false, false);
                        adapter.set_pressed(&component, false, false, false);
                    });
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 标题栏按钮按下
            WM_NCLBUTTONDOWN
                if wparam.0 == HTMINBUTTON as usize
                    || wparam.0 == HTMAXBUTTON as usize
                    || wparam.0 == HTCLOSE as usize =>
            {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let hit = wparam.0;
                    *state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()) = Some(hit);

                    let weak = state.weak.clone();
                    let adapter = state.adapter.clone();
                    let _ = weak.upgrade_in_event_loop(move |component| {
                        adapter.set_pressed(
                            &component,
                            hit == HTMINBUTTON as usize,
                            hit == HTMAXBUTTON as usize,
                            hit == HTCLOSE as usize,
                        );
                        adapter.set_hover(&component, false, false, false);
                    });
                }
                LRESULT(0)
            }

            // 标题栏按钮释放
            WM_NCLBUTTONUP | WM_LBUTTONUP => {
                let hit = wparam.0 as usize;
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let pressed = state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()).take();

                    let weak = state.weak.clone();
                    let adapter = state.adapter.clone();
                    let frame = WindowFrame {
                        state: Arc::new(FrameState {
                            weak: state.weak.clone(),
                            adapter: state.adapter.clone(),
                            on_maximized: Mutex::new(None),
                            pressed_hit: Mutex::new(None),
                        }),
                    };

                    let _ = weak.upgrade_in_event_loop(move |component| {
                        adapter.set_pressed(&component, false, false, false);
                        adapter.set_hover(
                            &component,
                            hit == HTMINBUTTON as usize,
                            hit == HTMAXBUTTON as usize,
                            hit == HTCLOSE as usize,
                        );

                        if let Some(pressed_hit) = pressed {
                            if pressed_hit == hit {
                                match hit {
                                    x if x == HTMINBUTTON as usize => {
                                        adapter.on_minimize_clicked(&component, &frame);
                                    }
                                    x if x == HTMAXBUTTON as usize => {
                                        adapter.on_maximize_clicked(&component, &frame);
                                    }
                                    x if x == HTCLOSE as usize => {
                                        adapter.on_close_clicked(&component, &frame);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    });
                }

                let is_ctrl_button = hit == HTMINBUTTON as usize
                    || hit == HTMAXBUTTON as usize
                    || hit == HTCLOSE as usize;
                if is_ctrl_button {
                    LRESULT(0)
                } else {
                    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
                }
            }

            // 取消模式
            WM_CANCELMODE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    *state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    let weak = state.weak.clone();
                    let adapter = state.adapter.clone();
                    let _ = weak.upgrade_in_event_loop(move |component| {
                        adapter.set_hover(&component, false, false, false);
                        adapter.set_pressed(&component, false, false, false);
                    });
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}

/// 为 Slint 弱引用提供无边框窗口初始化的 Trait
pub trait TitlebarSetup<T: ComponentHandle + 'static> {
    /// 使用指定适配器安装无边框支持
    fn setup_borderless_with_adapter<A: TitlebarAdapter<T> + 'static>(
        &self,
        adapter: A,
    ) -> Result<WindowFrame<T>, slint::PlatformError>;

    /// 使用默认简易适配器安装基础无边框支持（带边缘拉伸与阴影）
    fn setup_borderless_simple(&self) -> Result<WindowFrame<T>, slint::PlatformError> {
        self.setup_borderless_with_adapter(SimpleBorderlessAdapter)
    }
}

impl<T: ComponentHandle + 'static> TitlebarSetup<T> for slint::Weak<T> {
    fn setup_borderless_with_adapter<A: TitlebarAdapter<T> + 'static>(
        &self,
        adapter: A,
    ) -> Result<WindowFrame<T>, slint::PlatformError> {
        let component = self.upgrade().ok_or_else(|| {
            slint::PlatformError::Other("Failed to upgrade component handle".to_string())
        })?;

        let frame = WindowFrame::new(&component, Arc::new(adapter));

        self.upgrade_in_event_loop({
            let frame = frame.clone();
            move |_| {
                frame.apply();
            }
        })
        .map_err(|e| slint::PlatformError::Other(format!("Failed to schedule borderless setup: {e}")))?;

        Ok(frame)
    }
}
