# Settings 界面实现计划

基于 `docs/plans/2026-02-28-settings-remote-channels-brainstorm.md`。

## P0: Settings Overlay 骨架

**目标**：菜单 Preferences… 打开 Settings overlay，可关闭，空面板。

### 任务 0.1：OpenSettings 通知 AppRoot

- `src/main.rs`：`open_settings` 需获取当前窗口、向 AppRoot 发送打开 Settings 信号
- GPUI 方式：`cx.dispatch_action(OpenSettings)` 会广播，但需确保 AppRoot 订阅。或使用 `window.update()` 更新 AppRoot 的 `show_settings: true`。查 gpui 文档：可用 `cx.dispatch_action` 配合 `on_action`，AppRoot 需实现 `actions!(OpenSettings)` 并 `cx.on_action(open_settings_handler)`。但 actions 在 main 注册的是全局 handler。要让 AppRoot 响应，需在 AppRoot 内 `cx.on_action(|_: &OpenSettings, cx| { ... })` 或类似。查现有代码：`open_settings` 在 main 的 `cx.on_action` 注册，只 println。改为：获取 active window，调用 `window.update(cx, |app_root: &mut AppRoot, cx| { app_root.show_settings = true; cx.notify(); })` 或通过 dispatch。

- 更简单：`open_settings` 中 `cx.activate()` 激活窗口，然后需要一种方式更新 AppRoot。`cx.focus_window()` 聚焦。`Window::update` 可以更新 root。所以：在 open_settings 里，遍历 `cx.windows()` 找到有 AppRoot 的，调用 `window.update(cx, |root: &mut AppRoot, cx| { root.show_settings = true; cx.notify(); })`。

### 任务 0.2：AppRoot 添加 show_settings 状态

- `src/ui/app_root.rs`：
  - 添加 `show_settings: bool` 字段，默认 false
  - 在 `new()` / `Default` 里初始化

### 任务 0.3：注册 AppRoot 对 OpenSettings 的响应

- 在 AppRoot 的 `init` 或 `mount` 或首次 render 的 `on_action`：订阅 `OpenSettings`，设置 `show_settings = true`，`cx.notify()`。
- 或：main 的 `open_settings` 直接 `window.update` 更新 AppRoot（见 0.1）。这样 AppRoot 不需要单独订阅。

### 任务 0.4：Settings Overlay 渲染

- 参考 `new_branch_dialog_ui.rs`：`when(self.is_open(), |el| { modal_overlay ... })`
- 在 AppRoot 的 render 中：`.when(self.show_settings, |el| { settings_overlay })`
-  overlay 结构：
  - 半透明遮罩 `bg-black/50` 或类似，`absolute` 覆盖全屏
  - 居中面板：白/深色背景，圆角，固定最大宽 560px，标题 "Settings"，[×] Close 按钮
  - 点击遮罩或 Close 关闭：`show_settings = false`，`cx.notify()`

### 任务 0.5：Close 按钮与遮罩点击

- Close 按钮 `on_click`：`this.show_settings = false; cx.notify();`
- 遮罩 `on_click`：同上（点击面板内部不冒泡关闭）

### 文件变更清单

- `src/main.rs`：修改 `open_settings`，找到窗口并 update AppRoot
- `src/ui/app_root.rs`：`show_settings` 字段，overlay 渲染，close 逻辑
- `src/ui/mod.rs`：若新建 `settings_ui.rs` 则添加 mod

**可选**：新建 `src/ui/settings_ui.rs` 作为 Settings 组件，AppRoot 只 `when(show_settings, settings_panel)`

---

## P1: Remote Channels 主界面（后续）

- 在 Settings 面板内渲染三个 channel 卡片：Discord、KOOK、飞书
- 每卡片：名称、enabled toggle、状态（已配置/未配置）、占位「配置」按钮
- 从 Config + Secrets 读取当前状态

---

## 参考代码

- `src/ui/new_branch_dialog_ui.rs`：modal overlay 结构、关闭回调
- `src/ui/delete_worktree_dialog_ui.rs`：同上
- `src/main.rs`：`open_settings`、菜单注册
- `src/ui/app_root.rs`：render 结构、`diff_overlay_open` 的 overlay 渲染
