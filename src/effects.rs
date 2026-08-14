use crate::AppWindow;
use slint::ComponentHandle;

/// 尝试应用 Windows 11 Mica 透明效果
///
/// - `hook_will_handle = true`：CBT Hook 已安装，会在 `show()` 期间自动注入 DWM Mica，
///   这里只需在 event loop 中设置 UI 侧的透明背景标志位。
/// - `hook_will_handle = false`：Hook 未安装或失败，走原有的 `window_vibrancy` 路径。
///
/// Try to apply Windows 11 Mica transparency effect.
///
/// - `hook_will_handle = true`: CBT Hook is installed and will inject DWM Mica during `show()`,
///   we only need to set the UI transparency flag in the event loop.
/// - `hook_will_handle = false`: Hook not installed or failed, use the original `window_vibrancy` path.
pub fn apply_mica_effect(app: &AppWindow, hook_will_handle: bool) {
    #[cfg(target_os = "windows")]
    {
        if hook_will_handle {
            // 检查系统是否支持 Mica (Windows 11 build 22000+)
            // Check if system supports Mica
            if !crate::sys_info::is_win11() {
                println!("系统不支持 Mica (Win10或更低版本)，已降级为系统自适应纯色背景");
                // The system does not support Mica (Win10 or lower), falling back to solid background.
                return;
            }

            // CBT Hook 会在 run() -> show() 期间通过 HCBT_CREATEWND 注入 DWM Mica 属性。
            // 这里只需在 event loop 中激活 UI 侧的透明背景开关（让 Slint 背景透明以透出 DWM 材质）。
            //
            // The CBT Hook will inject DWM Mica via HCBT_CREATEWND during run() -> show().
            // Here we only need to activate the UI transparency flag in the event loop
            // (making the Slint background transparent so the DWM material shows through).
            let app_weak = app.as_weak();
            slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    app.set_is_mica_active(true);
                    println!("Mica UI 透明标志已激活（DWM 属性由 CBT Hook 注入）");
                    // Mica UI transparency flag activated (DWM attribute injected by CBT Hook)
                }
            })
            .expect("Failed to queue Mica UI flag activation");
            return;
        }

        // Fallback 路径：CBT Hook 未使用或失败，直接回退到系统自适应纯色背景
        // Fallback path: CBT Hook not used or failed, fallback to system adaptive solid background directly.
        println!("CBT Hook 注入未生效，已直接降级为系统自适应纯色背景");
    }
}
