#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

mod borderless;
mod controls;
mod display;
mod effects;

use borderless::TitlebarSetup;

// 引入自动生成的 UI 模块
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // 1. 创建 App 实例
    let app = AppWindow::new()?;

    // 2. 启动无边框机制
    let frame = app.as_weak().setup_borderless().expect("无边框初始化失败");

    // 3. 定位窗口到鼠标所在屏幕并居中（含防超屏处理）
    display::center_on_active_monitor(&app);

    // 4. 应用 Mica 视觉特效
    effects::apply_mica_effect(&app);

    // 5. 绑定窗口控制回调
    controls::setup_window_controls(&app, frame);

    // 6. 运行主循环
    app.run()?;

    Ok(())
}
