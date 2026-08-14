//! 视觉特效模块：配置 Slint 窗口背景透明度与 DWM Mica/Acrylic 联动
//!
//! Visual effects module: configures Slint window background transparency
//! in coordination with DWM Mica/Acrylic materials.

use slint::ComponentHandle;

/// 为 Slint 窗口应用 Mica 视觉特效
///
/// - `hook_will_handle = true`：CBT Hook 已安装，已在 `show()` 瞬间注入 DWM 属性，
///   本函数只需在事件循环中激活 UI 侧的透明背景回调。
/// - `hook_will_handle = false`：CBT Hook 未安装，打印提示降级为默认背景。
pub fn apply_mica_effect<T: ComponentHandle + 'static>(
    component: &T,
    on_mica_active: impl FnOnce(&T) + Send + 'static,
    hook_will_handle: bool,
) {
    #[cfg(target_os = "windows")]
    {
        if hook_will_handle {
            // 检查系统是否支持 Mica (Windows 11 build 22000+)
            if !crate::sys_info::is_win11() {
                println!("[Effects] 系统不支持 Mica (Win10或更低版本)，已降级为系统自适应纯色背景");
                return;
            }

            let weak = component.as_weak();
            let _ = weak.upgrade_in_event_loop(move |c| {
                on_mica_active(&c);
                println!("[Effects] Mica UI 透明标志已激活（DWM 材质由 CBT Hook 注入）");
            });
            return;
        }

        println!("[Effects] CBT Hook 注入未生效，已直接降级为系统自适应纯色背景");
    }
}
