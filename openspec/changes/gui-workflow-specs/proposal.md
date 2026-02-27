## Why

pmux 目前缺少完整的 GUI 工作流实现，用户无法通过图形界面完成从启动、选择工作区到管理多个 worktree 的完整流程。本变更旨在实现设计文档中定义的 6 个核心规格，提供完整的 GUI 操作体验，让用户无需 CLI 即可管理 AI Agent 多分支开发。

## What Changes

- **规格 1**：实现启动页与工作区选择流程，支持最近工作区列表
- **规格 2**：实现单仓主分支 + Sidebar 布局，集成 tmux session 管理
- **规格 3**：实现通过 GUI 新建 Branch + Worktree 功能
- **规格 4**：实现多 Worktree 管理与 TabBar，支持切换和删除
- **规格 5**：实现 Agent 状态检测与可视化展示（Running/Waiting/Error/Idle）
- **规格 6**：实现 Diff 视图集成，通过 nvim diffview 进行代码审查

## Capabilities

### New Capabilities
- `startup-workflow`: 应用启动流程和工作区选择
- `sidebar-worktree-mgmt`: Sidebar 组件和 worktree 管理
- `tabbar-pane-switching`: TabBar 组件和 pane 切换
- `branch-worktree-creation`: 通过 GUI 创建分支和 worktree
- `agent-status-monitoring`: Agent 状态检测和实时监控
- `diff-view-integration`: Diff 视图集成和代码审查

### Modified Capabilities
- 无现有规格需要修改

## Impact

- **UI 层**: 新增/完善 TerminalView、Sidebar、TopBar、TabBar、NotificationPanel 组件
- **Tmux 集成**: 扩展 tmux session/window/pane 管理能力
- **Git 集成**: 增加 worktree 和分支管理功能
- **状态管理**: 实现状态轮询和实时更新机制
