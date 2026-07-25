[English](README.md) | [简体中文](README_CN.md)

# slint-windows-mica-template

A Fluent Design template for Slint on Windows, featuring **Mica background effect** and **multi-monitor centered popups**! ✨

---

## 📖 About The Project

I first discovered Slint through a recommendation from Gemini and was immediately drawn to how lightweight and elegant it is.

However, when using it in actual projects, I felt a few minor gaps: it didn't offer out-of-the-box centered popups, Mica material support, or Win11-style borderless window components.

Special thanks to [@Drew-Chase](https://github.com/Drew-Chase) for creating [slint_borderless_windows](https://github.com/Drew-Chase/slint_borderless_windows)! To fix a few minor issues more conveniently, I merged their code directly into this project. Big thanks for the open-source contribution! 🙏

---

## ✨ Features

1. **Multi-Monitor Smart Centered Popups**: Uses the `winit` backend to retrieve the window handle natively. Popups launch centered on the monitor where your cursor currently resides, completely eliminating window position flickering.
2. **Mica Visual Effect**: Implements a Windows 11-style Mica background (Acrylic is not included yet).
3. **Fixed Icon Rendering**: Resolved an issue in the original borderless component where the windowed/restore icon showed dark color blocks.
4. **Complete Window Messaging**: Added extra window message handling for seamless toggling between windowed and fullscreen modes.
5. **Smoother Titlebar Experience**: Tweaked the `titlebar` logic to eliminate an extra click flicker during double-click maximize/restore actions.

---

## ⚠️ Known Issues & Lessons Learned

### 1. NVIDIA OpenGL Transparent Background Turning Black
* **The Issue**: On my GTX 1650, using FemtoVG's OpenGL backend caused transparent window areas to render completely black. *(Honestly, this drove me crazy for 2 or 3 days! 😭)*
* **Root Cause**: `NVIDIA Control Panel` -> `Manage 3D settings` -> `Global Settings` -> `OpenGL GDI compatibility` (When set to "Auto" or "Prefer performance", transparency breaks).
* **Fix**: Change `OpenGL GDI compatibility` to **"Prefer compatibility"**.
* **Default Setting**: Since I don't have AMD or Intel GPUs on hand for testing, the default rendering backend is set to **`software`** to guarantee out-of-the-box compatibility for everyone.

---

## 🚀 How to Use

1. Clone this repository.
2. **Remember to update `name = "..."` in `Cargo.toml` to your own project name!**
3. Run `cargo run`.

---

## 📦 Releases & False Positive Warnings

This is also my first time setting up a CI/CD pipeline to automatically build and publish `.exe` binaries.

Because this project is newly released and does not carry an expensive digital code-signing certificate, Windows Defender or other antivirus tools will likely flag the pre-compiled `.exe` in Releases as a false positive.

* If you need to test the pre-compiled `.exe` from Releases, you might need to temporarily disable Defender's Real-time Protection or manually choose to allow/keep the file.
* **Strongly Recommended**: Simply clone this repository and execute `cargo run` locally (local builds will never trigger false positive warnings).

Hope this small template helps you out!