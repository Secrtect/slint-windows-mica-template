#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ::windows::Win32::Foundation::HWND;

pub mod platform;
mod sys_info;
mod windows;

// 引入自动生成的 UI 模块
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // ── 共享 Frame 持有器，用于 CBT Hook 回调中同步安装子类化 + 阴影
    //    主窗口使用 app.run() 而非显式 show()，窗口创建发生在 run() 内部。
    //    CBT Hook 在 CreateWindowExW 瞬间捕获 HWND 并通过 apply_to_hwnd 同步安装，
    //    窗口首帧绘制前一切就绪，彻底消除阴影/子类化延迟 ──
    let frame_holder: Arc<Mutex<Option<platform::WindowFrame<AppWindow>>>> =
        Arc::new(Mutex::new(None));

    // 1. 安装主窗口 CBT Hook（在 CreateWindowExW 瞬间注入 DWM 属性 + 子类化 + 阴影）
    let hook_installed = platform::CbtHookGuard::install(
        Some("Slint标准组件全家桶".to_string()),
        {
            let frame_holder = Arc::clone(&frame_holder);
            move |hwnd| {
                // DWM 属性注入（Mica、暗色模式、圆角等）
                let attrs = windows::app::attributes::get_attributes();
                attrs.apply(hwnd);

                // 子类化 + 阴影（通过 HWND 同步安装，无需等 winit 就绪）
                let hwnd_struct = HWND(hwnd as *mut std::ffi::c_void);
                if let Some(ref frame) = *frame_holder.lock().unwrap() {
                    frame.apply_to_hwnd(hwnd_struct);
                }
            }
        },
    );
    let hook_ok = hook_installed.is_ok();
    // _hook_guard 保留到 main 结束，防止提前 drop
    let _hook_guard = hook_installed.ok();

    if hook_ok {
        println!("[Main] CBT Hook 已安装，等待 run() 期间捕获 UI 窗口");
    } else {
        println!("[Main] CBT Hook 安装失败，将通过 event loop 路径 fallback");
    }

    // 2. 创建主窗口实例
    let app = AppWindow::new()?;

    // 3. 创建无边框框架（不调度 apply，由 CBT Hook 同步安装）
    let frame = windows::app::create_frame(&app);
    *frame_holder.lock().unwrap() = Some(frame.clone());

    // 4. 配置主窗口（居中定位、Mica 特效、控件绑定）
    windows::app::setup(&app, &frame, hook_ok);

    // 5. 绑定"打开自绘终端窗口"按钮
    //    默认行为：关闭窗口时销毁（释放资源）
    //    如需改为隐藏窗口（保留状态），将 CloseBehavior::Destroy 改为 CloseBehavior::Hide
    let custom_terminal_handle: Rc<RefCell<Option<CustomTerminalWindow>>> = Rc::new(RefCell::new(None));
    let handle_custom = custom_terminal_handle.clone();
    app.on_open_custom_terminal_window(move || {
        match windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Destroy) {
            Ok(()) => {
                println!("[Main] 自绘终端窗口已打开");
            }
            Err(e) => {
                eprintln!("[Main] 打开自绘终端窗口失败: {}", e);
            }
        }
    });

    // 6. 绑定"打开原生终端窗口"按钮
    let native_terminal_handle: Rc<RefCell<Option<NativeTerminalWindow>>> = Rc::new(RefCell::new(None));
    let handle_native = native_terminal_handle.clone();
    app.on_open_native_terminal_window(move || {
        match windows::native_terminal::open(handle_native.clone(), windows::CloseBehavior::Destroy) {
            Ok(()) => {
                println!("[Main] 原生终端窗口已打开");
            }
            Err(e) => {
                eprintln!("[Main] 打开原生终端窗口失败: {}", e);
            }
        }
    });

    // 7. 运行主循环
    app.run()?;

    Ok(())
}