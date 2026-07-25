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
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TRACKMOUSEEVENT_FLAGS, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTCLOSE, HTLEFT, HTMAXBUTTON,
    HTMINBUTTON, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS,
    SIZE_MAXIMIZED, SIZE_RESTORED, WM_MOUSEMOVE, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST,
    WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_SIZE,
};

/// 最大化状态变更回调类型
type MaximizeCallback = Arc<dyn Fn(&AppWindow, bool) + Send + Sync + 'static>;

/// 窗口框架的内部状态，通过 Arc 共享
struct FrameState {
    /// 应用窗口的弱引用，用于在事件回调中获取窗口实例
    weak: slint::Weak<AppWindow>,
    /// 最大化状态变更的回调函数（可选）
    on_maximized: Mutex<Option<MaximizeCallback>>,
    /// 记录鼠标按下时命中的按钮区域，防止跨按钮滑动误触
    pressed_hit: Mutex<Option<usize>>,
}

/// 无边框窗口框架的封装，提供窗口控制功能
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
    /// 窗口边缘可拉伸区域的宽度（像素）
    const BORDER_WIDTH: i32 = 8;
    /// 窗口子类化的唯一标识符
    const SUBCLASS_ID: usize = 1;

    /// 创建新的窗口框架实例
    fn new(component: &AppWindow) -> Self {
        Self {
            state: Arc::new(FrameState {
                weak: component.as_weak(),
                on_maximized: Mutex::new(None),
                pressed_hit: Mutex::new(None),
            }),
        }
    }

    /// 设置最大化状态变更的回调函数
    pub fn on_maximized_changed<F>(&self, callback: F)
    where
        F: Fn(&AppWindow, bool) + Send + Sync + 'static,
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

    /// 关闭窗口（隐藏当前窗口；若无其他可见窗口，Slint 将自动退出事件循环）
    pub fn close(&self) {
        self.with_window(|w| w.hide());
    }

    /// 开始拖拽窗口（调用 winit 的原生拖拽功能）
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

    /// 应用无边框窗口样式和行为到窗口
    fn apply(&self) {
        self.with_winit_window(|window| {
            let Some(hwnd) = Self::get_hwnd(window) else {
                warn!("Failed to extract HWND from winit window");
                return;
            };
            // 应用圆角效果
            Self::apply_rounded_corners(hwnd);
            // 应用阴影效果
            Self::apply_drop_shadow(hwnd);
            // 安装自定义窗口过程（子类化）
            Self::install_custom_frame(hwnd, self.state.clone());
        });
    }

    /// 从 winit 窗口获取 Windows HWND 句柄
    fn get_hwnd(window: &winit::window::Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
            _ => None,
        }
    }

    /// 通过 DWM API 应用圆角效果到窗口
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

    /// 通过 DWM API 应用阴影效果到窗口
    /// 使用 DwmExtendFrameIntoClientArea 将框架扩展到客户区来触发阴影
    fn apply_drop_shadow(hwnd: HWND) {
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 1, // 底部扩展1像素以触发阴影
        };
        unsafe {
            if let Err(e) = DwmExtendFrameIntoClientArea(hwnd, &margins) {
                warn!("DwmExtendFrameIntoClientArea (drop shadow) failed: {e}");
            }
        }
    }

    /// 安装自定义窗口过程（子类化），用于拦截和处理窗口消息
    fn install_custom_frame(hwnd: HWND, state: Arc<FrameState>) {
        // 将 Arc 转换为原始指针作为参考数据传递给子类化过程
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
                // 如果子类化失败，需要恢复 Arc 的所有权以避免内存泄漏
                let _ = Arc::from_raw(ref_data as *const FrameState);
            }
        }
    }

    /// 自定义窗口过程（子类化回调），处理各种窗口消息
    /// 这是整个无边框窗口逻辑的核心
    unsafe extern "system" fn custom_frame_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        uid_subclass: usize,
        ref_data: usize,
    ) -> LRESULT {
        // 将参考数据转换回 FrameState 指针
        let state_ptr = ref_data as *const FrameState;

        match msg {
            // 窗口大小变更消息：用于检测最大化/还原状态
            WM_SIZE => {
                let size_type = wparam.0 as u32;
                let is_maximized = match size_type {
                    SIZE_MAXIMIZED => Some(true),   // 窗口被最大化
                    SIZE_RESTORED => Some(false),   // 窗口从最大化还原
                    _ => None,                      // 其他大小变更（如最小化）
                };

                if let Some(is_max) = is_maximized {
                    if !state_ptr.is_null() {
                        let state = unsafe { &*state_ptr };
                        // 获取并调用最大化回调
                        let callback = state.on_maximized.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if let Some(cb) = callback {
                            let weak = state.weak.clone();
                            let _ = weak.upgrade_in_event_loop(move |app| {
                                cb(&app, is_max);
                            });
                        }
                    }
                }
                // 调用默认的子类过程处理其余逻辑
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 窗口销毁消息：清理资源
            WM_NCDESTROY => {
                unsafe {
                    // 1. 先让下游子类和原始窗口过程处理销毁消息
                    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                    // 2. 移除自身子类注册
                    let _ = RemoveWindowSubclass(hwnd, Some(Self::custom_frame_proc), uid_subclass);
                    // 3. 最后释放 Arc，确保此时无任何代码再访问 ref_data
                    if !state_ptr.is_null() {
                        let _ = Arc::from_raw(state_ptr);
                    }
                    result
                }
            }

            // 非客户区大小计算：用于处理最大化时的工作区适配
            WM_NCCALCSIZE if wparam.0 != 0 => {
                if unsafe { IsZoomed(hwnd) }.as_bool() {
                    // 获取窗口所在的显示器信息
                    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
                    let mut monitor_info = MONITORINFO {
                        cbSize: size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
                        // 将窗口矩形设置为显示器工作区（排除任务栏等）
                        let params = unsafe { &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS) };
                        params.rgrc[0] = monitor_info.rcWork;
                    }
                }
                // 返回0表示我们已处理此消息
                LRESULT(0)
            }

            // 非客户区点击测试：用于自定义标题栏按钮和边缘拉伸
            WM_NCHITTEST => {
                // 从 lparam 中提取鼠标坐标（低位为x，高位为y）
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                }

                // 最大化时，GetWindowRect 返回的矩形比实际可见区域大（四边各多出
                // ~8px 不可见边框），但 Slint UI 是按 WM_NCCALCSIZE 设定的工作区
                // 绘制的。必须用工作区矩形做按钮命中测试，否则按钮区域会右偏 8px。
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
                // 优先检查标题栏控制按钮（按钮优先级 > 边缘拉伸，
                // 否则按钮角落 8px 区域会被误判为边缘拉伸）
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        // 获取窗口控制的全局状态
                        let controls = app.global::<crate::WindowControls>();
                        // 获取 DPI 缩放因子
                        let dpi = unsafe { GetDpiForWindow(hwnd) };
                        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };

                        // 根据 DPI 缩放按钮尺寸
                        let title_h = (controls.get_titlebar_height() * scale) as i32;
                        let close_w = (controls.get_close_width() * scale) as i32;
                        let max_w = (controls.get_maximize_width() * scale) as i32;
                        let min_w = (controls.get_minimize_width() * scale) as i32;

                        let btn_top = rect.top;
                        let btn_bottom = rect.top + title_h;

                        if y >= btn_top && y < btn_bottom {
                            // 1. 关闭按钮区域判断（从右至左：关闭 -> 最大化 -> 最小化）
                            let close_left = rect.right - close_w;
                            let close_right = rect.right;
                            if x >= close_left && x < close_right {
                                return LRESULT(HTCLOSE as isize);
                            }

                            // 2. 最大化按钮区域判断（可选显示）
                            let max_left = close_left - max_w;
                            let max_right = close_left;
                            if controls.get_show_maximize() && x >= max_left && x < max_right {
                                return LRESULT(HTMAXBUTTON as isize);
                            }

                            // 3. 最小化按钮区域判断（位置根据是否显示最大化按钮调整）
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

                // 按钮未命中后，非最大化时判定窗口边缘拉伸区域
                if !is_zoomed {
                    let left = x - rect.left < Self::BORDER_WIDTH;
                    let right = rect.right - x <= Self::BORDER_WIDTH;
                    let top = y - rect.top < Self::BORDER_WIDTH;
                    let bottom = rect.bottom - y <= Self::BORDER_WIDTH;

                    if top || bottom || left || right {
                        // 根据鼠标位置返回对应的边缘/角点命中测试结果
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

                // 默认返回客户区，允许窗口内容接收鼠标事件
                LRESULT(HTCLIENT as isize)
            }

            // 非客户区鼠标移动：实时更新按钮悬停状态
            WM_NCMOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        // wparam 包含命中测试结果
                        let hit = wparam.0 as usize;

                        // 判断鼠标悬停在哪个按钮上
                        let is_min = hit == HTMINBUTTON as usize;
                        let is_max = hit == HTMAXBUTTON as usize;
                        let is_close = hit == HTCLOSE as usize;

                        // 更新全局状态以触发 UI 更新
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
                // Bug 3 修复：WM_NCMOUSELEAVE 是一次性通知，每次 WM_NCMOUSEMOVE 都必须重新订阅，
                // 否则系统永远不会发送 WM_NCMOUSELEAVE，hover 状态将永久卡住。
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TRACKMOUSEEVENT_FLAGS(TME_NONCLIENT.0 | TME_LEAVE.0),
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe { let _ = TrackMouseEvent(&mut tme); }

                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 鼠标离开非客户区或进入客户区：重置按钮悬停状态
            WM_NCMOUSELEAVE | WM_MOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        // 重置所有按钮的悬停状态
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
                    // 清理可能残留的按下状态（如按住按钮拖出窗口后松手）
                    *state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()) = None;
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 拦截标题栏按钮的按下消息：记录按下区域，阻止默认视觉效果
            WM_NCLBUTTONDOWN
                if wparam.0 == HTMINBUTTON as usize
                    || wparam.0 == HTMAXBUTTON as usize
                    || wparam.0 == HTCLOSE as usize =>
            {
                // Bug 4 修复：记录按下时的命中区域，用于松手时的区域一致性校验
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    *state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()) = Some(wparam.0);
                }
                LRESULT(0)
            }

            // 处理标题栏按钮的释放消息：仅当按下与松开在同一按钮时才执行操作
            WM_NCLBUTTONUP => {
                let hit = wparam.0 as usize;
                let is_ctrl_button = hit == HTMINBUTTON as usize
                    || hit == HTMAXBUTTON as usize
                    || hit == HTCLOSE as usize;

                if is_ctrl_button {
                    if !state_ptr.is_null() {
                        let state = unsafe { &*state_ptr };
                        // Bug 4 修复：取出按下时的区域，与松开区域比较，防止跨按钮滑动误触
                        let pressed = state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()).take();
                        if pressed == Some(wparam.0) {
                            if let Some(app) = state.weak.upgrade() {
                                let controls = app.global::<crate::WindowControls>();
                                match hit {
                                    x if x == HTMINBUTTON as usize => controls.invoke_minimize(),
                                    x if x == HTMAXBUTTON as usize => controls.invoke_maximize(),
                                    x if x == HTCLOSE as usize => controls.invoke_close(),
                                    _ => {}
                                }
                            }
                        }
                    }
                    LRESULT(0)
                } else {
                    // 非控制按钮区域的释放消息交给默认处理（如标题栏双击还原等）
                    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
                }
            }

            // 其他未处理的消息：调用默认的子类过程
            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}

/// 为 Slint 组件提供无边框窗口设置的 trait
pub trait TitlebarSetup {
    fn setup_borderless(&self) -> Result<WindowFrame, slint::PlatformError>;
}

/// 在 slint::Weak<AppWindow> 上实现无边框窗口设置
impl TitlebarSetup for slint::Weak<AppWindow> {
    fn setup_borderless(&self) -> Result<WindowFrame, slint::PlatformError> {
        // 尝试升级弱引用
        let component = self.upgrade().ok_or_else(|| {
            slint::PlatformError::Other("Failed to upgrade component handle".to_string())
        })?;
        // 创建窗口框架实例
        let frame = WindowFrame::new(&component);

        // 在事件循环中应用无边框样式
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
