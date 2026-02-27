# Proposal: 移除 gpui-component 依赖

## 背景

pmux 当前依赖 `gpui-component`（longbridge/gpui-component），但项目实际用不上它提供的高级 UI 组件（Button、Dialog、Select、Table、Theme 系统、i18n 等）。pmux 的核心 UI 是终端渲染 + 简单布局（Sidebar、TabBar、TopBar），用 gpui 原语就能搞定。

gpui-component 带来的问题：
- 强制依赖 `gpui_platform` 的 `font-kit` feature，否则系统字体无法加载
- 强制用 `Root` 组件包裹 view
- 拉入大量无用依赖（schemars、rust-i18n、color picker、dock、sheet 等），增加编译时间
- API 与 gpui 版本耦合，升级时容易出问题

## 目标

移除 `gpui-component` 依赖，仅保留 `gpui` + `gpui_platform`（带 `font-kit` feature）。

## 范围

### 包含

1. 从 Cargo.toml 移除 `gpui-component`
2. 移除代码中所有 `gpui_component::` 引用（当前源码中已无引用）
3. 替换 `gpui_component::Root` 包裹逻辑，直接用 gpui 原语设置字体和主题色
4. 更新 main.rs 的应用启动流程
5. 更新 openspec 文档中对 gpui-component 的引用

### 不包含

1. UI 功能变更（视觉效果保持不变）
2. 新增 UI 组件
3. 主题系统实现（后续按需自建简单版本）

## 技术方案

- 应用入口继续使用 `gpui_platform::application()` 启动
- 窗口根 view 直接设置 `font_family`、`text_color`、`bg` 等样式，替代 `Root` 组件的功能
- 系统字体加载通过 `gpui_platform` 的 `font-kit` feature 保证

## 成功标准

- `cargo build` 编译通过，无 gpui-component 相关依赖
- 应用启动后 UI 显示正常（文字可见、按钮可点击）
- 编译时间有所缩短
