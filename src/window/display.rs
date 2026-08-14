//! 显示器与窗口定位模块：根据鼠标所在显示器进行 DPI 换算并居中窗口，防止超屏。
//!
//! Display and window positioning module: calculates DPI based on monitor where
//! mouse is located, centers window, and prevents overflow.

use slint::ComponentHandle;

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{POINT, RECT};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForSystem, MDT_EFFECTIVE_DPI};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// 根据当前鼠标所在屏幕计算 DPI 并调整窗口居中定位，防止超屏
pub fn center_on_active_monitor(window: &slint::Window, init_w_logical: f32, init_h_logical: f32) {
    #[cfg(target_os = "windows")]
    {
        let (target_x_phys, target_y_phys, target_w_logical, target_h_logical, is_resized) = unsafe {
            // 1. 获取当前鼠标物理位置
            let mut cursor_pos = POINT { x: 0, y: 0 };
            GetCursorPos(&mut cursor_pos);

            // 2. 获取目标显示器句柄
            let h_monitor = MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST);

            // 3. 获取工作区 RECT
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                rcMonitor: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                rcWork: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                dwFlags: 0,
            };
            GetMonitorInfoW(h_monitor, &mut monitor_info as *mut _ as *mut _);

            // 4. 获取显示器 DPI
            let mut dpi_x: u32 = 96;
            let mut dpi_y: u32 = 96;
            if GetDpiForMonitor(h_monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) != 0 {
                dpi_x = GetDpiForSystem();
            }
            let scale = dpi_x as f32 / 96.0;

            let work_area = monitor_info.rcWork;
            let work_left_phys = work_area.left as f32;
            let work_top_phys = work_area.top as f32;
            let work_w_phys = (work_area.right - work_area.left) as f32;
            let work_h_phys = (work_area.bottom - work_area.top) as f32;

            // 5. 计算逻辑工作区
            let work_w_logical = work_w_phys / scale;
            let work_h_logical = work_h_phys / scale;

            // 6. 防超屏截断
            let target_w_logical = init_w_logical.min(work_w_logical);
            let target_h_logical = init_h_logical.min(work_h_logical);
            let is_resized = target_w_logical < init_w_logical || target_h_logical < init_h_logical;

            // 7. 计算居中物理坐标
            let target_w_phys = target_w_logical * scale;
            let target_h_phys = target_h_logical * scale;

            let target_x_phys = work_left_phys + (work_w_phys - target_w_phys) / 2.0;
            let target_y_phys = work_top_phys + (work_h_phys - target_h_phys) / 2.0;

            (
                target_x_phys as i32,
                target_y_phys as i32,
                target_w_logical,
                target_h_logical,
                is_resized,
            )
        };

        // 应用位置与尺寸
        window.set_position(slint::PhysicalPosition::new(target_x_phys, target_y_phys));

        if is_resized {
            window.set_size(slint::LogicalSize::new(target_w_logical, target_h_logical));
        }
    }
}

/// 针对实现了 `ComponentHandle` 的 Slint 组件的居中定位辅助函数
pub fn center_component_on_active_monitor<T: ComponentHandle>(
    component: &T,
    init_w_logical: f32,
    init_h_logical: f32,
) {
    center_on_active_monitor(component.window(), init_w_logical, init_h_logical);
}
