use crate::borderless::WindowFrame;
use crate::{AppWindow, WindowControls};
use slint::ComponentHandle;

/// 标题栏按钮可见性配置（用户在 main.rs 里修改这里即可）
/// Titlebar button visibility configuration (users only need to modify this in main.rs)
pub struct TitlebarButtons {
    pub show_minimize: bool,
    pub show_maximize: bool,
    pub show_close: bool,
}

impl Default for TitlebarButtons {
    fn default() -> Self {
        Self {
            show_minimize: true,
            show_maximize: true,
            show_close: true,
        }
    }
}

/// 绑定无边框窗口控件（关闭、最小化、最大化、拖拽等）
/// Bind borderless window controls (close, minimize, maximize, drag, etc.)
pub fn setup_window_controls(app: &AppWindow, frame: WindowFrame, buttons: TitlebarButtons) {
    let controls = app.global::<WindowControls>();

    // 1. 初始化按钮可见性
    // 1. Initialize button visibility
    controls.set_show_minimize(buttons.show_minimize);
    controls.set_show_maximize(buttons.show_maximize);
    controls.set_show_close(buttons.show_close);

    // 2. 初始化 Slint 中的 maximized 状态
    // 2. Initialize maximized state in Slint
    controls.set_maximized(false);

    // 3. 监听底层的最大化/还原状态改变（如拖拽标题栏还原、Win+方向键等）
    // 3. Listen for underlying maximize/restore state changes (e.g., drag titlebar to restore, Win+arrow keys)
    frame.on_maximized_changed(|app, is_max| {
        app.global::<WindowControls>().set_maximized(is_max);
    });

    // 4. 绑定最大化 / 还原按钮
    // 4. Bind maximize/restore button
    let frame_maximize = frame.clone();
    controls.on_maximize(move || {
        frame_maximize.toggle_maximized();
    });

    // 5. 绑定双击标题栏
    // 5. Bind titlebar double-click
    let frame_dblclick = frame.clone();
    controls.on_double_click(move || {
        frame_dblclick.toggle_maximized();
    });

    // 6. 绑定关闭按钮
    // 6. Bind close button
    let frame_close = frame.clone();
    controls.on_close(move || {
        frame_close.close();
    });

    // 7. 绑定拖拽
    // 7. Bind drag
    let frame_drag = frame.clone();
    controls.on_drag(move || {
        let _ = frame_drag.drag();
    });

    // 8. 绑定最小化按钮
    // 8. Bind minimize button
    let frame_minimize = frame.clone();
    controls.on_minimize(move || {
        frame_minimize.minimize();
    });
}
