# Spec 2 设计：Sidebar + 终端渲染

## 目标

选择工作区后能看到终端、输入能透传。具体：
1. 选择工作区后创建/attach tmux session
2. 用 alacritty_terminal 渲染终端输出（带颜色）
3. 键盘输入透传到 tmux pane（焦点模式）
4. 左侧 Sidebar 显示 worktree 列表，点击切换

## 架构

```
AppRoot (GPUI Entity)
├── state: WorkspaceState
│   ├── worktrees: Vec<WorktreeInfo>
│   ├── selected_index: usize        ← 当前选中的 worktree
│   ├── pane_id: Option<String>      ← 对应的 tmux pane
│   ├── terminal_content: TerminalContent
│   └── input_focused: bool          ← 焦点模式
│
├── Sidebar (左 250px)
│   └── WorktreeItem × N  ← 点击 → 切换 selected_index
│
└── TerminalView (右侧剩余)
    └── 字符网格 (alacritty_terminal 解析后渲染)
```

### 启动流程

```
选择工作区
  → Session::ensure("sdlc-{repo}")
  → discover_worktrees(repo_path)
  → 为每个 worktree 找/创建对应 tmux pane
  → 选中 index=0（主分支）
  → 启动轮询线程（50ms）
```

## 终端渲染

**新增依赖**：`alacritty-terminal`（crates.io）

### 渲染流程

```
轮询线程（每 50ms）
  → tmux capture-pane -e -t <pane_id>   ← -e 保留 ANSI 转义
  → alacritty_terminal::Term::advance()  ← 解析 VT 序列
  → 提取字符网格 (cell.c, cell.fg, cell.bg)
  → hash 比较，有变化 → cx.notify()

GPUI render()
  → 遍历字符网格
  → 每个 cell 渲染为等宽字符，带前景/背景色
```

`TerminalView` 改为真正的 GPUI `Render` 组件，持有 `alacritty_terminal::Term` 实例。

## 输入透传

焦点模式切换：

```
TerminalView 区域点击
  → input_focused = true
  → GPUI 捕获所有 KeyDown

KeyDown 事件
  → 是 pmux 快捷键（⌘B 等）？→ 拦截
  → 否 → 转换为 tmux key string → send-keys -t <pane_id>

Sidebar 点击
  → input_focused = false
  → 切换 selected_index → 切换 pane_id
```

特殊键映射：Enter → "Enter"，Backspace → "BSpace"，方向键 → "Up/Down/Left/Right"，Escape → "Escape"，Tab → "Tab"。

## TDD 策略

GPUI 渲染本身不做单元测试（需要 GPU 上下文），其余逻辑全部 TDD。

| 模块 | 测试内容 |
|------|---------|
| `tmux/session` | `ensure()` 幂等性（mock tmux 命令） |
| `worktree` | 边界情况补充（detached HEAD、无 branch 字段） |
| `terminal_view` | `Term` 解析 ANSI 输出正确字符/颜色 |
| `input_handler` | 键值映射（Enter/Backspace/方向键/Escape） |
| `app_root` | 工作区加载后 worktrees 非空；切换 worktree 更新 pane_id |

## 不在 Spec 2 范围内

- tmux control mode（v0.5）
- 多 worktree 并排显示
- 状态检测（Spec 4）
- 通知系统（Spec 5）
