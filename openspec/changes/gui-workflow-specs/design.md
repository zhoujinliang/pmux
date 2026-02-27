## Overview

本设计实现 pmux GUI 工作流的 6 个核心规格，提供从启动到代码审查的完整用户体验。

## Architecture

### 组件层级
```
AppRoot
├── TopBar (Workspace Tabs)
│   ├── Sidebar Toggle (⌘B)
│   ├── Notification Bell
│   ├── Add Workspace Button
│   └── Workspace Tabs
├── Main Content
│   ├── Sidebar (条件渲染)
│   │   ├── Repo Name
│   │   ├── Worktree List
│   │   └── New Branch Button
│   └── Right Panel
│       ├── TabBar (Pane Tabs)
│       └── TerminalView
└── NotificationPanel (浮层)
```

### 数据流
1. **用户操作** → GPUI Event → AppRoot 处理
2. **Tmux 状态** → StatusPoller → UI 更新
3. **Git 操作** → Command 执行 → Sidebar 刷新

## Component Design

### 1. StartupWorkflow (规格 1)
- **触发**: 应用启动时检查 config.recent_workspace
- **空状态**: 显示居中的 Welcome 页面
- **有状态**: 恢复上次工作区，进入 WorkspaceView
- **交互**: ⌘N 打开新工作区，CTA 按钮打开文件选择器

### 2. SidebarWorktreeMgmt (规格 2, 4)
- **结构**: 固定宽度 200px，可折叠
- **条目**: 每个 worktree 显示分支名、状态图标、ahead 计数
- **右键菜单**: View Diff, Remove Worktree
- **底部**: + New Branch 按钮

### 3. TabbarPaneSwitching (规格 4)
- **位置**: 主内容区顶部
- **Tab**: 每个 pane 一个 tab，显示 🖥 图标和分支名
- **关闭**: × 按钮关闭 pane（确认对话框）
- **快捷键**: ⌘1-8 切换 tab

### 4. BranchWorktreeCreation (规格 3)
- **入口**: Sidebar 底部按钮或 ⌘⇧N
- **对话框**: 输入分支名，选择基于分支
- **流程**:
  ```
  git branch <name>
  git worktree add ../<repo>-<name> <name>
  tmux new-window -t <session> -n <name>
  Sidebar 刷新 → 自动切换
  ```

### 5. AgentStatusMonitoring (规格 5)
- **轮询**: StatusPoller 每 500ms capture-pane
- **检测**: detect_status() 识别关键词
- **展示**:
  - Sidebar: ● Running / ◐ Waiting / ✕ Error / ○ Idle
  - TopBar: 整体状态计数（🔔 2）
- **通知**: Error/Waiting 状态时触发系统通知

### 6. DiffViewIntegration (规格 6)
- **触发**: ⌘⇧R 或 Sidebar 右键 View Diff
- **实现**:
  ```
  tmux new-window -t <session> -n "review-<branch>"
  tmux send-keys "nvim -c 'DiffviewOpen main...HEAD'" C-m
  ```
- **展示**: 新增 review tab，渲染 nvim 终端
- **关闭**: ⌘W 或 nvim :q → 移除 review tab

## State Management

### AppRoot State
```rust
struct AppRoot {
    workspace_manager: WorkspaceManager,  // 多工作区管理
    status_poller: StatusPoller,          // 状态轮询器
    sidebar_visible: bool,                // Sidebar 显隐
    active_pane: Option<String>,          // 当前 pane
    notifications: Vec<Notification>,     // 通知列表
}
```

### WorkspaceManager
```rust
struct WorkspaceManager {
    tabs: Vec<WorkspaceTab>,      // 所有工作区
    active_index: Option<usize>,  // 当前激活
}
```

## Key Interactions

| 快捷键 | 功能 |
|--------|------|
| ⌘N | 打开新工作区 |
| ⌘⇧N | 新建 Branch + Worktree |
| ⌘B | 切换 Sidebar |
| ⌘1-8 | 切换 Pane Tab |
| ⌘W | 关闭当前 Tab/Pane |
| ⌘⇧R | 打开 Diff 视图 |
| ⌘I | 打开通知面板 |

## Error Handling

- **非 Git 目录**: 启动页显示红色错误提示
- **删除未推送**: 确认对话框警告
- **Tmux 失败**: 通知面板显示错误
