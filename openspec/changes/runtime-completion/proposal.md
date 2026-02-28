## Why

根据 design.md 的成功标志，pmux Runtime 架构还有多个关键需求未实现。这些需求是完成 "AI Agent Runtime + 多终端工作台" 重构目标的必要条件。

当前已实现：
- ✅ PTY streaming 架构 (LocalPtyRuntime)
- ✅ Event Bus 基础框架
- ✅ AgentRuntime trait 定义
- ✅ 状态发布与订阅机制

但以下核心功能仍缺失或未完成：
- ❌ Pane 分屏功能
- ❌ Diff/Review 视图
- ❌ TUI 应用支持 (vim/claude code 光标问题)
- ❌ Session 持久化 (关闭 UI 后进程终止)
- ❌ Session 恢复 (recover)
- ❌ 基于进程生命周期的状态检测 (仍在轮询)

## What Changes

1. **实现 `split_pane` 功能** - 支持 ⌘D/⌘⇧D 分屏，多 pane 并行工作
2. **修复 TUI 光标支持** - 使 vim、Claude Code 等全屏 TUI 应用正常工作
3. **实现 `open_diff` / `open_review`** - 支持 ⌘⇧D 和 ⌘⇧R 快捷键查看 diff
4. **实现 tmux backend** - 解决 session 持久化问题，关闭 UI 后进程继续运行
5. **实现 `recover()`** - 重启 pmux 后能恢复之前的 session
6. **移除状态轮询** - 改为基于进程生命周期的事件驱动状态检测

## Capabilities

### New Capabilities

- `pane-split`: 垂直/水平分屏，支持多 pane 并行工作
- `diff-view`: 打开 diff 视图查看工作区变更
- `review-mode`: 打开 review 模式进行代码审查
- `session-persistence`: 关闭 UI 后保持进程运行
- `session-recovery`: 重启后恢复之前的 session 状态
- `tui-support`: 完整的 TUI 应用支持 (vim, neovim, claude code)

### Modified Capabilities

- `agent-status-detection`: 从文本轮询改为进程生命周期事件驱动
- `backend-selection`: 支持 local_pty / tmux backend 切换

## Impact

- **UI Layer**: 修复 terminal_view 光标渲染，支持 TUI 应用
- **Runtime Layer**: 完善 AgentRuntime trait 实现，添加缺失方法
- **Backend Layer**: 新增 tmux backend 实现 session 持久化
- **State Management**: 实现完整的 recover() 逻辑
- **Event System**: 优化状态检测，移除轮询循环
