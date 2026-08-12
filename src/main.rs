#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

mod borderless;
mod cbt_hook;
mod controls;
mod display;
mod effects;

use borderless::TitlebarSetup;

// 引入自动生成的 UI 模块
// Include auto-generated UI modules
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // 1. 安装 CBT Hook（在窗口创建前注入 DWM 属性，杜绝闪烁）
    // 1. Install CBT Hook (inject DWM attributes before window creation to prevent flickering)
    let dwm_preset = cbt_hook::DwmPreset::new()
        .with_mica()
        .with_dark_mode(true);
    let hook_guard = cbt_hook::CbtHookGuard::install(dwm_preset);
    let hook_ok = hook_guard.is_ok();
    // 保留 guard 直到 AppWindow 创建完成后再 drop
    // Keep guard alive until after AppWindow creation, then drop
    let hook_guard = hook_guard.ok();

    // 2. 创建 App 实例（内部会触发 HCBT_CREATEWND，Hook 自动注入 DWM 属性并卸载）
    // 2. Create App instance (triggers HCBT_CREATEWND internally, hook auto-injects and uninstalls)
    let app = AppWindow::new()?;

    // 3. 检查 Hook 是否成功注入了 Mica，然后释放 guard
    // 3. Check if the hook successfully injected Mica, then release the guard
    let mica_injected = hook_guard.as_ref().map(|g| g.was_applied()).unwrap_or(false);
    drop(hook_guard);

    if hook_ok {
        if mica_injected {
            println!("[Main] CBT Hook 已成功注入 DWM 属性");
            // CBT Hook successfully injected DWM attributes
        } else {
            println!("[Main] CBT Hook 已安装但未捕获到窗口，将通过 event loop 路径 fallback");
            // CBT Hook installed but no window captured, will fall back via event loop
        }
    } else {
        println!("[Main] CBT Hook 安装失败，将通过 event loop 路径 fallback");
        // CBT Hook installation failed, will fall back via event loop
    }

    // 4. 启动无边框机制（不受 Hook 影响）
    // 4. Start borderless mechanism (unaffected by the hook)
    let frame = app.as_weak().setup_borderless().expect("无边框初始化失败");

    // 5. 定位窗口到鼠标所在屏幕并居中（含防超屏处理）
    // 5. Position window to the monitor where the mouse is located and center it (with overflow prevention)
    display::center_on_active_monitor(&app);

    // 6. 应用 Mica 视觉特效（如果 Hook 已注入则跳过 DWM 调用，否则 fallback）
    // 6. Apply Mica visual effects (skip DWM call if hook already injected, otherwise fallback)
    effects::apply_mica_effect(&app, mica_injected);

    // 7. 绑定窗口控制回调（按需调整三个按钮的可见性）
    // 7. Bind window control callbacks (adjust button visibility as needed)
    controls::setup_window_controls(&app, frame, controls::TitlebarButtons {
        show_minimize: true,
        show_maximize: true,
        show_close: true,
    });

    // 8. 运行主循环
    // 8. Run main loop
    app.run()?;

    Ok(())
}
