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
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmDefWindowProc, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute,
    DWMWA_CAPTION_BUTTON_BOUNDS,
};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetWindowRect, HTCAPTION, HTCLIENT, IsZoomed, NCCALCSIZE_PARAMS, SetWindowPos,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WM_NCCALCSIZE,
    WM_NCDESTROY, WM_NCHITTEST,
};
use crate::NativeTerminalWindow;

/// 标题栏高度（逻辑像素，与 Slint titlebar.slint 中的 height: 36px 一致）
/// Titlebar height in logical pixels
const TITLEBAR_HEIGHT: i32 = 36;

/// DWM 原生按钮窗口框架
///
/// 安装简化子类化 proc，核心逻辑：
/// 1. WM_NCCALCSIZE — 扩展客户区覆盖标题栏，DWM 在客户区之上绘制原生按钮
/// 2. WM_NCHITTEST — 标题栏区域返回 HTCAPTION，边缘区域返回拉伸码
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

    /// 简化子类化窗口过程：仅处理 WM_NCCALCSIZE（扩展客户区）和 WM_NCHITTEST（标题栏拖拽）
    /// Simplified subclassing proc: only handles WM_NCCALCSIZE (extend client area)
    /// and WM_NCHITTEST (titlebar drag)
    unsafe extern "system" fn native_caption_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        uid_subclass: usize,
        _ref_data: usize,
    ) -> LRESULT {
        // 首次调用时打印消息确认子类化已生效
        // One-time log to confirm subclassing is active
        {
            use std::sync::atomic::{AtomicBool, Ordering};
            static FIRST_CALL: AtomicBool = AtomicBool::new(true);
            if FIRST_CALL.swap(false, Ordering::Relaxed) {
                println!("[NativeCaption] ✅ 子类化 proc 已激活! 首条消息: 0x{:04X}", msg);
            }
        }

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
                let original_left = params.rgrc[0].left;
                let original_right = params.rgrc[0].right;
                let original_bottom = params.rgrc[0].bottom;
                println!("[NativeCaption] WM_NCCALCSIZE 触发! original rgrc[0] = ({}, {}, {}, {})",
                    original_left, original_top, original_right, original_bottom);

                if unsafe { IsZoomed(hwnd) }.as_bool() {
                    // 最大化时：使用标准 Monitor Work Area 避免任务栏遮挡
                    // Maximized: use standard monitor work area to avoid taskbar overlap
                    let monitor = unsafe {
                        windows::Win32::Graphics::Gdi::MonitorFromWindow(
                            hwnd,
                            windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST,
                        )
                    };
                    let mut monitor_info = windows::Win32::Graphics::Gdi::MONITORINFO {
                        cbSize: size_of::<windows::Win32::Graphics::Gdi::MONITORINFO>() as u32,
                        ..Default::default()
                    };
                    if unsafe {
                        windows::Win32::Graphics::Gdi::GetMonitorInfoW(monitor, &mut monitor_info)
                    }
                    .as_bool()
                    {
                        params.rgrc[0] = monitor_info.rcWork;
                        println!("[NativeCaption]    maximized → work_area = ({}, {}, {}, {})",
                            monitor_info.rcWork.left, monitor_info.rcWork.top,
                            monitor_info.rcWork.right, monitor_info.rcWork.bottom);
                    }
                } else {
                    let _ = unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
                    let after_default = params.rgrc[0];
                    println!("[NativeCaption]    after DefWindowProcW → rgrc[0] = ({}, {}, {}, {})",
                        after_default.left, after_default.top, after_default.right, after_default.bottom);
                    params.rgrc[0].top = original_top;
                    println!("[NativeCaption]    restored top → rgrc[0] = ({}, {}, {}, {})",
                        params.rgrc[0].left, params.rgrc[0].top,
                        params.rgrc[0].right, params.rgrc[0].bottom);
                }
                return LRESULT(0);
            }

            // 非客户区命中测试：DWM 按钮已在 DwmDefWindowProc 中处理，
            // DefWindowProcW 处理拉伸边框，标题栏区域返回 HTCAPTION 允许拖拽
            WM_NCHITTEST => {
                // DefWindowProcW 处理拉伸边框（left/right/bottom 非客户区由 NCCALCSIZE 保留）
                let hit = unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
                if hit != LRESULT(HTCLIENT as isize) {
                    return hit;
                }

                // 转换为窗口相对坐标（客户区已通过 NCCALCSIZE 扩展至窗口顶部）
                let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
                    return LRESULT(HTCLIENT as isize);
                }
                let cy = y - rect.top;

                // 标题栏区域：返回 HTCAPTION 允许拖拽
                // DWM 按钮的 hit-test 已由 DwmDefWindowProc 在上方处理
                if cy >= 0 && cy < TITLEBAR_HEIGHT {
                    return LRESULT(HTCAPTION as isize);
                }

                LRESULT(HTCLIENT as isize)
            }

            // 窗口销毁时注销子类化
            // Uninstall subclassing on window destroy
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