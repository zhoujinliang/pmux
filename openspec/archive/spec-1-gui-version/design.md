# 规格 1 GUI 版本设计

## 1. 架构设计

### 1.1 组件层次

```
App (GPUI App)
└── AppRoot (ViewComponent)
    ├── State: AppState (Model)
    │   ├── workspace_path: Option<PathBuf>
    │   └── error_message: Option<String>
    │
    └── Render:
        ├── if workspace_path.is_none() → StartupPage
        └── if workspace_path.is_some() → WorkspaceView
```

### 1.2 核心组件

#### AppRoot
- 应用根组件，管理全局状态
- 处理窗口事件和生命周期
- 协调子组件之间的通信

#### StartupPage
- 居中显示的启动页面
- 包含标题、描述、CTA 按钮
- 显示错误消息（如果有）

#### WorkspaceView
- 显示已选择的工作区
- 简单的确认界面
- 提供"更换工作区"选项

## 2. UI 设计

### 2.1 启动页布局

```
┌─────────────────────────────────────────┐
│              pmux Window                │
│                                         │
│                                         │
│         ┌───────────────────┐          │
│         │                   │          │
│         │   Welcome to      │          │
│         │   pmux            │          │
│         │                   │          │
│         │   Select a Git    │          │
│         │   repository to   │          │
│         │   manage your     │          │
│         │   AI agents       │          │
│         │                   │          │
│         │   [📁 Select      │          │
│         │    Workspace]     │          │
│         │                   │          │
│         └───────────────────┘          │
│                                         │
│         [Error message if any]          │
│                                         │
└─────────────────────────────────────────┘
```

### 2.2 视觉规范

- **背景色**: 深色主题 (#1e1e1e 或类似)
- **卡片背景**: 稍浅的深色 (#252526)
- **主按钮**: 品牌色（蓝色系）
- **文字**: 浅色 (#cccccc) 用于正文，白色用于标题
- **错误文字**: 红色 (#f48771)
- **圆角**: 8px 用于卡片，4px 用于按钮
- **间距**: 24px 大间距，16px 中间距，8px 小间距

## 3. 状态管理

### 3.1 AppState Model

```rust
#[derive(Clone)]
pub struct AppState {
    pub workspace_path: Option<PathBuf>,
    pub error_message: Option<String>,
    pub is_loading: bool,
}
```

### 3.2 状态流转

```
Initial State
    ↓
StartupPage displayed
    ↓ User clicks "Select Workspace"
File picker opens
    ↓ User selects path
Validate Git repo
    ├─ Valid → Save config → Show WorkspaceView
    └─ Invalid → Show error → Stay on StartupPage
```

## 4. 事件处理

### 4.1 用户交互

| 动作 | 事件 | 处理 |
|------|------|------|
| 点击按钮 | on_click | 打开文件选择器 |
| 选择文件 | pick_folder | 验证并更新状态 |
| 窗口关闭 | on_close | 保存状态（如果需要） |

### 4.2 异步操作

文件选择器是模态对话框，会阻塞直到用户选择。这简化了我们的实现，不需要复杂的异步状态管理。

## 5. 依赖项

### 5.1 Cargo.toml

```toml
[dependencies]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }
rfd = "0.14"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dirs = "5.0"
```

### 5.2 模块结构

```
src/
├── main.rs              # GPUI app entry
├── lib.rs               # Library exports
├── app.rs               # App state and logic
├── config.rs            # Config management (reuse existing)
├── git_utils.rs         # Git validation (reuse existing)
├── file_selector.rs     # File picker (reuse existing)
└── ui/
    ├── mod.rs           # UI module exports
    ├── app_root.rs      # Root component
    ├── startup_page.rs  # Startup page component
    └── workspace_view.rs # Workspace view component
```

## 6. 实现步骤

1. 更新 Cargo.toml 添加 gpui dependencies
2. 创建 ui/app_root.rs - 根组件
3. 创建 ui/startup_page.rs - 启动页
4. 创建 ui/workspace_view.rs - 工作区视图
5. 修改 main.rs - GPUI 应用入口
6. 测试 GUI 功能
7. 移除旧的 CLI main.rs（或保留为备选）

## 7. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| gpui 编译时间长 | 高 | 使用 release 模式缓存，文档说明 |
| gpui API 变化 | 中 | 锁定特定 commit，定期更新 |
| 跨平台差异 | 中 | 在 macOS/Linux/Windows 上测试 |
