# Proposal: 规格 3 - TabBar 与多仓切换

## 背景

规格 2 已完成单仓的 Sidebar + TerminalView。现在需要支持同时管理多个仓库，每个仓库作为一个独立的 Tab。

## 目标

实现规格 3：在窗口顶部提供 TabBar，每个 Tab 代表一个独立的 git 仓库（或会话），支持多仓切换和管理。

## 范围

### 包含的功能

1. **TabBar 组件**：
   - 显示所有打开的仓库标签
   - 支持点击切换
   - 显示关闭按钮
   - 新 Tab 指示器

2. **多仓管理**：
   - 通过「打开仓库」新增 Tab
   - 为每个 Tab 创建独立 tmux session
   - 每个 Tab 有自己的 Sidebar + TerminalView 状态

3. **Tab 切换**：
   - 点击 Tab 切换当前仓库
   - 快捷键切换（⌘1-8）
   - 恢复对应 Sidebar / TerminalView 状态

4. **Tab 关闭**：
   - 点击 × 关闭 Tab
   - 可选删除 worktree（确认对话框）
   - 清理对应 tmux session

### 不包含的功能

1. **Agent 状态检测**（规格 4）
2. **通知系统**（规格 5）
3. **Diff 视图**（规格 6）
4. **拖拽排序 Tab**（v2）

## 技术方案

- **UI 框架**: gpui-component
- **状态管理**: WorkspaceManager 管理多个 WorkspaceState
- **tmux**: 每个 Tab = 独立 session
- **持久化**: 保存所有打开的仓库列表

## 用户体验流程

```
用户已有 1 个仓库打开
  ↓ 点击「📂 打开仓库」
弹出文件选择器
  ↓ 选择新的 Git 仓库
创建新 Tab
  ├─ 创建新 tmux session
  ├─ 发现 worktrees
  ├─ 初始化 Sidebar + TerminalView
  └─ 切换到新 Tab
显示 TabBar（2 个 tabs）
  ↓ 点击 Tab 1
切换到仓库 1，恢复其状态
  ↓ 点击 Tab 2
切换到仓库 2，恢复其状态
  ↓ 点击 Tab 2 的 ×
确认对话框
  ├─ 确认 → 关闭 Tab，清理 session
  └─ 取消 → 保持打开
```

## 成功标准

- 可以同时打开多个仓库（2-3 个）
- TabBar 清晰显示所有打开的仓库
- 点击 Tab 流畅切换，无延迟
- 每个 Tab 的状态独立（Sidebar 选中、Terminal 内容）
- 快捷键 ⌘1-8 快速切换 Tab
- 关闭 Tab 时正确清理资源
