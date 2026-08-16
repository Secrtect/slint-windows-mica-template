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
- 标题栏非客户区消息处理、最大化贴靠与 8 方向拉伸边框。

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
│   ├── platform/                 # Windows 平台级窗口交互基础设施
│   │   ├── attributes.rs         # 窗口属性（Mica、暗色、圆角、逃生通道）
│   │   ├── borderless.rs         # Win32 无边框窗口子类化与 Snap Layouts 消息处理
│   │   ├── controls.rs           # 标题栏控制适配器与按钮交互
│   │   ├── display.rs            # 多显示器鼠标跟随智能居中定位计算
│   │   ├── effects.rs            # Mica 透明度与特效联动
│   │   ├── hook.rs               # CBT Hook 防闪烁瞬时注入守卫
│   │   └── mod.rs
│   └── windows/                  # 具体的 UI 窗口业务实现
│       ├── app/                  # 主窗口逻辑（自绘标题栏控件）
│       ├── custom_terminal/      # 自绘终端子窗口逻辑
│       ├── native_terminal/      # 原生 DWM 按钮子窗口逻辑与子类化
│       └── mod.rs
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