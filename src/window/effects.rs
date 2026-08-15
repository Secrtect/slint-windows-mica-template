//! 视觉特效模块：配置 Slint 窗口背景透明度与 DWM Mica/Acrylic 联动
//!
//! Visual effects module: configures Slint window background transparency
//! in coordination with DWM Mica/Acrylic materials.

use slint::ComponentHandle;

/// 为 Slint 窗口应用 Mica 视觉特效
///
/// - `hook_will_handle = true`：CBT Hook 已安装，已在 `CreateWindowExW` 瞬间注入 DWM 属性，
///   本函数同步设置 UI 侧的透明背景标志（set_is_mica_active 是纯属性赋值，无需事件循环）。
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

            // 同步设置 Slint UI 透明背景标志
            // DWM Mica 属性已在 CBT Hook HCBT_ACTIVATE 中注入，此处只需让 Slint
            // 背景透明以透出底层 Mica 材质。set_is_mica_active 是纯属性赋值，
            // 无需事件循环，直接同步调用，确保首帧即透明。
            on_mica_active(component);
            println!("[Effects] Mica UI 透明标志已激活（DWM 材质由 CBT Hook 注入）");
            return;
        }

        println!("[Effects] CBT Hook 注入未生效，已直接降级为系统自适应纯色背景");
    }
}
