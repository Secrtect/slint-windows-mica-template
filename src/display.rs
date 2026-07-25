use crate::AppWindow;
use slint::ComponentHandle;
use windows_sys::Win32::Foundation::{POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForSystem, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// 根据当前鼠标所在屏幕计算 DPI 并调整窗口居中定位，防止超屏
/// Calculate DPI based on the monitor where the mouse is located and center the window, preventing overflow
pub fn center_on_active_monitor(app: &AppWindow) {
    let (target_x_phys, target_y_phys, target_w_logical, target_h_logical, is_resized) = unsafe {
        // 1. 获取当前鼠标物理位置
        // 1. Get current mouse physical position
        let mut cursor_pos = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos);

        // 2. 获取目标显示器句柄
        // 2. Get target monitor handle
        let h_monitor = MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST);

        // 3. 获取工作区 RECT
        // 3. Get workspace RECT
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
        // 4. Get monitor DPI
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
        // 5. Calculate logical workspace
        let work_w_logical = work_w_phys / scale;
        let work_h_logical = work_h_phys / scale;

        // 6. 获取 UI 设置的初始逻辑尺寸
        // 6. Get initial logical size set by UI
        let init_w_logical = app.get_init_width();
        let init_h_logical = app.get_init_height();

        // 7. 防超屏截断
        // 7. Prevent overflow truncation
        let target_w_logical = init_w_logical.min(work_w_logical);
        let target_h_logical = init_h_logical.min(work_h_logical);
        let is_resized = target_w_logical < init_w_logical || target_h_logical < init_h_logical;

        // 8. 计算居中物理坐标
        // 8. Calculate centered physical coordinates
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
    // Apply position and size
    app.window()
        .set_position(slint::PhysicalPosition::new(target_x_phys, target_y_phys));

    if is_resized {
        app.window()
            .set_size(slint::LogicalSize::new(target_w_logical, target_h_logical));
    }
}
