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
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use winit::platform::windows::{CornerPreference, WindowExtWindows};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TRACKMOUSEEVENT_FLAGS, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCLIENT, HTCLOSE, HTLEFT, HTMAXBUTTON,
    HTMINBUTTON, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed, NCCALCSIZE_PARAMS,
    SIZE_MAXIMIZED, SIZE_RESTORED, SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SetWindowPos, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_CANCELMODE, WM_LBUTTONUP,
    WM_MOUSEMOVE, WM_NCCALCSIZE, WM_NCDESTROY, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP,
    WM_NCMOUSELEAVE, WM_NCMOUSEMOVE, WM_SIZE,
};

/// 最大化状态变更回调类型
/// Maximize state change callback type
type MaximizeCallback = Arc<dyn Fn(&AppWindow, bool) + Send + Sync + 'static>;

/// 窗口框架的内部状态，通过 Arc 共享
/// Internal state of the window frame, shared via Arc
struct FrameState {
    /// 应用窗口的弱引用，用于在事件回调中获取窗口实例
    /// Weak reference to the application window, used to get the window instance in event callbacks
    weak: slint::Weak<AppWindow>,
    /// 最大化状态变更的回调函数（可选）
    /// Callback function for maximize state changes (optional)
    on_maximized: Mutex<Option<MaximizeCallback>>,
    /// 记录鼠标按下时命中的按钮区域，防止跨按钮滑动误触
    /// Records the hit button area when mouse is pressed, preventing cross-button slide misclicks
    pressed_hit: Mutex<Option<usize>>,
}

/// 无边框窗口框架的封装，提供窗口控制功能
/// Encapsulation of borderless window frame, providing window control functionality
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
    /// 根据窗口 DPI 动态获取系统标准拉伸边框宽度（SM_CXSIZEFRAME + SM_CXPADDEDBORDER）
    /// Dynamically gets the system standard resizable border width for the window's DPI
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

    /// 窗口子类化的唯一标识符
    /// Unique identifier for window subclassing
    const SUBCLASS_ID: usize = 1;

    /// 创建新的窗口框架实例
    /// Creates a new window frame instance
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
    /// Sets the callback for maximize state changes
    pub fn on_maximized_changed<F>(&self, callback: F)
    where
        F: Fn(&AppWindow, bool) + Send + Sync + 'static,
    {
        let mut guard = self.state.on_maximized.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(Arc::new(callback));
    }

    /// 在持有窗口实例的情况下执行闭包
    /// Executes a closure while holding the window instance
    fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
        self.state.weak.upgrade().map(|c| f(c.window()))
    }

    /// 设置窗口最大化状态
    /// Sets the window maximize state
    pub fn maximize(&self, is_maximized: bool) {
        self.with_window(|w| w.set_maximized(is_maximized));
    }

    /// 切换窗口最大化状态
    /// Toggles the window maximize state
    pub fn toggle_maximized(&self) {
        self.with_window(|w| w.set_maximized(!w.is_maximized()));
    }

    /// 最小化窗口
    /// Minimizes the window
    pub fn minimize(&self) {
        self.with_window(|w| w.set_minimized(true));
    }

    /// 关闭窗口（隐藏当前窗口；若无其他可见窗口，Slint 将自动退出事件循环）
    /// Closes the window (hides the current window; Slint will automatically exit the event loop if no other visible windows exist)
    pub fn close(&self) {
        self.with_window(|w| w.hide());
    }

    /// 开始拖拽窗口（调用 winit 的原生拖拽功能）
    /// Starts dragging the window (calls winit's native drag functionality)
    pub fn drag(&self) {
        self.with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    /// 在持有 winit 原生窗口的情况下执行闭包
    /// Executes a closure while holding the winit native window
    fn with_winit_window<R>(&self, f: impl FnOnce(&winit::window::Window) -> R) -> Option<R> {
        self.state
            .weak
            .upgrade()
            .and_then(|c| c.window().with_winit_window(|w| f(w)))
    }

    /// 应用无边框窗口样式和行为到窗口
    /// Applies borderless window style and behavior to the window
    fn apply(&self) {
        self.with_winit_window(|window| {
            let Some(hwnd) = Self::get_hwnd(window) else {
                warn!("Failed to extract HWND from winit window");
                return;
            };
            // 圆角 + 阴影直接用 winit 封装好的 API，无需手写 DWM FFI
            // Rounded corners and shadow via winit's built-in Windows API wrappers
            window.set_corner_preference(CornerPreference::Round);
            window.set_undecorated_shadow(true);
            // 安装自定义窗口过程（子类化）
            // Install custom window procedure (subclassing)
            Self::install_custom_frame(hwnd, self.state.clone());

            // 触发 SWP_FRAMECHANGED 通知 DWM 重新计算和更新非客户区框架与 hit-test 缓存
            // 解决 Win11 贴靠布局 (Snap Layouts) 在刚打开窗口悬停最大化按钮时不弹出的问题
            // Trigger SWP_FRAMECHANGED to notify DWM to recalculate non-client frame metrics
            // Fixes Win11 Snap Layouts hover popup not appearing when window first opens
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

    /// 从 winit 窗口获取 Windows HWND 句柄
    /// Gets the Windows HWND handle from the winit window
    fn get_hwnd(window: &winit::window::Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let handle = window.window_handle().ok()?;
        match handle.as_raw() {
            RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut c_void)),
            _ => None,
        }
    }

    /// 安装自定义窗口过程（子类化），用于拦截和处理窗口消息
    /// Installs a custom window procedure (subclassing) for intercepting and handling window messages
    fn install_custom_frame(hwnd: HWND, state: Arc<FrameState>) {
        // 将 Arc 转换为原始指针作为参考数据传递给子类化过程
        // Convert Arc to raw pointer for passing as reference data to subclassing procedure
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
                // If subclassing fails, recover Arc ownership to avoid memory leak
                let _ = Arc::from_raw(ref_data as *const FrameState);
            }
        }
    }

    /// 自定义窗口过程（子类化回调），处理各种窗口消息
    /// 这是整个无边框窗口逻辑的核心
    /// Custom window procedure (subclassing callback) that handles various window messages
    /// This is the core of the entire borderless window logic
    unsafe extern "system" fn custom_frame_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        uid_subclass: usize,
        ref_data: usize,
    ) -> LRESULT {
        // 将参考数据转换回 FrameState 指针
        // Convert reference data back to FrameState pointer
        let state_ptr = ref_data as *const FrameState;

        match msg {
            // 窗口大小变更消息：用于检测最大化/还原状态
            // Window size change message: used to detect maximize/restore state
            WM_SIZE => {
                let size_type = wparam.0 as u32;
                let is_maximized = match size_type {
                    SIZE_MAXIMIZED => Some(true),   // 窗口被最大化 / Window is maximized
                    SIZE_RESTORED => Some(false),   // 窗口从最大化还原 / Window is restored from maximize
                    _ => None,                      // 其他大小变更（如最小化）/ Other size changes (e.g., minimized)
                };

                if let Some(is_max) = is_maximized {
                    if !state_ptr.is_null() {
                        let state = unsafe { &*state_ptr };
                        // 获取并调用最大化回调 / Get and invoke the maximize callback
                        let callback = state.on_maximized.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        if let Some(cb) = callback {
                            let weak = state.weak.clone();
                            let _ = weak.upgrade_in_event_loop(move |app| {
                                cb(&app, is_max);
                            });
                        }
                    }
                }
                // 调用默认的子类过程处理其余逻辑 / Call default subclass procedure to handle remaining logic
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 窗口销毁消息：清理资源
            // Window destruction message: clean up resources
            WM_NCDESTROY => {
                unsafe {
                    // 1. 先让下游子类和原始窗口过程处理销毁消息
                    // 1. Let downstream subclasses and original window procedure handle the destruction message first
                    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                    // 2. 移除自身子类注册
                    // 2. Remove self from subclass registration
                    let _ = RemoveWindowSubclass(hwnd, Some(Self::custom_frame_proc), uid_subclass);
                    // 3. 最后释放 Arc，确保此时无任何代码再访问 ref_data
                    // 3. Finally release the Arc, ensuring no code accesses ref_data at this point
                    if !state_ptr.is_null() {
                        let _ = Arc::from_raw(state_ptr);
                    }
                    result
                }
            }

            // 非客户区大小计算：用于处理最大化时的工作区适配
            // Non-client area size calculation: used to handle workspace adaptation when maximized
            WM_NCCALCSIZE if wparam.0 != 0 => {
                if unsafe { IsZoomed(hwnd) }.as_bool() {
                    // 获取窗口所在的显示器信息
                    // Get monitor info for the window's display
                    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
                    let mut monitor_info = MONITORINFO {
                        cbSize: size_of::<MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
                        // 将窗口矩形设置为显示器工作区（排除任务栏等）
                        // Set window rectangle to monitor workspace (excluding taskbar, etc.)
                        let params = unsafe { &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS) };
                        params.rgrc[0] = monitor_info.rcWork;
                    }
                }
                // 返回0表示我们已处理此消息
                // Return 0 indicates we've handled this message
                LRESULT(0)
            }

            // 非客户区点击测试：用于自定义标题栏按钮和边缘拉伸
            // Non-client area hit testing: used for custom titlebar buttons and edge resizing
            WM_NCHITTEST => {
                // 从 lparam 中提取鼠标坐标（低位为x，高位为y）
                // Extract mouse coordinates from lparam (lower 16 bits for x, upper 16 bits for y)
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };
                }

                // 最大化时，GetWindowRect 返回的矩形比实际可见区域大（四边各多出
                // ~8px 不可见边框），但 Slint UI 是按 WM_NCCALCSIZE 设定的工作区
                // 绘制的。必须用工作区矩形做按钮命中测试，否则按钮区域会右偏 8px。
                // When maximized, GetWindowRect returns a rectangle larger than the actual visible area
                // (~8px invisible border on each side), but Slint UI is drawn according to the workspace
                // set by WM_NCCALCSIZE. We must use the workspace rectangle for button hit testing,
                // otherwise the button area will be offset 8px to the right.
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
                // Check titlebar control buttons first (button priority > edge resizing,
                // otherwise the 8px corner area of buttons would be misjudged as edge resizing)
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        // 获取窗口控制的全局状态
                        // Get global state for window controls
                        let controls = app.global::<crate::WindowControls>();
                        // 获取 DPI 缩放因子
                        // Get DPI scaling factor
                        let dpi = unsafe { GetDpiForWindow(hwnd) };
                        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };

                        // 根据 DPI 缩放按钮尺寸
                        // Scale button dimensions according to DPI
                        let title_h = (controls.get_titlebar_height() * scale) as i32;
                        let close_w = (controls.get_close_width() * scale) as i32;
                        let max_w = (controls.get_maximize_width() * scale) as i32;
                        let min_w = (controls.get_minimize_width() * scale) as i32;

                        let btn_top = rect.top;
                        let btn_bottom = rect.top + title_h;

                        if y >= btn_top && y < btn_bottom {
                            // 从右向左依次判断各按钮热区，跳过所有 show-* = false 的按钮
                            // Detect button hit zones right-to-left, skipping any button with show-* = false

                            // 1. 关闭按钮
                            // 1. Close button
                            if controls.get_show_close() {
                                let close_left = rect.right - close_w;
                                if x >= close_left && x < rect.right {
                                    return LRESULT(HTCLOSE as isize);
                                }
                            }

                            // 2. 计算关闭按钮占用的实际宽度（不显示则为 0）
                            // 2. Effective width occupied by close button (0 if hidden)
                            let effective_close_w = if controls.get_show_close() { close_w } else { 0 };

                            // 3. 最大化按钮
                            // 3. Maximize button
                            if controls.get_show_maximize() {
                                let max_right = rect.right - effective_close_w;
                                let max_left = max_right - max_w;
                                if x >= max_left && x < max_right {
                                    return LRESULT(HTMAXBUTTON as isize);
                                }
                            }

                            // 4. 计算前两个按钮占用的实际宽度
                            // 4. Effective width of the two preceding buttons
                            let effective_max_w = if controls.get_show_maximize() { max_w } else { 0 };

                            // 5. 最小化按钮
                            // 5. Minimize button
                            if controls.get_show_minimize() {
                                let min_right = rect.right - effective_close_w - effective_max_w;
                                let min_left = min_right - min_w;
                                if x >= min_left && x < min_right {
                                    return LRESULT(HTMINBUTTON as isize);
                                }
                            }
                        }
                    }
                }

                // 按钮未命中后，非最大化时判定窗口边缘拉伸区域
                // After button miss, determine window edge resizing area when not maximized
                if !is_zoomed {
                    let border_width = Self::get_resize_border_width(hwnd);
                    let left = x - rect.left < border_width;
                    let right = rect.right - x <= border_width;
                    let top = y - rect.top < border_width;
                    let bottom = rect.bottom - y <= border_width;

                    if top || bottom || left || right {
                        // 根据鼠标位置返回对应的边缘/角点命中测试结果
                        // Return corresponding edge/corner hit test result based on mouse position
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
                // Default return client area, allowing window content to receive mouse events
                LRESULT(HTCLIENT as isize)
            }

            // 非客户区鼠标移动：实时更新按钮悬停与按下状态
            // Non-client area mouse move: update button hover and pressed states in real-time
            WM_NCMOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };

                    let hit = wparam.0 as usize;
                    let pressed = state
                        .pressed_hit
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();

                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();

                        if let Some(pressed_code) = pressed {
                            // 按住某个按钮拖拽场景 (Dragging while button held down)
                            if hit == pressed_code {
                                // 鼠标留在/移回当初按下的按钮：恢复该按钮的 pressed 状态，取消所有 hover
                                let is_min = pressed_code == HTMINBUTTON as usize;
                                let is_max = pressed_code == HTMAXBUTTON as usize;
                                let is_close = pressed_code == HTCLOSE as usize;

                                if controls.get_min_pressed() != is_min { controls.set_min_pressed(is_min); }
                                if controls.get_max_pressed() != is_max { controls.set_max_pressed(is_max); }
                                if controls.get_close_pressed() != is_close { controls.set_close_pressed(is_close); }

                                if controls.get_min_hover() { controls.set_min_hover(false); }
                                if controls.get_max_hover() { controls.set_max_hover(false); }
                                if controls.get_close_hover() { controls.set_close_hover(false); }
                            } else {
                                // 鼠标移出按下的按钮（无论移到其他按钮还是空白处）：
                                // 按下的按钮取消 pressed 视觉，其他按钮也不显示 hover（符合 Win11 原生行为）
                                if controls.get_min_pressed() { controls.set_min_pressed(false); }
                                if controls.get_max_pressed() { controls.set_max_pressed(false); }
                                if controls.get_close_pressed() { controls.set_close_pressed(false); }

                                if controls.get_min_hover() { controls.set_min_hover(false); }
                                if controls.get_max_hover() { controls.set_max_hover(false); }
                                if controls.get_close_hover() { controls.set_close_hover(false); }
                            }
                        } else {
                            // 正常悬停（无按钮被按住）：按 wparam 命中测试结果更新 hover
                            let is_min = hit == HTMINBUTTON as usize;
                            let is_max = hit == HTMAXBUTTON as usize;
                            let is_close = hit == HTCLOSE as usize;

                            if controls.get_min_hover() != is_min { controls.set_min_hover(is_min); }
                            if controls.get_max_hover() != is_max { controls.set_max_hover(is_max); }
                            if controls.get_close_hover() != is_close { controls.set_close_hover(is_close); }

                            if controls.get_min_pressed() { controls.set_min_pressed(false); }
                            if controls.get_max_pressed() { controls.set_max_pressed(false); }
                            if controls.get_close_pressed() { controls.set_close_pressed(false); }
                        }
                    }
                }
                // Bug 3 修复：WM_NCMOUSELEAVE 是一次性通知，每次 WM_NCMOUSEMOVE 都必须重新订阅，
                // 否则系统永远不会发送 WM_NCMOUSELEAVE，hover 状态将永久卡住。
                // Bug 3 Fix: WM_NCMOUSELEAVE is a one-time notification, must re-subscribe on each WM_NCMOUSEMOVE,
                // otherwise the system will never send WM_NCMOUSELEAVE, and the hover state will be permanently stuck.
                let mut tme = TRACKMOUSEEVENT {
                    cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TRACKMOUSEEVENT_FLAGS(TME_NONCLIENT.0 | TME_LEAVE.0),
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe { let _ = TrackMouseEvent(&mut tme); }

                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 鼠标离开非客户区或进入客户区：若无按住事件则清理 hover，若有按住事件则保持 pressed_hit
            // Mouse leaves non-client area or enters client area: reset hover and pressed states if not dragging
            WM_NCMOUSELEAVE | WM_MOUSEMOVE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        if controls.get_min_hover() { controls.set_min_hover(false); }
                        if controls.get_max_hover() { controls.set_max_hover(false); }
                        if controls.get_close_hover() { controls.set_close_hover(false); }
                        if controls.get_min_pressed() { controls.set_min_pressed(false); }
                        if controls.get_max_pressed() { controls.set_max_pressed(false); }
                        if controls.get_close_pressed() { controls.set_close_pressed(false); }
                    }
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 拦截标题栏按钮的按下消息：记录按下区域，阻止默认视觉效果，同时驱动 Slint Pressed 状态
            // Intercept titlebar button press: record press area, prevent default visuals, drive Slint pressed state
            WM_NCLBUTTONDOWN
                if wparam.0 == HTMINBUTTON as usize
                    || wparam.0 == HTMAXBUTTON as usize
                    || wparam.0 == HTCLOSE as usize =>
            {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let hit = wparam.0;
                    // 记录按下区域（用于松手区域一致性校验）
                    // Record press area (for release area consistency check)
                    *state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()) = Some(hit);

                    // 通知 Slint UI 进入按下状态
                    // Notify Slint UI to enter pressed state
                    let weak = state.weak.clone();
                    let _ = weak.upgrade_in_event_loop(move |app| {
                        let controls = app.global::<crate::WindowControls>();
                        controls.set_min_pressed(hit == HTMINBUTTON as usize);
                        controls.set_max_pressed(hit == HTMAXBUTTON as usize);
                        controls.set_close_pressed(hit == HTCLOSE as usize);
                        controls.set_min_hover(false);
                        controls.set_max_hover(false);
                        controls.set_close_hover(false);
                    });
                }
                LRESULT(0)
            }

            // 处理标题栏按钮的释放消息：清除 pressed 状态，并仅当按下与松开在同一按钮时才执行操作
            // Handle titlebar button release: clear pressed state, execute action only if press+release on same button
            WM_NCLBUTTONUP | WM_LBUTTONUP => {
                let hit = wparam.0 as usize;
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    let pressed = state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()).take();

                    // 先清除 Slint 中的 pressed 状态并按 release hit 更新 hover 状态
                    // Clear pressed state in Slint and update hover per release hit
                    let weak = state.weak.clone();
                    let _ = weak.upgrade_in_event_loop(move |app| {
                        let controls = app.global::<crate::WindowControls>();
                        controls.set_min_pressed(false);
                        controls.set_max_pressed(false);
                        controls.set_close_pressed(false);

                        controls.set_min_hover(hit == HTMINBUTTON as usize);
                        controls.set_max_hover(hit == HTMAXBUTTON as usize);
                        controls.set_close_hover(hit == HTCLOSE as usize);
                    });

                    // 按下与松开在同一按钮时，触发操作
                    // Execute action only when press and release are on the same button
                    if let Some(pressed_hit) = pressed {
                        if pressed_hit == hit {
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

            // 取消模式（如弹出模态框或切屏）：清理 pressed 和 hover 状态
            // Cancel mode (e.g. modal popups or window switch): clean up pressed and hover states
            WM_CANCELMODE => {
                if !state_ptr.is_null() {
                    let state = unsafe { &*state_ptr };
                    *state.pressed_hit.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    if let Some(app) = state.weak.upgrade() {
                        let controls = app.global::<crate::WindowControls>();
                        controls.set_min_hover(false);
                        controls.set_max_hover(false);
                        controls.set_close_hover(false);
                        controls.set_min_pressed(false);
                        controls.set_max_pressed(false);
                        controls.set_close_pressed(false);
                    }
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 其他未处理的消息：调用默认的子类过程
            // Other unhandled messages: call default subclass procedure
            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}

/// 为 Slint 组件提供无边框窗口设置的 trait
/// Trait that provides borderless window setup for Slint components
pub trait TitlebarSetup {
    fn setup_borderless(&self) -> Result<WindowFrame, slint::PlatformError>;
}

/// 在 slint::Weak<AppWindow> 上实现无边框窗口设置
/// Implements borderless window setup on slint::Weak<AppWindow>
impl TitlebarSetup for slint::Weak<AppWindow> {
    fn setup_borderless(&self) -> Result<WindowFrame, slint::PlatformError> {
        // 尝试升级弱引用
        // Try to upgrade the weak reference
        let component = self.upgrade().ok_or_else(|| {
            slint::PlatformError::Other("Failed to upgrade component handle".to_string())
        })?;
        // 创建窗口框架实例
        // Create window frame instance
        let frame = WindowFrame::new(&component);

        // 在事件循环中应用无边框样式
        // Apply borderless style in the event loop
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
