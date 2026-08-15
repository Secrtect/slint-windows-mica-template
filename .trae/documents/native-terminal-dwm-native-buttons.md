# native_terminal_window DWM 原生按钮计划

## 背景

之前将 native_terminal_window 升级为与 custom_terminal_window 一致的 Slint 自绘按钮方案。但用户实际想要的是 test 文件夹那种效果——**右上角三个按钮由 DWM 系统原生绘制**，而非 Slint 自绘。

## 核心差异

| 特性 | custom_terminal_window（当前） | native_terminal_window（目标） |
|------|-------------------------------|-------------------------------|
| 按钮渲染 | Slint Path 自绘 | DWM 系统原生绘制 |
| 按钮状态 | Win32 子类化驱动 hover/pressed | DWM 自动处理 |
| 标题栏 | Slint 组件含按钮 | Slint 组件仅拖拽区（图标+文字），无按钮 |
| WM_NCCALCSIZE | 最大化时适配 work area | 扩展客户区覆盖标题栏区域 |
| WM_NCHITTEST | 返回具体按钮码 | 标题栏区域返回 HTCAPTION |

## 实施步骤

### 步骤 1：还原 Slint UI 文件

将 titlebar.slint 和 native-terminal-window.slint 还原为简洁版本（无按钮状态属性，仅保留拖拽区）。

**titlebar.slint**：移除所有按钮（minimize/maximize/close），只保留左侧拖拽区（图标 + 标题）。标题栏高度保持 36px 作为拖拽区域。

**native-terminal-window.slint**：移除 titlebar-* 状态属性和 callback double-click，保持简洁。

### 步骤 2：修改 native_terminal_window/attributes.rs

通过 `with_raw_style()` 添加 `WS_CAPTION` 样式：

```rust
use windows_sys::Win32::UI::WindowsAndMessaging::WS_CAPTION;

pub fn get_attributes() -> WindowAttributes {
    WindowAttributes::new()
        .with_mica()
        .with_dark_mode(is_system_dark_mode())
        .with_corner(CornerPreference::Round)
        .with_raw_style(WS_CAPTION, 0)  // 添加 WS_CAPTION 让 DWM 绘制原生按钮
}
```

### 步骤 3：新建 native_terminal_window/borderless.rs

创建专用于 DWM 原生按钮的简化子类化 proc，参考 test 文件夹的 `WM_NCCALCSIZE` 和 `WM_NCHITTEST` 逻辑：

- **WM_NCCALCSIZE**：未最大化时，调用 DefWindowProcW 计算标准客户区后将 top 恢复为窗口顶部，使客户区覆盖标题栏区域
- **WM_NCHITTEST**：标题栏区域（y < 36px）返回 HTCAPTION，让 DWM 绘制原生按钮；按钮区域使用 DWMWA_CAPTION_BUTTON_BOUNDS 判定并放行；其余为 HTCLIENT；边缘区域保留拉伸判定
- **1px 阴影修复**：DwmExtendFrameIntoClientArea
- **WM_NCDESTROY**：注销子类化

这个结构体提供 `new()`、`apply_to_hwnd()`、`drag()`、`toggle_maximized()` 方法。

### 步骤 4：修改 native_terminal_window/mod.rs

- 移除 `controls` 模块引用，改用 `borderless` 模块
- 移除 `NativeTerminalTitlebarAdapter`、`TitlebarButtons`、`WindowFrame` 导入
- 改用 `NativeCaptionFrame`（新 borderless 类型）
- CBT Hook 回调中调用 `frame.apply_to_hwnd(hwnd)` 安装简化子类化
- 保留拖拽、关闭、复制日志等回调

### 步骤 5：删除不再需要的文件

- 删除 `src/native_terminal_window/controls.rs`

## 涉及文件

| 文件 | 操作 |
|------|------|
| `ui/native-terminal-window/titlebar.slint` | **修改** - 移除所有按钮，仅保留拖拽区 |
| `ui/native-terminal-window/native-terminal-window.slint` | **修改** - 移除 titlebar-* 状态属性 |
| `src/native_terminal_window/attributes.rs` | **修改** - 添加 WS_CAPTION |
| `src/native_terminal_window/borderless.rs` | **新建** - DWM 原生按钮子类化 |
| `src/native_terminal_window/mod.rs` | **修改** - 使用新 borderless 类型 |
| `src/native_terminal_window/controls.rs` | **删除** |

## 验证方式

1. `cargo check` 确认编译通过
2. `cargo run` 运行，点击"打开原生终端窗口"
3. 验证：右上角三个按钮为 **DWM 系统原生按钮**（非 Slint 自绘）
4. 验证：按钮 hover 有系统原生高亮效果，pressed 有原生反馈
5. 验证：标题栏区域可拖拽，双击切换最大化
6. 验证：Mica 材质在标题栏区域透明透出
7. 验证：窗口边缘可拖拽缩放