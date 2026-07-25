// 临时测试文件 test_skia.rs
// Temporary test file test_skia.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AppWindow::new()?;

    // 什么都不做，直接运行
    // Do nothing, run directly
    app.run()?;
    Ok(())
}
