# 移除 gpui-component 设计

## 1. 变更前状态

### 1.1 依赖关系

```
pmux
├── gpui (zed-industries/zed)
├── gpui-component (longbridge/gpui-component)  ← 已移除
├── gpui_platform (zed-industries/zed)           ← 保留，需 font-kit feature
└── anyhow
```

### 1.2 gpui-component 在代码中的使用点

- `Cargo.toml`: 依赖声明
- `main.rs`: `gpui_component::init(cx)` 和 `gpui_component::Root::new()`
- 源码 (`src/`): 无引用

## 2. 实际改动

### 2.1 Cargo.toml

移除 `gpui-component`，GPUI 依赖注释待启用时只需：

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
anyhow = "1.0"
```

注意：`font-kit` feature 是必须的，否则系统字体无法加载，文字不可见。

### 2.2 main.rs — 替换 Root 组件

`gpui_component::Root` 做了三件事（见 root.rs:434-451）：

1. `window.set_rem_size(cx.theme().font_size)` — 设置 rem 基准
2. `.font_family(cx.theme().font_family)` — 设置字体
3. `.text_color(cx.theme().foreground)` / `.bg(cx.theme().background)` — 设置前景/背景色

替换方案：在 `open_window` 回调中直接返回 AppRoot view，由 AppRoot 的 render 方法设置这些样式。

启用 GUI 时 main.rs 应为：

```rust
fn main() {
    gpui_platform::application().run(move |cx| {
        cx.spawn(async move |cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(800.0), px(600.0)),
                })),
                ..Default::default()
            };
            cx.open_window(options, |window, cx| {
                window.set_rem_size(px(16.0));
                cx.new(|cx| AppRoot::new(cx))
            })?;
            Ok::<_, anyhow::Error>(())
        }).detach();
    });
}
```

启用 GUI 时 AppRoot render 应为：

```rust
fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .id("root")
        .size_full()
        .font_family(".SystemUIFont")
        .text_color(rgb(0xcccccc))  // 浅色文字
        .bg(rgb(0x1e1e1e))          // 深色背景
        .child(...)
}
```

### 2.3 移除 gpui_component::init()

`init()` 注册了 theme、global_state、inspector 等全局状态。移除后不需要替代 — pmux 不使用这些功能。

### 2.4 工具链要求

需要 `rust-toolchain.toml` 固定 Rust 1.93，nightly 编译器会导致 `resvg 0.45.x`（gpui 间接依赖）编译失败。

```toml
[toolchain]
channel = "1.93"
```

## 3. 验证结果

- `cargo build`: 通过（gpui 依赖注释状态下）
- `cargo test`: 280 passed, 3 failed（3 个失败为已有问题，与本次变更无关）
- `Cargo.lock`: 不含 gpui-component 相关条目

## 4. 风险

| 风险 | 缓解 |
|------|------|
| 未来需要复杂 UI 组件 | 按需自建，或届时重新引入 |
| 深色/浅色主题切换 | 后续自建简单的颜色常量模块 |
