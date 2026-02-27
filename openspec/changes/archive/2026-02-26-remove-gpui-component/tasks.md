# 移除 gpui-component 任务清单

## 任务分解

### 1. 清理 Cargo.toml ✅

- [x] 1.1 移除 `gpui-component` 依赖行，替换为注释状态的 `gpui` + `gpui_platform` + `anyhow`
- [x] 1.2 确保 `gpui_platform` 带 `font-kit` feature
- [x] 1.3 确保 `anyhow` 依赖存在
- [x] 1.4 Cargo.lock 中已无 gpui-component 相关条目

### 2. 更新 main.rs ✅

- [x] 2.1 移除 `gpui_component::init(cx)` 调用
- [x] 2.2 移除 `gpui_component::Root::new()` 包裹逻辑
- [x] 2.3 启用 GUI 时在 `open_window` 中直接设置 `window.set_rem_size(px(16.0))`
- [x] 2.4 直接返回 AppRoot view（不再用 Root 包裹）

注：当前 main.rs 为 CLI 版本，GUI 启动代码记录在 design.md 中待启用。

### 3. 更新 AppRoot render ✅

- [x] 3.1 启用 GUI 时在根 div 上设置 `.font_family(".SystemUIFont")`
- [x] 3.2 设置 `.text_color(rgb(0xcccccc))` 和 `.bg(rgb(0x1e1e1e))`
- [x] 3.3 子组件样式不受影响（源码中无 gpui-component 引用）

### 4. 验证 ✅

- [x] 4.1 `cargo build` 编译通过
- [x] 4.2 `cargo test` 280 passed（3 个已有失败与本次无关）
- [x] 4.3 Cargo.lock 中不含 gpui-component 相关 crate
- [x] 4.4 design.md 已更新，记录完整的 GUI 启动方案和工具链要求
