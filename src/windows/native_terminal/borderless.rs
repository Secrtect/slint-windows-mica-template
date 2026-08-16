//! DWM 原生按钮子类化模块
//!
//! 为 native_terminal_window 提供简化子类化，使用 DWM 系统原生绘制
//! 标题栏按钮（最小化/最大化/关闭），Slint 仅负责拖拽区渲染。
//!
//! DWM native button subclassing module.
//! Provides simplified subclassing for native_terminal_window,
//! using DWM system-native titlebar buttons. Slint only renders the drag area.

use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use slint::Window;
use std::mem::size_of;
use tracing::warn;
use ::windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use ::windows::Win32::Graphics::Dwm::{
    DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
    DWMWA_CAPTION_BUTTON_BOUNDS,
};
use ::windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use ::windows::Win32::UI::Controls::MARGINS;
use ::windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use ::windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use ::windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetWindowRect, HTBOTTOM, HTBOTTOMLEFT, HTBOTTOMRIGHT, HTCAPTION, HTCLIENT,
    HTCLOSE, HTLEFT, HTMAXBUTTON, HTMINBUTTON, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, IsZoomed,
    NCCALCSIZE_PARAMS, SM_CXPADDEDBORDER, SM_CXSIZEFRAME, SetWindowPos, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_NCCALCSIZE, WM_NCDESTROY,
    WM_NCHITTEST, WM_NCMOUSELEAVE, WM_NCMOUSEMOVE,
};
use crate::NativeTerminalWindow;

/// 标题栏高度（逻辑像素，与 Slint titlebar.slint 中的 height: 36px 一致）
/// Titlebar height in logical pixels
const TITLEBAR_HEIGHT: i32 = 36;

/// 根据 DPI 获取窗口拉伸边框宽度
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

/// 获取窗口所在显示器的工作区矩形
fn get_monitor_work_area(hwnd: HWND) -> Option<RECT> {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            Some(monitor_info.rcWork)
        } else {
            None
        }
    }
}

/// DWM 原生按钮窗口框架
///
/// 安装简化子类化 proc，核心逻辑：
/// 1. WM_NCCALCSIZE — 扩展客户区覆盖标题栏，DWM 在客户区之上绘制原生按钮
/// 2. WM_NCHITTEST — 标题栏区域返回 HTCAPTION，边缘区域返回拉伸码，按钮区域精确透传/兜底
/// 3. 1px 阴影修复 — DwmExtendFrameIntoClientArea
pub struct NativeCaptionFrame {
    weak: slint::Weak<NativeTerminalWindow>,
}

impl Clone for NativeCaptionFrame {
    fn clone(&self) -> Self {
        Self {
            weak: self.weak.clone(),
        }
    }
}

impl NativeCaptionFrame {
    const SUBCLASS_ID: usize = 2001;

    pub fn new(component: &NativeTerminalWindow) -> Self {
        Self {
            weak: component.as_weak(),
        }
    }

    fn with_window<R>(&self, f: impl FnOnce(&Window) -> R) -> Option<R> {
        self.weak.upgrade().map(|c| f(c.window()))
    }

    /// 开始拖拽窗口
    pub fn drag(&self) {
        self.with_winit_window(|window| {
            let _ = window.drag_window();
        });
    }

    /// 切换最大化/还原
    pub fn toggle_maximized(&self) {
        self.with_window(|w| w.set_maximized(!w.is_maximized()));
    }

    fn with_winit_window<R>(&self, f: impl FnOnce(&winit::window::Window) -> R) -> Option<R> {
        self.weak
            .upgrade()
            .and_then(|c| c.window().with_winit_window(|w| f(w)))
    }

    /// 通过 HWND 安装子类化 + 阴影（用于 CBT Hook 场景）
    /// Install subclassing + shadow via HWND (for CBT Hook scenarios)
    pub fn apply_to_hwnd(&self, hwnd: HWND) {
        // 1. 将 DWM 材质帧扩展至整个客户区，让 DWM 有空间绘制原生按钮和背景材质
        //    Extend DWM frame into entire client area for native button rendering and backdrop
        {
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            unsafe {
                if let Err(e) = DwmExtendFrameIntoClientArea(hwnd, &margins) {
                    warn!("DwmExtendFrameIntoClientArea failed: {e}");
                }
            }
        }

        // 2. 安装简化子类化 proc（仅处理 WM_NCCALCSIZE / WM_NCHITTEST / WM_NCDESTROY）
        let ref_data = self as *const Self as usize;
        unsafe {
            if !SetWindowSubclass(
                hwnd,
                Some(Self::native_caption_proc),
                Self::SUBCLASS_ID,
                ref_data,
            )
            .as_bool()
            {
                warn!("SetWindowSubclass (native caption) failed");
            }
        }

        // 3. 触发 SWP_FRAMECHANGED 通知 DWM 刷新
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

        // 4. 诊断：DWM 是否分配了按钮空间？
        //    Diagnostic: did DWM allocate button space?
        {
            let mut btn_rect = RECT::default();
            let btn_result = unsafe {
                DwmGetWindowAttribute(
                    hwnd,
                    DWMWA_CAPTION_BUTTON_BOUNDS,
                    &mut btn_rect as *mut _ as _,
                    size_of::<RECT>() as u32,
                )
            };
            if btn_result.is_ok() && btn_rect.left > 0 {
                println!("[NativeCaption] CAPTION_BUTTON_BOUNDS = ({}, {}, {}, {}) — DWM 已分配按钮!",
                    btn_rect.left, btn_rect.top, btn_rect.right, btn_rect.bottom);
            } else {
                println!("[NativeCaption] CAPTION_BUTTON_BOUNDS = 空 — DWM 未分配按钮空间!");
            }
        }
    }

    /// 简化子类化窗口过程：处理 WM_NCCALCSIZE（扩展客户区）和 WM_NCHITTEST（标题栏与原生按钮）
    /// Simplified subclassing proc: handles WM_NCCALCSIZE (extend client area)
    /// and WM_NCHITTEST (titlebar and native buttons)
    unsafe extern "system" fn native_caption_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        uid_subclass: usize,
        _ref_data: usize,
    ) -> LRESULT {
        // 先让 DWM 处理 hit-test 及按钮交互消息（关键！否则 DWM 不会绘制/响应原生按钮）
        // Let DWM process hit-test and button interaction messages first
        {
            let mut dwm_result = LRESULT(0);
            if unsafe { DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut dwm_result) }.as_bool() {
                return dwm_result;
            }
        }

        match msg {
            // 非客户区大小计算：扩展客户区覆盖标题栏，DWM 原生按钮悬浮在客户区之上
            // NCCALCSIZE: extend client area to cover titlebar, DWM native buttons float on top
            WM_NCCALCSIZE if wparam.0 != 0 => {
                let params = unsafe { &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS) };
                let original_top = params.rgrc[0].top;

                // 无论是窗口化还是最大化，均先调用 DefWindowProcW 让 Windows/DWM 计算标准非客户区布局
                let _ = unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };

                // 统一将 top 恢复至 original_top，使客户区延伸至窗口最顶部（offset=0）
                // 这样 DWM 能识别到标题栏扩展，从而在窗口化和最大化下均原生接管按钮 hit-test 与悬停高亮动画
                params.rgrc[0].top = original_top;
                LRESULT(0)
            }

            // 非客户区命中测试：DWM 按钮优先由 DwmDefWindowProc 处理；
            // 兜底逻辑按屏幕物理坐标精准计算三大按钮区域，确保最大化时悬停与点击体验与原生窗口完全一致；
            // 窗口化时提供 8 方向拉伸检测（包含最上方边缘 HTTOP/HTTOPLEFT/HTTOPRIGHT）；标题栏其余区域返回 HTCAPTION。
            WM_NCHITTEST => {
                let x = (lparam.0 & 0xFFFF) as i16 as i32;
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let is_zoomed = unsafe { IsZoomed(hwnd) }.as_bool();

                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return LRESULT(HTCLIENT as isize);
                }

                // 确定标题栏顶部及右侧边界（基于屏幕坐标）
                let (btn_top, btn_bottom, btn_right) = if is_zoomed {
                    let work_area = get_monitor_work_area(hwnd).unwrap_or(rect);
                    (work_area.top, work_area.top + TITLEBAR_HEIGHT, work_area.right)
                } else {
                    (rect.top, rect.top + TITLEBAR_HEIGHT, rect.right)
                };

                // DWMWA_CAPTION_BUTTON_BOUNDS 获取按钮总宽度（若获取失败则使用 140px 标准宽度）
                let mut btn_rect = RECT::default();
                let btn_area_w = if unsafe {
                    DwmGetWindowAttribute(
                        hwnd,
                        DWMWA_CAPTION_BUTTON_BOUNDS,
                        &mut btn_rect as *mut _ as _,
                        size_of::<RECT>() as u32,
                    )
                }
                .is_ok() && btn_rect.right > btn_rect.left
                {
                    btn_rect.right - btn_rect.left
                } else {
                    140
                };

                let btn_left = btn_right - btn_area_w;

                // 1. 优先判定三大原生按钮区域（Close / Maximize / Minimize）
                if y >= btn_top && y < btn_bottom && x >= btn_left && x <= btn_right {
                    let single_btn_w = (btn_area_w / 3).max(1);
                    let dist_from_right = btn_right - x;
                    if dist_from_right < single_btn_w {
                        return LRESULT(HTCLOSE as isize);
                    } else if dist_from_right < single_btn_w * 2 {
                        return LRESULT(HTMAXBUTTON as isize);
                    } else {
                        return LRESULT(HTMINBUTTON as isize);
                    }
                }

                // 2. 窗口边缘 8 个方向拉伸判定（仅窗口化状态生效）
                if !is_zoomed {
                    let border_width = get_resize_border_width(hwnd);
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

                // 3. 标题栏区域：返回 HTCAPTION 允许拖拽和双击最大化/还原
                if y >= btn_top && y < btn_bottom && x < btn_left {
                    return LRESULT(HTCAPTION as isize);
                }

                LRESULT(HTCLIENT as isize)
            }

            // 非客户区鼠标移动：注册 TrackMouseEvent 开启鼠标离开监听，确保按钮悬停高亮能正确刷新与清除
            WM_NCMOUSEMOVE => {
                let mut tme = windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT {
                    cbSize: size_of::<windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT>() as u32,
                    dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::TRACKMOUSEEVENT_FLAGS(
                        windows::Win32::UI::Input::KeyboardAndMouse::TME_NONCLIENT.0
                            | windows::Win32::UI::Input::KeyboardAndMouse::TME_LEAVE.0,
                    ),
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::TrackMouseEvent(&mut tme);
                }
                unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
            }

            // 鼠标离开非客户区：通知 DWM 与系统底层清理按钮高亮状态
            WM_NCMOUSELEAVE => unsafe {
                let mut dwm_result = LRESULT(0);
                let _ = DwmDefWindowProc(hwnd, msg, wparam, lparam, &mut dwm_result);
                DefSubclassProc(hwnd, msg, wparam, lparam)
            },

            // 窗口销毁时注销子类化
            WM_NCDESTROY => {
                println!("[NativeCaption] WM_NCDESTROY — 注销子类化");
                unsafe {
                    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                    let _ = RemoveWindowSubclass(hwnd, Some(Self::native_caption_proc), uid_subclass);
                    result
                }
            }

            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }
}