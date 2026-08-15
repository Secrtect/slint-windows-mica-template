# native_terminal_window 标题栏升级计划

## 背景

当前 `native_terminal_window` 使用 `SimpleBorderlessAdapter`（仅提供边缘拉伸和阴影，不处理标题栏按钮）。而 `custom_terminal_window` 已经使用完整的 CBT Hook 同步安装子类化 + `TitlebarAdapter` 驱动按钮状态。用户希望将 `native_terminal_window` 对齐到与 `custom_terminal_window` 一致的模式。

## 核心变更

将 `native_terminal_window` 从 `setup_borderless_simple()` + `SimpleBorderlessAdapter` 升级为 `Arc<Mutex<Option<WindowFrame>>>` + `NativeTerminalTitlebarAdapter`，让 CBT Hook 在 `HCBT_ACTIVATE` 时同步调用 `frame.apply_to_hwnd(hwnd)` 安装子类化和阴影。

## 实施步骤

### 步骤 1：新建 `src/native_terminal_window/controls.rs`

参考 `src/custom_terminal_window/controls.rs` 创建 `NativeTerminalTitlebarAdapter`，实现 `TitlebarAdapter<NativeTerminalWindow>` trait：
- `metrics()` 返回标题栏高度 36px、按钮宽度 46px，三个按钮全部可见
- `set_active()` / `set_hover()` / `set_pressed()` 驱动 Slint 属性

### 步骤 2：修改 `ui/native-terminal-window/titlebar.slint`

- 将 `show-minimize` / `show-maximize` 默认值从 `false` 改为 `true`
- 新增 `min-hover` / `min-pressed` / `max-hover` / `max-pressed` / `close-hover` / `close-pressed` 属性
- 将按钮背景和 stroke 的判定从 `TouchArea` 的 `pressed`/`has-hover` 改为 root 属性（`root.min-pressed` 等），与 custom_terminal_window 的 titlebar 一致

### 步骤 3：修改 `ui/native-terminal-window/native-terminal-window.slint`

- `background` 改为 `self.is-mica-active ? transparent : Palette.background`
- 新增标题栏按钮状态属性：`titlebar-min-hover`、`titlebar-min-pressed`、`titlebar-max-hover`、`titlebar-max-pressed`、`titlebar-close-hover`、`titlebar-close-pressed`、`titlebar-is-active`、`titlebar-maximized`
- 新增 `callback double-click()`
- 修改 `TerminalTitlebar` 绑定，传入所有状态属性和 `double-click` 回调

### 步骤 4：修改 `src/native_terminal_window/mod.rs`

- 添加 `pub mod controls;`
- 替换导入：移除 `TitlebarSetup`，新增 `WindowFrame`、`TitlebarButtons`、`NativeTerminalTitlebarAdapter`、`Arc`、`Mutex`、`HWND`
- 重写 `open()` 函数：
  - 新增 `frame_holder: Arc<Mutex<Option<WindowFrame<NativeTerminalWindow>>>>` 共享持有器
  - CBT Hook 回调中同时调用 `attrs.apply(hwnd_isize)` 和 `frame.apply_to_hwnd(hwnd)`
  - 用 `WindowFrame::new(&terminal, Arc::new(NativeTerminalTitlebarAdapter::new(buttons)))` 替换 `setup_borderless_simple()`
  - 将 frame 存入 `frame_holder`
  - 新增 `frame.on_maximized_changed` 和 `terminal.on_double_click` 回调

## 涉及文件

| 文件 | 操作 |
|------|------|
| `src/native_terminal_window/controls.rs` | **新建** |
| `src/native_terminal_window/mod.rs` | **修改** |
| `ui/native-terminal-window/native-terminal-window.slint` | **修改** |
| `ui/native-terminal-window/titlebar.slint` | **修改** |

## 验证方式

1. `cargo check` 确认编译通过
2. `cargo run` 运行程序，点击"打开原生终端窗口"按钮
3. 验证原生终端窗口：
   - 三个标题栏按钮（最小化、最大化、关闭）均可见且可点击
   - 按钮 hover 有高亮效果，pressed 有按压效果
   - 窗口失焦时标题栏文字/按钮变淡
   - 双击标题栏切换最大化/还原
   - 最大化时标题栏按钮图标切换为还原图标
   - 窗口边缘可拖拽缩放
   - Mica 材质生效
   - 窗口首帧无闪烁——阴影和子类化在窗口显示前就已就绪