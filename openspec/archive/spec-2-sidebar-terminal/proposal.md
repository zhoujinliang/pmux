# Proposal: 规格 2 - 单仓主分支 + Sidebar

## 背景

规格 1 已完成工作区选择功能。现在需要实现核心功能：显示单个仓库的主分支，并提供 Sidebar 展示 worktree 列表。

## 目标

实现规格 2：在用户选择了 git 仓库后，创建/attach tmux session，显示 Sidebar 和 TerminalView。

## 范围

### 包含的功能

1. **tmux Session 管理**：
   - 创建新的 tmux session（如果不存在）
   - Attach 到现有 session
   - Session 命名规范：`sdlc-<repo-name>`

2. **Sidebar 组件**：
   - 显示当前仓库的所有 worktree
   - 显示主分支（main/master）
   - 支持选中高亮
   - 显示分支状态信息

3. **TerminalView 组件**：
   - 渲染 tmux pane 的输出
   - 支持键盘输入透传
   - 基础终端仿真

4. **Worktree 发现**：
   - 自动发现仓库中的所有 worktree
   - 为主分支创建默认 pane

### 不包含的功能

1. **多仓 TabBar**（规格 3）
2. **Agent 状态检测**（规格 4）
3. **通知系统**（规格 5）
4. **Diff 视图**（规格 6）

## 技术方案

- **tmux 集成**：使用 `tmux` crate 或直接调用 tmux 命令
- **终端渲染**：使用 `alacritty_terminal` 进行 VT 解析
- **UI 框架**：gpui-component
- **布局**：Sidebar (左侧) + TerminalView (右侧)

## 用户体验流程

```
用户选择工作区（规格 1）
  ↓
检查 tmux session 是否存在
  ├─ 不存在 → 创建 session + window + pane
  └─ 存在 → attach 到现有 session
  ↓
发现所有 worktrees
  ↓
为每个 worktree 创建/关联 pane
  ↓
显示 Sidebar（worktree 列表）
显示 TerminalView（主分支 pane 输出）
```

## 成功标准

- 选择工作区后自动创建/attach tmux session
- Sidebar 显示所有 worktree（包括主分支）
- TerminalView 渲染主分支的终端输出
- 可以在 Sidebar 中选择不同 worktree
- 键盘输入可以透传到 tmux pane
