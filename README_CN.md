[English](README.md) | [简体中文](README_CN.md)

# slint-windows-mica-template

一套专为 Windows 平台打造的 **Slint Fluent Design** 现代化无边框窗口模版！✨

集成了 **DWM Mica 原生背景材质**、**CBT Hook 零闪烁瞬时注入**、**多显示器鼠标跟随智能居中**、**Win11 Snap Layouts 贴靠布局** 以及 **自绘 / DWM 原生双模标题栏按钮** 等全套企业级基础设施。

---

## 📖 关于本项目

Slint 是一个极其轻量、现代且声明式的 Rust GUI 框架。但在 Windows 平台上开发原生质感的桌面应用时，通常面临以下痛点：
- 缺乏开箱即用的 Windows 11 Mica 材质与暗色/亮色自适应；
- 无边框窗口在启动时容易出现白屏闪烁或阴影丢失；
- 多屏幕环境下弹窗定位复杂；
- 标题栏双击抖动、最大化工作区任务栏遮挡、拉伸边框不完整等细节坑点。

本项目基于 Slint + Win32/DWM 深度子类化，提炼出一整套完善、稳定、开箱即用的 Windows 无边框窗口与弹窗架构，无论是学习还是直接作为项目骨架均可无缝上手！

> 特别感谢 [@Drew-Chase](https://github.com/Drew-Chase) 开源的 [slint_borderless_windows](https://github.com/Drew-Chase/slint_borderless_windows) 提供的基础思路！

---

## ✨ 核心特性

### 1. 🪟 三种典型窗口范式（开箱即用）
- **主窗口（全套自绘控件模版）**：
  - Slint 自绘 Fluent 风格标题栏与最小化/最大化/关闭按钮；
  - 完美接入 **Windows 11 Snap Layouts（贴靠布局菜单）**；
  - 支持 8 方向顺滑拉伸、标题栏拖拽、双击切换最大化/还原。
- **自绘终端日志子窗口（自定义轻量弹窗模版）**：
  - 演示轻量级自绘标题栏与控制区；
  - 内置控制台日志实时展示与一键复制功能。
- **原生终端日志子窗口（DWM 系统原生按钮模版）**：
  - 标题栏右侧直接由 **Windows DWM 系统原生绘制按钮**（关闭/最大化/最小化）；
  - Slint 仅渲染左侧内容与拖拽区，右侧原生透明占位；
  - 提供了最大化模式下的 `WM_NCCALCSIZE`、物理屏幕绝对坐标 hit-test、`TrackMouseEvent` 悬停高亮动画及 8 方向拉伸。

### 2. ⚡ CBT Hook 零延迟无闪烁注入
- 利用 Win32 `WH_CBT` 钩子（`HCBT_CREATEWND` / `HCBT_ACTIVATE`）在 `CreateWindowExW` 窗口创建瞬间捕获原生 HWND；
- 在窗口首帧绘制前同步完成 Mica 材质注入、暗色/亮色模式设置、圆角配置及 Win32 子类化安装；
- 彻底消除传统事件循环中“先白屏/灰屏再变 Mica”的视觉闪烁。

### 3. 🖥️ 多显示器智能居中定位
- 自动识别当前鼠标所在的活动显示器，以该屏幕的 **工作区（Work Area，自动避开任务栏）** 为基准居中弹出；
- 内置防超屏尺寸校验，确保窗口在任意 DPI 和分辨率下均完美展示。

### 4. 🎨 Fluent Design & Mica 材质联动
- 支持 Win11 Mica 效果（兼容暗色与亮色模式自动跟随系统）；
- 兼容多种 Slint 渲染后端（Skia、FemtoVG、Software 等），保证透明通道与文本渲染的稳定性。

### 5. 🛠️ 模块化解耦架构
- UI 与 Rust 逻辑严格按模块对应：
  - `src/app_window/` ↔ `ui/app-window/`
  - `src/custom_terminal_window/` ↔ `ui/custom-terminal-window/`
  - `src/native_terminal_window/` ↔ `ui/native-terminal-window/`
- 通用能力下沉至 `src/window/`，包含属性配置、无边框子类化、CBT Hook 守卫、多屏定位等，易于复用与 DIY。

---

## 📂 项目结构

```text
slint-windows-mica-template/
├── src/
│   ├── main.rs                   # 程序入口，窗口初始化与 CBT Hook 调度
│   ├── app_window/               # 主窗口逻辑（自绘标题栏控件）
│   ├── custom_terminal_window/   # 自绘终端子窗口逻辑
│   ├── native_terminal_window/   # 原生 DWM 按钮子窗口逻辑与子类化
│   └── window/                   # 核心底层基础设施
│       ├── attributes.rs         # 窗口属性（Mica、暗色、圆角）
│       ├── borderless.rs         # Win32 无边框窗口子类化与消息处理
│       ├── cbt_hook.rs           # CBT Hook 防闪烁瞬时注入守卫
│       ├── effects.rs            # Mica 透明度与特效联动
│       ├── monitor.rs            # 多显示器鼠标跟随定位计算
│       └── sys_info.rs           # Windows 系统版本与深浅色检测
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

> 💡 **提示**：在实际开发中，请记得在 `Cargo.toml` 中将 `name = "..."` 替换为您自己的项目名称。

---

## ⚠️ 踩坑记录与排错指引

### 1. NVIDIA 显卡 OpenGL 透明背景黑屏问题
* **现象**：在 NVIDIA 显卡设备上使用 OpenGL 渲染后端（如 FemtoVG / Skia-OpenGL）时，透明窗口区域可能直接呈现纯黑背景。
* **原因**：NVIDIA 驱动设置中 `OpenGL GDI 兼容性` 默认设为“自动”或“优先性能”模式，会导致 DWM 窗口 alpha 透明通道失效。
* **解决与后端选择**：
  1. **修改显卡设置**：在 `NVIDIA 控制面板` -> `管理 3D 设置` -> `全局设置` 中，将 `OpenGL GDI 兼容性` 手动更改为 **“优先兼容性”**；
  2. **免修改驱动方案（推荐）**：如果不希望或无法让最终用户修改显卡控制面板设置，在不修改 OpenGL GDI 兼容性的前提下 **不能使用 OpenGL 渲染**，建议选用 **`software`（软件渲染）** 或 **`renderer-wgpu`（基于 Direct3D 12 / Vulkan）** 后端，它们不受此限制，开箱即可完美支持透明与 Mica 效果。

### 2. DWM 原生按钮最大化悬停与拉伸边框
* **现象**：自定义无边框窗口最大化后，DWM 原生按钮无法触发悬停高亮或无法点击。
* **解决**：本项目在 `borderless.rs` 中实现了精准的 `WM_NCCALCSIZE` 顶部对齐、物理屏幕绝对坐标命中测试与 `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE)` 事件链，确保 8 方向拉伸与按钮交互 100% 贴合 Windows 原生规范。

---

## 📄 开源许可

本项目遵循 [MIT License](LICENSE) 开源协议，欢迎自由使用、修改与衍生开发！