[English](README.md) | [简体中文](README_CN.md)

# slint-windows-mica-template

A template and reference implementation for borderless windows and Windows 11 Mica effect using Slint.

Demonstrates **DWM Mica backdrop material**, **CBT Hook startup flicker prevention**, **multi-monitor cursor-following window centering**, **Win11 Snap Layouts**, and **custom-drawn vs. DWM native titlebar buttons**.

---

## 📖 Overview

When building desktop applications on Windows with Slint, several platform-specific challenges often arise:
- Lack of built-in Win11 Mica material and dark/light mode synchronization;
- Borderless window startup visual flickering or delayed shadow rendering;
- Multi-monitor window positioning and bounds checking;
- Non-client area message handling, maximized state snap layouts, and 8-direction resize borders.

This project wraps basic Win32/DWM windowing primitives on top of Slint, providing a clean starter scaffold and practical examples for reference.

> Thanks to [@Drew-Chase](https://github.com/Drew-Chase) for the [slint_borderless_windows](https://github.com/Drew-Chase/slint_borderless_windows) project which provided the initial inspiration.

---

## 📌 Features

### 1. Window Paradigms
- **Main Window (Custom-drawn titlebar & controls)**:
  - Slint custom-drawn titlebar and control buttons (Minimize, Maximize/Restore, Close);
  - Integrated **Windows 11 Snap Layouts menu** on hover;
  - Supports 8-direction edge resizing, dragging, and double-click to toggle maximize.
- **Custom Terminal Child Window**:
  - Lightweight custom titlebar and custom controls example;
  - Basic log display and copy operations.
- **Native Terminal Child Window (DWM native buttons)**:
  - Caption buttons rendered natively by **Windows DWM** (Minimize, Maximize, Close);
  - Slint renders content and drag area, leaving the button area transparent;
  - Handles maximized `WM_NCCALCSIZE` alignment, coordinate hit-testing, and `TrackMouseEvent` hover glow.

### 2. CBT Hook HWND Injection
- Uses Win32 `WH_CBT` hook to capture native HWND synchronously at `CreateWindowExW` creation time;
- Injects Mica backdrop, theme modes, corner preferences, and subclassing before the first frame is rendered;
- Avoids the startup white/gray frame flash.

### 3. Multi-Monitor Centering
- Detects the active monitor where the mouse cursor is located;
- Positions windows centered within the screen's **Work Area** (avoiding taskbar overlap);
- Basic boundary protection against exceeding screen bounds.

### 4. Mica Material & System Theme
- Supports Win11 Mica material (automatically following system dark/light theme);
- Compatible with common Slint rendering backends.

### 5. Modular Structure
- 1-to-1 mapping between UI components and Rust modules:
  - `src/windows/app/` ↔ `ui/app-window/`
  - `src/windows/custom_terminal/` ↔ `ui/custom-terminal-window/`
  - `src/windows/native_terminal/` ↔ `ui/native-terminal-window/`
- Common Windows platform primitives grouped under `src/platform/` (attributes, borderless subclassing, CBT Hook, multi-monitor helpers) for easier reuse and customization.

---

## 📂 Project Structure

```text
slint-windows-mica-template/
├── src/
│   ├── main.rs                   # Entry point, window initialization & CBT Hook dispatch
│   ├── sys_info.rs               # Windows OS version & theme detection
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

---

## ⚠️ Notes & Troubleshooting

### 1. NVIDIA OpenGL Transparent Background Turning Black
* **Symptom**: On NVIDIA GPU systems, using OpenGL-based rendering backends (such as FemtoVG / Skia-OpenGL) may cause transparent window regions to turn black.
* **Cause**: NVIDIA driver's `OpenGL GDI compatibility` defaults to "Auto" or "Prefer performance", which interferes with DWM alpha transparency.
* **Solutions**:
  1. **GPU Driver Settings**: In `NVIDIA Control Panel` -> `Manage 3D settings` -> `Global Settings`, set `OpenGL GDI compatibility` to **"Prefer compatibility"**;
  2. **Driver-independent workaround**: Use the **`software`** or **`renderer-wgpu`** backend instead.

### 2. DWM Native Caption Buttons Interaction
* In native window mode, caption buttons are directly managed by Windows DWM. This template handles top-aligned `WM_NCCALCSIZE`, physical coordinate hit-testing, and `TrackMouseEvent` hover state updates in `src/windows/native_terminal/borderless.rs`.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).