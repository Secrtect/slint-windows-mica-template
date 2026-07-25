use crate::borderless::WindowFrame;
use crate::{AppWindow, WindowControls};
use slint::ComponentHandle;

/// 绑定无边框窗口控件（关闭、最小化、最大化、拖拽等）
/// Bind borderless window controls (close, minimize, maximize, drag, etc.)
pub fn setup_window_controls(app: &AppWindow, frame: WindowFrame) {
    let controls = app.global::<WindowControls>();

    // 1. 初始化 Slint 中的 maximized 状态
    // 1. Initialize maximized state in Slint
    controls.set_maximized(false);

    // 2. 监听底层的最大化/还原状态改变（如拖拽标题栏还原、Win+方向键等）
    // 2. Listen for underlying maximize/restore state changes (e.g., drag titlebar to restore, Win+arrow keys)
    frame.on_maximized_changed(|app, is_max| {
        app.global::<WindowControls>().set_maximized(is_max);
    });

    // 3. 绑定最大化 / 还原按钮
    // 3. Bind maximize/restore button
    let frame_maximize = frame.clone();
    controls.on_maximize(move || {
        frame_maximize.toggle_maximized();
    });

    // 4. 绑定双击标题栏
    // 4. Bind titlebar double-click
    let frame_dblclick = frame.clone();
    controls.on_double_click(move || {
        frame_dblclick.toggle_maximized();
    });

    // 5. 绑定关闭按钮
    // 5. Bind close button
    let frame_close = frame.clone();
    controls.on_close(move || {
        frame_close.close();
    });

    // 6. 绑定拖拽
    // 6. Bind drag
    let frame_drag = frame.clone();
    controls.on_drag(move || {
        let _ = frame_drag.drag();
    });

    // 7. 绑定最小化按钮
    // 7. Bind minimize button
    let frame_minimize = frame.clone();
    controls.on_minimize(move || {
        frame_minimize.minimize();
    });
}
