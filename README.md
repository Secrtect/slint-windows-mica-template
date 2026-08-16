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
│       │   ├── attributes.rs     # Main window DWM attributes configuration
│       │   ├── controls.rs       # Main window titlebar controls binding
│       │   └── mod.rs            # Main window lifecycle management
│       ├── custom_terminal/      # Custom terminal child window logic
│       ├── native_terminal/      # DWM native caption buttons child window logic & subclassing
│       └── mod.rs                # Public window types export (e.g. CloseBehavior)
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

## 💡 Usage & Developer Guide (Cookbook)

This section covers common window customization scenarios and secondary development workflows.

### 1. Disable / Hide Titlebar Buttons (Minimize / Maximize / Close)

The template uses `TitlebarButtons` to configure both non-client area Win32 hit-testing and Slint UI button visibility simultaneously.

#### For the Main Window
Open [src/windows/app/mod.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/app/mod.rs) and modify `TitlebarButtons`:

```rust
// Example: Disable minimize and maximize buttons, keeping only the close button
pub fn create_frame(app: &AppWindow) -> WindowFrame<AppWindow> {
    let buttons = TitlebarButtons {
        show_minimize: false, // Disable minimize
        show_maximize: false, // Disable maximize
        show_close: true,     // Keep close
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

> **How it works**: When `show_minimize` or `show_maximize` is set to `false`:
> 1. The Win32 non-client hit-test handler (`WM_NCHITTEST`) skips the corresponding button area, preventing accidental Snap Layouts or minimizing triggers.
> 2. The Slint UI binding (`WindowControls.show-minimize` / `show-maximize`) automatically hides the corresponding button visual element.

#### For Custom Child Windows
In [src/windows/custom_terminal/mod.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/custom_terminal/mod.rs), customize the `buttons` struct inside `open`:
```rust
let buttons = TitlebarButtons {
    show_minimize: false,
    show_maximize: false,
    show_close: true,
};
let frame = WindowFrame::new(&terminal, Arc::new(TerminalTitlebarAdapter::new(buttons)));
```

---

### 2. Window Close Behavior: Hide vs. Destroy

Secondary windows (such as settings dialogs, terminal logs, or debug panels) can adopt two closing strategies:
- **`Destroy` (Default)**: Completely releases the native window handle and Slint component instance. Ideal for disposable, low-frequency windows.
- **`Hide`**: Simply calls `hide()` on close, preserving memory state (e.g. scroll position, input contents) for quick subsequent toggles.

In [src/main.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/main.rs), pass the `CloseBehavior` enum to the window opener:

```rust
// Mode A: Destroy on close (default, frees resources)
windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Destroy)?;

// Mode B: Hide on close (keeps state alive)
windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Hide)?;
```

---

### 3. Window Attributes & Visual Material (WindowAttributes)

Each window configures its DWM effects and Win32 styles declaratively in its dedicated `attributes.rs` (e.g. [src/windows/app/attributes.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/app/attributes.rs)):

```rust
use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        // ── 1. DWM Material ──
        .with_mica()              // Win11 standard Mica material
        // .with_acrylic()        // Win11 Acrylic translucent blur
        // .with_tabbed()         // Win11 Tabbed material
        
        // ── 2. Dark / Light Mode ──
        .with_dark_mode(is_system_dark_mode()) // Follow system theme or force true/false
        
        // ── 3. Corner Preference ──
        .with_corner(CornerPreference::Round)  // Round / RoundSmall / DoNotRound
        
        // ── 4. Taskbar & Topmost ──
        .with_app_window(true)      // Display icon in taskbar
        // .with_tool_window(true)  // Tool window (hidden from taskbar & Alt+Tab)
        // .with_always_on_top(true)// Always on top
        
        // ── 5. Input Behavior ──
        // .with_click_through(true)// Mouse click-through
        // .with_no_activate(true)  // Do not steal focus on click
}
```

---

### 4. Creating a New Custom Window

To add a new window (e.g. `SettingsWindow`), follow these 4 steps:

#### Step 1: Define Slint UI
Create `ui/settings-window/settings-window.slint`:
```slint
import { Titlebar } from "../app-window/titlebar.slint";

export component SettingsWindow inherits Window {
    title: "Settings";
    in-out property <bool> is-mica-active: false;
    background: root.is-mica-active ? #00000000 : #202020;
    width: 600px;
    height: 400px;
    no-frame: true;

    VerticalLayout {
        Titlebar { title: root.title; }
        // Your content here...
    }
}
```

#### Step 2: Implement Rust Module
Create `src/windows/settings/` with `attributes.rs` and `mod.rs`:
```rust
use crate::SettingsWindow;
use crate::platform::{CbtHookGuard, WindowFrame, apply_mica_effect, center_component_on_active_monitor};
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::HWND;

pub fn open() -> Result<(), slint::PlatformError> {
    let frame_holder = Arc::new(Mutex::new(None));
    
    // 1. CBT Hook for zero-flicker startup
    let hook = CbtHookGuard::install(
        Some("Settings".to_string()),
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

    // 2. Initialize component & frame
    let window = SettingsWindow::new()?;
    let frame = WindowFrame::new(&window, Arc::new(/* your TitlebarAdapter */));
    *frame_holder.lock().unwrap() = Some(frame.clone());

    // 3. Center and apply Mica
    center_component_on_active_monitor(&window, 600.0, 400.0);
    apply_mica_effect(&window, |w| w.set_is_mica_active(true), hook_ok);

    // 4. Show window
    window.show()?;
    Ok(())
}
```

#### Step 3: Register in `main.rs`
Export the component in `ui/` and bind user interaction in `src/main.rs`.

---

### 5. Custom-Drawn Titlebar vs. DWM Native Buttons

| Feature | Custom Titlebar (`app` / `custom_terminal`) | DWM Native Buttons (`native_terminal`) |
| :--- | :--- | :--- |
| **Look & Feel** | 100% Slint custom UI with customizable icons, padding, & theme | Native Windows DWM caption buttons |
| **Win11 Snap Layouts** | ✅ Supported (via `WM_NCHITTEST` returning `HTMAXBUTTON`) | ✅ Supported (native) |
| **Inactive Window Dimming** | ✅ Supported (via `WM_NCACTIVATE`) | ✅ Supported (native) |
| **Best For** | Modern desktop apps with consistent brand design systems | Utility tools requiring strict system-native appearance |

---

### 6. Advanced Escape Hatches

For direct Win32 API access or undocumented DWM attributes:

```rust
WindowAttributes::new()
    // Escape Hatch 1: Direct DWM attribute injection
    .with_raw_dwm_attribute(1029 /* DWMWA_MICA_EFFECT */, 1u32)
    // Escape Hatch 2: Raw HWND access during CBT Hook capture
    .with_custom_action(|hwnd: windows_sys::Win32::Foundation::HWND| {
        // Execute arbitrary Win32 APIs here
        println!("Raw HWND: {:?}", hwnd);
    })
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