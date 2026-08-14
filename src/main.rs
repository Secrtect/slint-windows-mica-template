#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

mod app_window;
mod custom_terminal_window;
mod native_terminal_window;
mod sys_info;
pub mod window;

// 引入自动生成的 UI 模块
slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    // 1. 安装主窗口 CBT Hook（在 CreateWindowExW 瞬间注入属性，杜绝闪烁）
    //    主窗口属性在 src/app_window/attributes.rs 中 DIY 配置
    let hook_installed = window::CbtHookGuard::install(
        Some("Slint标准组件全家桶".to_string()),
        |hwnd| {
            let attrs = app_window::attributes::get_attributes();
            attrs.apply(hwnd);
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

    // 3. 配置主窗口（无边框框架、居中定位、Mica 特效、控件绑定）
    let _frame = app_window::setup(&app, hook_ok)?;

    // 4. 绑定“打开自绘终端窗口”按钮
    let custom_terminal_handle: Rc<RefCell<Option<CustomTerminalWindow>>> = Rc::new(RefCell::new(None));
    let handle_custom = custom_terminal_handle.clone();
    app.on_open_custom_terminal_window(move || {
        match custom_terminal_window::open() {
            Ok(terminal) => {
                *handle_custom.borrow_mut() = Some(terminal);
                println!("[Main] 自绘终端窗口已打开");
            }
            Err(e) => {
                eprintln!("[Main] 打开自绘终端窗口失败: {}", e);
            }
        }
    });

    // 5. 绑定“打开原生终端窗口”按钮
    let native_terminal_handle: Rc<RefCell<Option<NativeTerminalWindow>>> = Rc::new(RefCell::new(None));
    let handle_native = native_terminal_handle.clone();
    app.on_open_native_terminal_window(move || {
        match native_terminal_window::open() {
            Ok(terminal) => {
                *handle_native.borrow_mut() = Some(terminal);
                println!("[Main] 原生终端窗口已打开");
            }
            Err(e) => {
                eprintln!("[Main] 打开原生终端窗口失败: {}", e);
            }
        }
    });

    // 6. 运行主循环
    app.run()?;

    Ok(())
}
