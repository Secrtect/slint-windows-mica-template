[English](README.md) | [简体中文](README_CN.md)

# slint-windows-mica-template

A beginner practice project and reference implementation for borderless windows and Windows 11 Mica effect using Slint.

> ⚠️ **Note**: This is a personal beginner practice/learning project, not an enterprise-grade library or production software. Intended for study and reference only.

Demonstrates **DWM Mica backdrop material**, **CBT Hook startup flicker prevention**, **multi-monitor cursor-following window centering**, **Win11 Snap Layouts**, and **custom-drawn vs. DWM native titlebar buttons**.

---

## 📷 Preview

### 1. Main Window (Mica Backdrop & Full Component Set)
| Theme Preview A | Theme Preview B |
| :---: | :---: |
| <img width="1200" height="700" alt="Main Window Preview 1" src="https://github.com/user-attachments/assets/78035e29-3ec2-486d-82e8-0ae661614e8c" /> | <img width="1200" height="700" alt="Main Window Preview 2" src="https://github.com/user-attachments/assets/a9f55f5a-6795-4319-8181-c42b0ed08584" /> |

### 2. Custom-Drawn Terminal Window (Custom Terminal)
> 100% Slint custom-drawn titlebar and control buttons with full Windows 11 Snap Layouts menu support.

| Preview 1 | Preview 2 |
| :---: | :---: |
| <img width="580" height="420" alt="Custom Terminal Preview 1" src="https://github.com/user-attachments/assets/031d8a49-4ed9-472e-9916-1150e9101be0" /> | <img width="580" height="420" alt="Custom Terminal Preview 2" src="https://github.com/user-attachments/assets/68979644-a408-4214-8fab-2e1cca4663ff" /> |

### 3. Native DWM Caption Buttons Terminal Window (Native Terminal)
> Min/Max/Close caption buttons rendered natively by Windows DWM.

| Preview 1 | Preview 2 |
| :---: | :---: |
| <img width="582" height="452" alt="Native Terminal Preview 1" src="https://github.com/user-attachments/assets/fb6e35ab-7937-40d3-81a9-8d078aced88b" /> | <img width="582" height="452" alt="Native Terminal Preview 2" src="https://github.com/user-attachments/assets/682d4825-fb33-4817-8998-047938fe5bf8" /> |

---

## 📖 Overview

When learning and exploring desktop UI development on Windows with Slint, several platform-specific challenges often arise:
- Lack of built-in Win11 Mica material and dark/light mode synchronization;
- Borderless window startup visual flickering or delayed shadow rendering;
- Multi-monitor window positioning and bounds checking;
- Non-client area message handling, maximized state snap layouts, and 8-direction resize borders.

This project wraps basic Win32/DWM windowing primitives on top of Slint as a beginner learning sandbox and reference example.

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
│   ├── main.rs                   # Entry point, window setup & CBT Hook orchestration
│   ├── sys_info.rs               # Windows OS version & theme detection
│   ├── platform/                 # Windows platform windowing primitives
│   │   ├── attributes.rs         # Window attributes (Mica, dark mode, corners, topmost, escape hatches)
│   │   ├── borderless.rs         # Win32 borderless subclassing & Snap Layouts message handling
│   │   ├── controls.rs           # Titlebar control adapters & button interactions
│   │   ├── display.rs            # Multi-monitor active cursor centering logic
│   │   ├── effects.rs            # Mica transparency & backdrop linkage
│   │   ├── hook.rs               # CBT Hook zero-flicker synchronous injection guard
│   │   └── mod.rs
│   └── windows/                  # Window business logic implementations
│       ├── app/                  # Main window logic (custom-drawn titlebar & controls)
│       │   ├── attributes.rs     # Main window DWM attributes configuration
│       │   ├── controls.rs       # Main window titlebar controls binding
│       │   └── mod.rs            # Main window lifecycle management
│       ├── custom_terminal/      # Custom terminal child window logic
│       ├── native_terminal/      # Native DWM caption button child window logic & subclassing
│       └── mod.rs                # Common window types export (e.g. CloseBehavior)
├── ui/
│   ├── app-window/               # Main window Slint UI
│   ├── custom-terminal-window/   # Custom terminal child window Slint UI
│   └── native-terminal-window/   # Native terminal child window Slint UI
├── Cargo.toml
└── build.rs
```

---

## 🚀 Getting Started

### Prerequisites
- **Rust** (Latest stable recommended, supports Edition 2024)
- **Windows 11 / Windows 10** (Mica backdrop effect requires Windows 11 Build 22000+)

### Running Locally
```bash
# 1. Clone the repository
git clone https://github.com/your-username/slint-windows-mica-template.git
cd slint-windows-mica-template

# 2. Build and run
cargo run
```

---

## 💡 Usage & Customization Guide (Cookbook)

This section demonstrates how to customize common window behaviors based on this template.

### 1. Disabling/Hiding Titlebar Buttons (Minimize / Maximize / Close)

The project manages non-client hit-testing and Slint UI button visibility uniformly through `TitlebarButtons`.

#### Customizing in the Main Window
In [src/windows/app/mod.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/app/mod.rs), modify `TitlebarButtons`:

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

> **Mechanism**: Setting `show_minimize` or `show_maximize` to `false`:
> 1. Informs Win32 message handling (`WM_NCHITTEST`) to skip hit-testing for those regions, preventing hovering over blank space from triggering Snap Layouts;
> 2. Automatically hides corresponding Slint UI button elements (e.g. `WindowControls.show-minimize`).

#### Customizing in the Custom Child Window
In [src/windows/custom_terminal/mod.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/custom_terminal/mod.rs)'s `open` function:
```rust
let buttons = TitlebarButtons {
    show_minimize: false,
    show_maximize: false,
    show_close: true,
};
let frame = WindowFrame::new(&terminal, Arc::new(TerminalTitlebarAdapter::new(buttons)));
```

---

### 2. Secondary Window Close Behavior: Hide vs. Destroy

Secondary windows (settings panels, terminal logs, debugger overlays) typically adopt one of two strategies when dismissed:
- **`Destroy` (Default)**: Calls `hide()` and clears the handle holder, releasing the native window and Slint component instances. Ideal for one-shot or infrequent windows;
- **`Hide`**: Calls `hide()` while keeping the component instance and state (e.g. scroll position, input text) in memory; when opened again, `open` reuses the existing instance and calls `show()`.

In [src/main.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/main.rs), configure this behavior via `CloseBehavior`:

```rust
// Mode A: Destroy on close (default)
windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Destroy)?;
windows::native_terminal::open(handle_native.clone(), windows::CloseBehavior::Destroy)?;

// Mode B: Hide on close (preserves state, smart reuse on next open)
windows::custom_terminal::open(handle_custom.clone(), windows::CloseBehavior::Hide)?;
windows::native_terminal::open(handle_native.clone(), windows::CloseBehavior::Hide)?;
```

---

### 3. Configuring Window Styles & Visual Attributes (WindowAttributes)

Each window configures DWM effects and Win32 styles declaratively in its dedicated `attributes.rs` (e.g., [src/windows/app/attributes.rs](file:///d:/RustProject/SlintStudy/slint-windows-mica-template/src/windows/app/attributes.rs)).

```rust
use crate::platform::attributes::{CornerPreference, WindowAttributes, is_system_dark_mode};

pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        // ── 1. DWM Visual Backdrop ──
        .with_mica()              // Win11 standard Mica backdrop
        // .with_acrylic()        // Win11 Acrylic translucent blur backdrop
        // .with_tabbed()         // Win11 Tabbed backdrop
        
        // ── 2. Dark/Light Mode ──
        .with_dark_mode(is_system_dark_mode()) // Follow system theme, or pass true / false explicitly
        
        // ── 3. Corner Preference ──
        .with_corner(CornerPreference::Round)  // Round (standard rounded) / RoundSmall / DoNotRound
        
        // ── 4. Taskbar & Topmost ──
        .with_app_window(true)      // Ensure independent icon in taskbar
        // .with_tool_window(true)  // Set as tool window (hidden from taskbar & Alt+Tab)
        // .with_always_on_top(true)// Keep window always on top
        
        // ── 5. Interaction Styles ──
        // .with_click_through(true)// Enable click-through
        // .with_no_activate(true)  // Do not take focus on click
}
```

---

### 4. Workflow for Adding a New Custom Window

To add a new window (e.g., `SettingsWindow`), follow these 4 steps:

#### Step 1: Write the Slint UI
Create `ui/settings-window/settings-window.slint`:
```slint
import { Titlebar } from "../app-window/titlebar.slint";

export component SettingsWindow inherits Window {
    title: "设置 (Settings)";
    in-out property <bool> is-mica-active: false;
    background: root.is-mica-active ? #00000000 : #202020;
    width: 600px;
    height: 400px;
    no-frame: true;

    VerticalLayout {
        Titlebar { title: root.title; }
        // Your UI content...
    }
}
```

#### Step 2: Create the Rust Window Module
Create `src/windows/settings/`:
- `attributes.rs`: Configure `WindowAttributes` (Mica, corners, etc.).
- `mod.rs`: Implement the `open` function using `CbtHookGuard::install` for zero-flicker injection, and call `center_component_on_active_monitor`.

```rust
use crate::SettingsWindow;
use crate::platform::{CbtHookGuard, WindowFrame, apply_mica_effect, center_component_on_active_monitor};
use std::sync::{Arc, Mutex};
use windows::Win32::Foundation::HWND;

pub fn open() -> Result<(), slint::PlatformError> {
    let frame_holder = Arc::new(Mutex::new(None));
    
    // 1. CBT Hook prevents startup flickering
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

    // 2. Instantiate component & frame
    let window = SettingsWindow::new()?;
    let frame = WindowFrame::new(&window, Arc::new(/* your TitlebarAdapter */));
    *frame_holder.lock().unwrap() = Some(frame.clone());

    // 3. Center & Visual Effects
    center_component_on_active_monitor(&window, 600.0, 400.0);
    apply_mica_effect(&window, |w| w.set_is_mica_active(true), hook_ok);

    // 4. Show
    window.show()?;
    Ok(())
}
```

#### Step 3: Register in `main.rs`
Export in `ui/` entry file and wire up the button click handler in `src/main.rs`!

---

### 5. Custom-Drawn Titlebar vs. DWM Native Buttons

| Feature | Custom-Drawn Titlebar (`app` / `custom_terminal`) | DWM Native Buttons (`native_terminal`) |
| :--- | :--- | :--- |
| **Appearance & Styling** | 100% Slint drawn, customizable icons, spacing, and palettes | Windows DWM native Min/Max/Close caption buttons |
| **Win11 Snap Layouts** | ✅ Supported (via `WM_NCHITTEST` returning `HTMAXBUTTON`) | ✅ Supported (native OS behavior) |
| **Inactive Fade Effect**| ✅ Supported (linked via `WM_NCACTIVATE`) | ✅ Supported (native OS behavior) |
| **Best For** | Custom-branded UI styling practice | Lightweight utilities sticking closely to OS-native look |

---

### 6. Advanced Escape Hatches

If you need to invoke custom Windows APIs or undocumented DWM attribute IDs:

```rust
WindowAttributes::new()
    // Escape Hatch 1: Pass raw DWM attribute ID and data directly to HWND
    .with_raw_dwm_attribute(1029 /* DWMWA_MICA_EFFECT */, 1u32)
    // Escape Hatch 2: Access raw HWND closure when window is captured by CBT Hook
    .with_custom_action(|hwnd: windows_sys::Win32::Foundation::HWND| {
        // Execute any Win32 APIs here
        println!("Raw HWND: {:?}", hwnd);
    })
```

---

## ⚠️ Known Notes & Troubleshooting

### 1. NVIDIA OpenGL Background Transparency
* **Symptom**: On NVIDIA GPU devices with OpenGL rendering backends (FemtoVG / Skia-OpenGL), transparent window regions may render as black solid background.
* **Root Cause**: NVIDIA driver's `OpenGL GDI Compatibility` defaults to "Auto" or "Prefer Performance", breaking DWM alpha transparency.
* **Solutions**:
  1. **Driver Settings**: In `NVIDIA Control Panel` -> `Manage 3D settings` -> `Global Settings`, change `OpenGL GDI Compatibility` to **"Prefer Compatibility"**;
  2. **Driver-agnostic Backends**: Use **`software`** or **`renderer-wgpu` (Direct3D 12 / Vulkan)** backend.

### 2. DWM Native Caption Buttons Interaction & Flicker Note
* **Interaction Details**: In native window mode, caption buttons are managed by Windows DWM. This template handles top-aligned `WM_NCCALCSIZE`, physical coordinate hit-testing, and `TrackMouseEvent` hover state tracking in `src/windows/native_terminal/borderless.rs`.
* **Known Limitation ((0, 0) DWM Flicker)**: Native mode windows may occasionally produce a brief DWM flicker at coordinates `(0, 0)` upon opening. For completely flicker-free appearance, the custom titlebar mode (`custom_terminal`) is recommended.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).