[English](README.md) | [简体中文](README_CN.md)

# slint-windows-mica-template

A modern, production-grade **Slint Fluent Design** borderless window template tailored for Windows! ✨

Features out-of-the-box support for **DWM native Mica material**, **zero-flicker CBT Hook instant injection**, **multi-monitor cursor-following smart centering**, **Windows 11 Snap Layouts**, and **dual-mode (Custom-Drawn & DWM Native) titlebar caption buttons**.

---

## 📖 About The Project

Slint is a lightweight, modern, declarative GUI framework for Rust. However, building Windows applications with true native look-and-feel often comes with challenges:
- Lack of built-in Win11 Mica material and dark/light theme adaptability;
- Borderless window startup visual flickering (white/gray frame flash before rendering);
- Complex positioning and window centering across multiple monitors;
- Caption button glitches during maximization, taskbar occlusion, and missing resize borders.

This project provides a robust, modular, and fully tested Win32/DWM subclassing architecture on top of Slint. It serves as both a comprehensive learning reference and a turnkey application scaffold.

> Special thanks to [@Drew-Chase](https://github.com/Drew-Chase) for creating [slint_borderless_windows](https://github.com/Drew-Chase/slint_borderless_windows) and providing the foundation!

---

## ✨ Key Features

### 1. 🪟 Three Window Paradigms (Ready-to-Use)
- **Main Window (`AppWindow`) — Custom-Drawn Fluent Controls**:
  - Slint custom-drawn titlebar and control buttons (Minimize, Maximize/Restore, Close);
  - Full integration with **Windows 11 Snap Layouts menu** on hover;
  - Full 8-direction smooth edge resizing, titlebar dragging, and double-click to maximize/restore.
- **Custom Terminal Child Window (`CustomTerminalWindow`)**:
  - Demonstrates lightweight custom titlebar and controls;
  - Built-in real-time console log viewer with one-click copy.
- **Native Terminal Child Window (`NativeTerminalWindow`) — DWM Native Buttons**:
  - Right-side caption buttons are **rendered and handled natively by Windows DWM** (`WS_OVERLAPPEDWINDOW` + `WM_NCCALCSIZE` client area extension);
  - Slint only renders the left drag area and content, keeping the right button area transparent;
  - Provides `WM_NCCALCSIZE` alignment in maximized mode, screen-space absolute coordinate hit-testing, `TrackMouseEvent` hover glow animations, and full 8-direction resize borders.

### 2. ⚡ Zero-Flicker CBT Hook Instant Injection
- Utilizes Win32 `WH_CBT` hook (`HCBT_CREATEWND` / `HCBT_ACTIVATE`) to synchronously capture the window `HWND` at `CreateWindowExW` creation time;
- Injects DWM Mica backdrop, dark/light theme, rounded corners, and subclassing before the very first frame is rendered;
- Completely eliminates the classic "white/gray box flash" on window startup.

### 3. 🖥️ Multi-Monitor Smart Centering
- Automatically detects the active monitor where the mouse cursor is located;
- Positions popups centered within the screen's **Work Area** (automatically avoiding taskbar overlap);
- Built-in boundary protection to prevent windows from exceeding screen dimensions on any DPI.

### 4. 🎨 Fluent Design & Mica Material
- Native Windows 11 Mica material integration that automatically synchronizes with system dark/light modes;
- Compatible with various Slint rendering backends (Skia, FemtoVG, Software, etc.) ensuring stable transparency and text antialiasing.

### 5. 🛠️ Modular & Decoupled Architecture
- 1-to-1 mapping between UI components and Rust modules:
  - `src/windows/app/` ↔ `ui/app-window/`
  - `src/windows/custom_terminal/` ↔ `ui/custom-terminal-window/`
  - `src/windows/native_terminal/` ↔ `ui/native-terminal-window/`
- Reusable platform primitives organized under `src/platform/` (attributes, borderless engine, CBT Hook guard, display/monitor helpers, effects).

---

## 📂 Project Structure

```text
slint-windows-mica-template/
├── src/
│   ├── main.rs                   # Entry point, window initialization & CBT Hook dispatch
│   ├── sys_info.rs               # Windows OS version & system theme detection
│   ├── platform/                 # Windows platform-level window infrastructure
│   │   ├── attributes.rs         # Window attributes (Mica, Dark Mode, Corners, Escapes)
│   │   ├── borderless.rs         # Win32 borderless subclassing & Snap Layouts processing
│   │   ├── controls.rs           # Titlebar controls adapter & button callbacks
│   │   ├── display.rs            # Multi-monitor cursor-following centering calculations
│   │   ├── effects.rs            # Mica transparency & backdrop linkage
│   │   ├── hook.rs               # CBT Hook zero-flicker instant injection guard
│   │   └── mod.rs
│   └── windows/                  # Concrete UI windows logic
│       ├── app/                  # Main window logic (custom-drawn titlebar controls)
│       ├── custom_terminal/      # Custom terminal child window logic
│       ├── native_terminal/      # DWM native caption buttons child window logic & subclassing
│       └── mod.rs
├── ui/
│   ├── app-window/               # Main window Slint UI
│   ├── custom-terminal-window/   # Custom terminal child window Slint UI
│   └── native-terminal-window/   # Native terminal child window Slint UI
├── Cargo.toml
└── build.rs
```

---

## 🚀 Quick Start

### Prerequisites
- **Rust** (Latest stable version recommended, Edition 2024 supported)
- **Windows 11 / Windows 10** (Mica effect requires Windows 11 Build 22000+)

### Running Locally
```bash
# 1. Clone this repository
git clone https://github.com/your-username/slint-windows-mica-template.git
cd slint-windows-mica-template

# 2. Build and run locally
cargo run
```

> 💡 **Tip**: When adapting this template for your project, remember to update `name = "..."` in `Cargo.toml`.

---

## ⚠️ Known Issues & Troubleshooting

### 1. NVIDIA OpenGL Transparent Background Turning Black
* **Symptom**: On NVIDIA GPU systems, using OpenGL-based rendering backends (such as FemtoVG / Skia-OpenGL) may cause transparent window regions to turn completely black.
* **Cause**: The NVIDIA driver's `OpenGL GDI compatibility` defaults to "Auto" or "Prefer performance", which breaks DWM window alpha transparency.
* **Solutions & Backend Selection**:
  1. **Modify GPU Driver Settings**: In `NVIDIA Control Panel` -> `Manage 3D settings` -> `Global Settings`, manually set `OpenGL GDI compatibility` to **"Prefer compatibility"**;
  2. **Zero-Config Alternative (Recommended)**: If you cannot or do not want end-users to tweak NVIDIA driver settings, **OpenGL rendering is not viable**. Instead, switch to the **`software`** (software rendering) or **`renderer-wgpu`** (Direct3D 12 / Vulkan) backend, both of which are completely free of this restriction and deliver native Mica transparency out of the box.

### 2. DWM Native Caption Buttons Maximization & Resizing
* **Symptom**: Custom borderless windows with DWM buttons may lose hover effects or top-edge resizing when maximized.
* **Solution**: Handled with precision in `native_terminal_window/borderless.rs` via top-aligned `WM_NCCALCSIZE`, physical screen coordinate hit-testing, and `TrackMouseEvent(TME_NONCLIENT | TME_LEAVE)`.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE). Feel free to use, modify, and distribute it in your own projects!