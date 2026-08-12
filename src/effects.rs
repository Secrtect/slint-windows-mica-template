use crate::AppWindow;
use slint::ComponentHandle;
/// 尝试应用 Windows 11 Mica 透明效果
/// 如果 CBT Hook 已在窗口创建时注入了 Mica，则仅设置 UI 标志位而不再重复调用 DWM API。
///
/// Try to apply Windows 11 Mica transparency effect.
/// If the CBT Hook has already injected Mica at window creation, only set the UI flag
/// without redundantly calling the DWM API.
pub fn apply_mica_effect(app: &AppWindow, already_injected: bool) {
    #[cfg(target_os = "windows")]
    {
        if already_injected {
            // CBT Hook 已在窗口创建时注入了 Mica，只需激活 UI 侧的透明背景开关
            // CBT Hook has already injected Mica at creation, just activate the UI transparency flag
            app.set_is_mica_active(true);
            println!("Mica 已由 CBT Hook 预注入，跳过 event loop 路径");
            // Mica already pre-injected by CBT Hook, skipping event loop path
            return;
        }

        // Fallback 路径：CBT Hook 未使用或失败，走原有的 event loop 注入
        // Fallback path: CBT Hook not used or failed, use the original event loop injection
        let app_weak = app.as_weak();
        slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak.upgrade() {
                let handle = app.window().window_handle();

                if let Err(e) = window_vibrancy::apply_mica(&handle, None) {
                    println!("应用 Mica 失败: {:?}，已降级为系统自适应纯色背景", e);
                    // Mica application failed, falling back to system adaptive solid background
                } else {
                    app.set_is_mica_active(true);
                    println!("成功应用 Mica 效果（通过 event loop fallback 路径）");
                    // Mica effect applied successfully (via event loop fallback path)
                }
            }
        })
        .expect("Failed to queue event loop initialization");
    }
}
