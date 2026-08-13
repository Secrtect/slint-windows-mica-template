#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

mod app_window;
mod cbt_hook;
mod display;
mod effects;

use app_window::borderless::TitlebarSetup;
use app_window::controls;
use app_window::attributes;

// 引入自动生成的 UI 模块
// Include auto-generated UI modules
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // 1. 安装 CBT Hook（在窗口创建前注入 DWM 属性，杜绝闪烁）
    //    注意：Slint 的真正 UI 窗口不在 AppWindow::new() 时创建，
    //    而是在 app.run() 内部调用 show() 时才惰性创建。
    //    因此 Hook guard 必须保持存活到 run() 执行期间。
    //    Hook 在回调中捕获到 UI 窗口后会自卸载，guard 的 Drop 只是无害的清理。
    //
    // 1. Install CBT Hook (inject DWM attributes before window creation to prevent flickering)
    //    Note: Slint's actual UI window is NOT created during AppWindow::new(),
    //    but lazily during show() inside app.run().
    //    So the hook guard must stay alive through run().
    //    The hook self-uninstalls after capturing the UI window; guard's Drop is just cleanup.
    let hook_installed = cbt_hook::CbtHookGuard::install(|hwnd| {
        // 启动时读取系统当前主题色（浅色/深色）
        // Read the current system theme (light/dark) at startup
        let is_dark = attributes::is_system_dark_mode();

        // CBT Hook 捕获到 HWND 后，通过闭包注入 DWM 属性
        // When CBT Hook captures the HWND, inject DWM attributes via closure
        attributes::DwmPreset::new()
            .with_mica()
            .with_dark_mode(is_dark)
            .apply(hwnd);
    });
    let hook_ok = hook_installed.is_ok();
    // _hook_guard 必须用 _ 前缀保留到 main() 结束，不能提前 drop！
    // _hook_guard must be kept alive until main() ends with _ prefix, do NOT drop early!
    let _hook_guard = hook_installed.ok();

    if hook_ok {
        println!("[Main] CBT Hook 已安装，等待 run() 期间捕获 UI 窗口");
        // CBT Hook installed, waiting for UI window capture during run()
    } else {
        println!("[Main] CBT Hook 安装失败，将通过 event loop 路径 fallback");
        // CBT Hook installation failed, will fall back via event loop
    }

    // 2. 创建 App 实例（此时不会创建真正的 UI 窗口）
    // 2. Create App instance (the actual UI window is NOT created here)
    let app = AppWindow::new()?;

    // 3. 启动无边框机制（不受 Hook 影响）
    // 3. Start borderless mechanism (unaffected by the hook)
    let frame = app.as_weak().setup_borderless().expect("无边框初始化失败");

    // 4. 定位窗口到鼠标所在屏幕并居中（含防超屏处理）
    // 4. Position window to the monitor where the mouse is located and center it (with overflow prevention)
    display::center_on_active_monitor(&app);

    // 5. 应用 Mica 视觉特效
    //    如果 Hook 已安装：Hook 会在 run() -> show() 期间通过 HCBT_CREATEWND 注入 DWM Mica，
    //    这里只需在 event loop 中设置 UI 透明标志位。
    //    如果 Hook 未安装：走原有的 window_vibrancy fallback 路径。
    //
    // 5. Apply Mica visual effects
    //    If hook is installed: it will inject DWM Mica via HCBT_CREATEWND during run() -> show(),
    //    we only need to set the UI transparency flag in the event loop.
    //    If hook is not installed: use the original window_vibrancy fallback path.
    effects::apply_mica_effect(&app, hook_ok);

    // 6. 绑定窗口控制回调（按需调整三个按钮的可见性）
    // 6. Bind window control callbacks (adjust button visibility as needed)
    controls::setup_window_controls(&app, frame, controls::TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    });

    // 7. 运行主循环
    //    show() 在此期间被调用 -> CreateWindowExW 触发 HCBT_CREATEWND ->
    //    Hook 捕获 "Window Class" 窗口 -> 执行闭包注入 DWM 属性 -> 自卸载
    //
    // 7. Run main loop
    //    show() is called during this -> CreateWindowExW triggers HCBT_CREATEWND ->
    //    Hook captures "Window Class" window -> executes closure to inject DWM attributes -> self-uninstalls
    app.run()?;

    Ok(())
}
