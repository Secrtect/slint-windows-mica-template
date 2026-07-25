use crate::AppWindow;
use slint::ComponentHandle;
/// 尝试应用 Windows 11 Mica 透明效果
/// Try to apply Windows 11 Mica transparency effect
pub fn apply_mica_effect(app: &AppWindow) {
    #[cfg(target_os = "windows")]
    {
        let app_weak = app.as_weak();
        slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak.upgrade() {
                let handle = app.window().window_handle();

                if let Err(e) = window_vibrancy::apply_mica(&handle, None) {
                    println!("应用 Mica 失败: {:?}，已降级为系统自适应纯色背景", e);
                    // Mica application failed, falling back to system adaptive solid background
                } else {
                    app.set_is_mica_active(true);
                    println!("成功应用 Mica 效果");
                    // Mica effect applied successfully
                }
            }
        })
        .expect("Failed to queue event loop initialization");
    }
}
