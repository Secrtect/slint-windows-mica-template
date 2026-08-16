[English](README.md) | [简体中文](README_CN.md)

# slint-windows-mica-template

一个基于 Slint 的 Windows 平台无边框与 Mica 效果窗口模板与参考实现。

包含 **DWM Mica 背景材质**、**CBT Hook 防启动白屏/闪烁**、**多显示器鼠标跟随居中**、**Win11 Snap Layouts 贴靠菜单** 以及 **自绘 / DWM 原生两种标题栏按钮** 的接入示例。

---

## 📖 简介

在 Windows 平台上使用 Slint 开发桌面应用时，通常会遇到一些平台特定的问题：
- 缺乏开箱即用的 Win11 Mica 材质与系统深浅色联动；
- 无边框窗口启动瞬间容易出现白屏闪烁或阴影延迟；
- 多屏幕环境下弹窗定位和避免超出屏幕；
- 标题栏非客户区消息处理、最大化贴靠（Snap Layouts）与 8 方向拉伸边框。

本项目基于 Slint + Win32/DWM 子类化，封装了基础的窗口支撑代码，提供了一套可复用的示例与骨架，供学习或作为项目参考。

> 感谢 [@Drew-Chase](https://github.com/Drew-Chase) 开源的 [slint_borderless_windows](https://github.com/Drew-Chase/slint_borderless_windows) 提供的基础思路。

---

## 📌 功能特性

### 1. 窗口范式示例
- **主窗口（自绘标题栏与控件）**：
  - Slint 自绘标题栏与控制按钮（最小化、最大化/还原、关闭）；
  - 接入 **Windows 11 Snap Layouts（贴靠布局菜单）**；
  - 支持 8 方向拉伸、标题栏拖拽、双击切换最大化/还原。
- **自绘终端日志子窗口**：
  - 演示轻量级自绘标题栏与自定义控制；
  - 包含日志展示与复制操作。
- **原生终端日志子窗口（DWM 系统原生按钮）**：
  - 标题栏右侧按钮由 **Windows DWM 原生绘制**（最小化/最大化/关闭）；
  - Slint 仅负责左侧内容和拖拽区，右侧留空透传；
  - 包含最大化模式下的 `WM_NCCALCSIZE` 顶部对齐、物理坐标命中测试与 `TrackMouseEvent` 悬停高亮。

### 2. CBT Hook 瞬时注入
- 利用 Win32 `WH_CBT` 钩子在 `CreateWindowExW` 窗口创建瞬间捕获原生 HWND；
- 在窗口首帧绘制前同步完成 Mica 材质注入、深浅色模式设置、圆角配置及子类化安装；
- 避免传统事件循环中“先白屏再变色”的闪烁。

### 3. 多显示器居中定位
- 根据当前鼠标所在的活动显示器，以该屏幕的 **工作区（Work Area，避开任务栏）** 居中弹出；
- 包含基础的防超屏尺寸保护。

### 4. Mica 材质与深浅色联动
- 支持 Win11 Mica 材质（自动跟随系统深浅色模式）；
- 适配常见 Slint 渲染后端。

### 5. 模块化分层
- UI 与 Rust 逻辑按模块对应：
  - `src/windows/app/` ↔ `ui/app-window/`
  - `src/windows/custom_terminal/` ↔ `ui/custom-terminal-window/`
  - `src/windows/native_terminal/` ↔ `ui/native-terminal-window/`
- 通用 Windows 平台能力收归至 `src/platform/`（属性配置、无边框子类化、CBT Hook、多屏定位等），方便按需复用与修改。

---

## 📂 项目结构

```text
slint-windows-mica-template/
├── src/
│   ├── main.rs                   # 程序入口，窗口初始化与 CBT Hook 调度
│   ├── sys_info.rs               # Windows 系统版本与深浅色检测
│   ├── platform/                 # Windows 平台级窗口基础设施
│   │   ├── attributes.rs         # 窗口属性（Mica、暗色、圆角、置顶、逃生通道）
│   │   ├── borderless.rs         # Win32 无边框窗口子类化与 Snap Layouts 消息处理
│   │   ├── controls.rs           # 标题栏控制适配器与按钮交互
│   │   ├── display.rs            # 多显示器鼠标跟随智能居中定位计算
│   │   ├── effects.rs            # Mica 透明度与特效联动
│   │   ├── hook.rs               # CBT Hook 防闪烁瞬时注入守卫
│   │   └── mod.rs
│   └── windows/                  # 具体的 UI 窗口业务实现
│       ├── app/                  # 主窗口逻辑（自绘标题栏控件）
│       │   ├── attributes.rs     # 主窗口 DWM 属性配置
│       │   ├── controls.rs       # 主窗口标题栏控件绑定
│       │   └── mod.rs            # 主窗口生命周期管理
│       ├── custom_terminal/      # 自绘终端子窗口逻辑
│       ├── native_terminal/      # 原生 DWM 按钮子窗口逻辑与子类化
│       └── mod.rs                # 导出公共窗口类型（如 CloseBehavior）
├── ui/
│   ├── app-window/               # 主窗口 Slint UI
│   ├── custom-terminal-window/   # 自绘终端子窗口 Slint UI
│   └── native-terminal-window/   # 原生终端子窗口 Slint UI
├── Cargo.toml
└── build.rs
```

---

## 🚀 快速开始

### 依赖环境
- **Rust**（建议最新稳定版，支持 Edition 2024）
- **Windows 11 / Windows 10**（Mica 特效需要 Windows 11 Build 22000+）

### 运行步骤
```bash
# 1. 克隆本仓库
git clone https://github.com/your-username/slint-windows-mica-template.git
cd slint-windows-mica-template

# 2. 本地编译并运行
cargo run
```

---

## 💡 使用与定制教程 (Cookbook)

本节介绍如何基于此模板进行常见的窗口行为定制与二次开发。

### 1. 关掉/禁用标题栏按钮（最小化 / 最大化 / 关闭）

项目采用 `TitlebarButtons` 统一管理非客户区命中测试与 UI 按钮可见性。

#### 在主窗口中修改
进入 [src/windows/app/mod.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/app/mod.rs)，修改 `TitlebarButtons`：

```rust
// 示例：关闭最小化与最大化按钮，仅保留关闭按钮（如固定大小的对话框）
pub fn create_frame(app: &AppWindow) -> WindowFrame<AppWindow> {
    let buttons = TitlebarButtons {
        show_minimize: false, // 禁用最小化
        show_maximize: false, // 禁用最大化
        show_close: true,     // 保留关闭
    };
    WindowFrame::new(app, Arc::new(GlobalWindowControlsAdapter::new(buttons)))
}

pub fn setup(app: &AppWindow, frame: &WindowFrame<AppWindow>, hook_ok: bool) {
    // ...
    let buttons = TitlebarButtons {
        show_minimize: false,
        show_maximize: false,
        show_close: true,
    };
    controls::setup_window_controls(app, frame.clone(), buttons);
}
```

> **原理**：将 `show_minimize` 或 `show_maximize` 设为 `false` 后：
> 1. Win32 消息处理层（`WM_NCHITTEST`）会自动跳过对应按钮的命中测试，避免鼠标悬停在空白区域触发 Snap Layouts 或最小化操作；
> 2. Slint UI 侧的对应按钮元素（如 `WindowControls.show-minimize`）会自动隐藏。

#### 在自绘子窗口中修改
在 [src/windows/custom_terminal/mod.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/custom_terminal/mod.rs) 的 `open` 函数中修改：
```rust
let buttons = TitlebarButtons {
    show_minimize: false,
    show_maximize: false,
    show_close: true,
};
let frame = WindowFrame::new(&terminal, Arc::new(TerminalTitlebarAdapter::new(buttons)));
```

---

### 2. 子窗口关闭行为：隐藏 (Hide) 还是 销毁 (Destroy)？

在桌面应用中，二级窗口（如设置面板、终端日志、调试器等）在被用户点击关闭按钮时通常有两种策略：
- **`Destroy`（销毁模式）**：彻底释放原生窗口及 Slint 组件资源，适合用完即弃或低频打开的窗口；
- **`Hide`（隐藏模式）**：窗口关闭时仅调用 `hide()`，内存中的组件实例及输入状态（如日志滚动位置、输入框内容）依然保留，再次点击打开时直接 `show()`。

在 [src/main.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/main.rs) 中，通过 `CloseBehavior` 枚举指定行为：

```rust
// 方式 A：销毁模式（默认）
windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Destroy)?;

// 方式 B：隐藏模式（保留状态）
windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Hide)?;
```

若使用的是 `CloseBehavior::Hide`，下次打开时 `open` 函数会智能复用已存在的实例并重新显示。

---

### 3. 配置窗口样式与视觉材质 (WindowAttributes)

每个窗口都在其独立的 `attributes.rs`（例如 [src/windows/app/attributes.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/app/attributes.rs)）中声明式配置 DWM 特效与 Win32 窗口样式。

```rust
use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        // ── 1. DWM 视觉材质 ──
        .with_mica()              // Win11 标准 Mica 材质
        // .with_acrylic()        // Win11 亚克力半透明模糊材质
        // .with_tabbed()         // Win11 Tabbed 材质
        
        // ── 2. 深浅色模式 ──
        .with_dark_mode(is_system_dark_mode()) // 跟随系统主题，亦可强制传 true / false
        
        // ── 3. 圆角控制 ──
        .with_corner(CornerPreference::Round)  // Round（标准大圆角）/ RoundSmall（小圆角）/ DoNotRound（直角）
        
        // ── 4. 任务栏与置顶 ──
        .with_app_window(true)      // 确保在任务栏显示独立图标
        // .with_tool_window(true)  // 设置为工具窗口（不在任务栏和 Alt+Tab 中显示）
        // .with_always_on_top(true)// 窗口始终置顶
        
        // ── 5. 交互样式 ──
        // .with_click_through(true)// 鼠标点击穿透
        // .with_no_activate(true)  // 点击不抢焦点
}
```

---

### 4. 新建一个自定义窗口的完整流程

如果你需要为项目增加一个全新的窗口（例如 `SettingsWindow`），只需遵循以下 4 步：

#### 步骤 1：编写 Slint UI
在 `ui/` 下创建 `settings-window/settings-window.slint`：
```slint
import { Titlebar } from "../app-window/titlebar.slint";

export component SettingsWindow inherits Window {
    title: "设置 (Settings)";
    in-out property <bool> is-mica-active: false;
    background: root.is-mica-active ? #00000000 : #202020;
    width: 600px;
    height: 400px;
    no-frame: true; // 启用自定义无边框

    VerticalLayout {
        Titlebar { title: root.title; }
        // 你的业务 UI 内容...
    }
}
```

#### 步骤 2：创建 Rust 窗口模块
在 `src/windows/` 下创建 `settings/` 目录：
- `attributes.rs`：配置 `WindowAttributes`（Mica、圆角等）。
- `mod.rs`：编写 `open` 函数，使用 `CbtHookGuard::install` 瞬时注入属性与子类化，并调用 `center_component_on_active_monitor` 居中。

```rust
use crate::SettingsWindow;
use crate::platform::{CbtHookGuard, WindowFrame, apply_mica_effect, center_component_on_active_monitor};
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::HWND;

pub fn open() -> Result<(), slint::PlatformError> {
    let frame_holder = Arc::new(Mutex::new(None));
    
    // 1. CBT Hook 防启动闪烁
    let hook = CbtHookGuard::install(
        Some("设置 (Settings)".to_string()),
        {
            let frame_holder = Arc::clone(&frame_holder);
            move |hwnd_isize| {
                attributes::get_attributes().apply(hwnd_isize);
                if let Some(ref frame) = *frame_holder.lock().unwrap() {
                    frame.apply_to_hwnd(HWND(hwnd_isize as *mut _));
                }
            }
        }
    );
    let hook_ok = hook.is_ok();
    if let Ok(guard) = hook { std::mem::forget(guard); }

    // 2. 实例化组件与框架
    let window = SettingsWindow::new()?;
    let frame = WindowFrame::new(&window, Arc::new(/* your TitlebarAdapter */));
    *frame_holder.lock().unwrap() = Some(frame.clone());

    // 3. 居中与特效
    center_component_on_active_monitor(&window, 600.0, 400.0);
    apply_mica_effect(&window, |w| w.set_is_mica_active(true), hook_ok);

    // 4. 显示
    window.show()?;
    Ok(())
}
```

#### 步骤 3：在 `main.rs` 中注册 UI
在 `ui/` 入口中 export，并在 `src/main.rs` 中绑定点击事件即可！

---

### 5. 自绘标题栏 vs DWM 原生标题栏的选择

| 特性 | 自绘标题栏 (`app` / `custom_terminal`) | DWM 原生按钮 (`native_terminal`) |
| :--- | :--- | :--- |
| **外观与风格** | 100% Slint 自绘，可任意定制图标、间距与配色 | Windows 系统原生绘制的 Min/Max/Close 按钮 |
| **Win11 Snap Layouts** | ✅ 支持（通过 `WM_NCHITTEST` 返回 `HTMAXBUTTON`） | ✅ 支持（系统原生提供） |
| **失焦变淡动画** | ✅ 支持（通过 `WM_NCACTIVATE` 联动） | ✅ 支持（系统原生提供） |
| **适用场景** | 追求整体统一设计语言、个性化主题的现代化桌面软件 | 追求极简、完全贴合系统原生控件外观的工具软件 |

---

### 6. 高级逃生通道（Escape Hatches）

如果你需要调用底层特定的 Windows API 或未文档化的 DWM 属性：

```rust
WindowAttributes::new()
    // 逃生通道 1：直接向 HWND 设置任意 DWM 属性 ID 和数据
    .with_raw_dwm_attribute(1029 /* DWMWA_MICA_EFFECT */, 1u32)
    // 逃生通道 2：在窗口被 CBT Hook 捕获时直接获取原始 HWND 闭包
    .with_custom_action(|hwnd: windows_sys::Win32::Foundation::HWND| {
        // 在这里可以执行任何 Win32 API
        println!("Raw HWND: {:?}", hwnd);
    })
```

---

## ⚠️ 常见问题与注意事项

### 1. NVIDIA 显卡 OpenGL 透明背景黑屏问题
* **现象**：在 NVIDIA 显卡设备上使用 OpenGL 渲染后端（如 FemtoVG / Skia-OpenGL）时，透明窗口区域可能呈现纯黑背景。
* **原因**：NVIDIA 驱动设置中 `OpenGL GDI 兼容性` 默认设为“自动”或“优先性能”模式，会导致 DWM 窗口 alpha 透明通道失效。
* **解决与后端选择**：
  1. **修改显卡设置**：在 `NVIDIA 控制面板` -> `管理 3D 设置` -> `全局设置` 中，将 `OpenGL GDI 兼容性` 手动更改为 **“优先兼容性”**；
  2. **免修改驱动方案**：选用 **`software`（软件渲染）** 或 **`renderer-wgpu`（基于 Direct3D 12 / Vulkan）** 后端，不受此驱动选项限制。

### 2. DWM 原生按钮交互细节
* 原生窗口模式下，标题栏按钮区域由 Windows DWM 直接接管。本项目在 `src/windows/native_terminal/borderless.rs` 中处理了顶部对齐 `WM_NCCALCSIZE`、物理坐标命中测试以及 `TrackMouseEvent` 悬停状态更新。

---

## 📄 开源许可

本项目遵循 [MIT License](LICENSE) 开源协议。