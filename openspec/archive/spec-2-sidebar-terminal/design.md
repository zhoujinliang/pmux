# 规格 2 设计：单仓主分支 + Sidebar

## 1. 架构设计

### 1.1 组件层次

```
AppRoot (GPUI Window)
├── State: WorkspaceState
│   ├── repo_path: PathBuf
│   ├── tmux_session: String
│   ├── worktrees: Vec<WorktreeInfo>
│   └── selected_worktree: Option<String>
│
└── Render:
    ├── Sidebar (左侧, 固定宽度 250px)
    │   ├── WorktreeList
    │   │   ├── WorktreeItem (main)
    │   │   ├── WorktreeItem (feat-xxx)
    │   │   └── ...
    │   └── [+ New Branch] 按钮
    │
    └── TerminalView (右侧, 剩余空间)
        ├── TabBar (可选, v2)
        └── TerminalContent
            └── alacritty_terminal::Term output
```

### 1.2 数据流

```
AppRoot::init()
  → load_workspace_config()
  → tmux::ensure_session()
  → worktree::discover()
  → tmux::create_panes_for_worktrees()
  → start_terminal_poll()
```

## 2. tmux 集成设计

### 2.1 Session 管理

```rust
pub struct TmuxSession {
    name: String,
    window_name: String,
}

impl TmuxSession {
    /// 创建或 attach 到 session
    pub fn ensure(name: &str) -> Result<Self, TmuxError>;
    
    /// 检查 session 是否存在
    pub fn exists(name: &str) -> bool;
    
    /// 创建新窗口
    pub fn create_window(&self, name: &str) -> Result<WindowId, TmuxError>;
    
    /// 创建 pane
    pub fn create_pane(&self, window: WindowId, path: &Path) -> Result<PaneId, TmuxError>;
    
    /// 获取所有 pane
    pub fn list_panes(&self) -> Result<Vec<PaneInfo>, TmuxError>;
}
```

### 2.2 Pane 管理

```rust
pub struct PaneInfo {
    pub id: String,           // e.g., "sdlc-myproject:0.0"
    pub window_id: String,
    pub title: String,
    pub current_path: PathBuf,
}

pub fn capture_pane(pane_id: &str) -> Result<String, TmuxError>;
pub fn send_keys(pane_id: &str, keys: &str) -> Result<(), TmuxError>;
```

## 3. Sidebar 设计

### 3.1 布局

```
┌─────────────────┐
│ 📁 myproject    │  ← 仓库名称（标题）
├─────────────────┤
│ ● main          │  ← 主分支（选中状态）
│   ~/work/...    │  ← 路径缩写
│                 │
│ ○ feat-auth     │  ← 其他 worktree
│   feat/auth · +2│  ← 分支名 + ahead 计数
│                 │
│ ○ fix-bug       │
│   fix/bug · +1  │
│                 │
├─────────────────┤
│ [+ New Branch]  │  ← 新建分支按钮
└─────────────────┘
```

### 3.2 WorktreeItem 组件

```rust
pub struct WorktreeItemProps {
    pub branch_name: String,
    pub path: PathBuf,
    pub ahead: usize,
    pub behind: usize,
    pub is_selected: bool,
    pub on_click: Callback,
}
```

## 4. TerminalView 设计

### 4.1 布局

```
┌──────────────────────────────────────┐
│ 🖥 main                    [_][□][×] │  ← 标题栏（可选）
├──────────────────────────────────────┤
│                                      │
│  $ git status                        │
│  On branch main                      │
│                                      │
│  $ _                                │  ← 光标
│                                      │
│                                      │
└──────────────────────────────────────┘
```

### 4.2 终端渲染流程

```
tmux pane
  → tmux capture-pane -p -t <pane_id>
  → alacritty_terminal::Term::parse_bytes(content)
  → GPUI 渲染（字符网格）
```

### 4.3 输入处理

```
键盘事件
  → GPUI 事件循环
  → 应用级快捷键？→ 拦截
  → 否 → tmux send-keys -t <pane_id> <key>
```

## 5. Worktree 发现

### 5.1 Git Worktree 命令

```bash
# 列出所有 worktree
git worktree list --porcelain

# 输出格式：
# worktree /path/to/main
# HEAD abc123...
# branch refs/heads/main
#
# worktree /path/to/feat-x
# HEAD def456...
# branch refs/heads/feat-x
```

### 5.2 Rust 实现

```rust
pub fn discover_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>, GitError> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()?;
    
    parse_worktree_list(&String::from_utf8_lossy(&output.stdout))
}
```

## 6. 状态管理

### 6.1 WorkspaceState

```rust
pub struct WorkspaceState {
    pub repo_path: PathBuf,
    pub repo_name: String,
    pub tmux_session: String,
    pub worktrees: Vec<WorktreeInfo>,
    pub selected_worktree: Option<usize>,
    pub terminal_content: HashMap<String, TerminalContent>, // pane_id -> content
}
```

### 6.2 状态流转

```
Initial
  ↓
LoadingWorkspace
  ↓
WorkspaceReady { worktrees, selected_index: 0 }
  ↓ User selects worktree
WorkspaceReady { worktrees, selected_index: N }
```

## 7. 性能考虑

### 7.1 终端轮询

- **频率**: 50ms（平衡响应性和 CPU 使用）
- **优化**: 只轮询当前可见的 pane
- **增量更新**: 比较内容 hash，变化时才重绘

### 7.2 大仓库处理

- Worktree 列表懒加载
- 虚拟滚动（如果 worktree 数量 > 20）

## 8. 错误处理

| 场景 | 处理 |
|------|------|
| tmux 未安装 | 显示错误提示，提供安装指南 |
| session 创建失败 | 重试一次，然后显示错误 |
| worktree 发现失败 | 显示警告，仅使用主分支 |
| pane 捕获失败 | 显示占位符，后台重试 |

## 9. 测试策略

### 9.1 单元测试

- `tmux::Session` 管理
- `worktree::discover()` 解析
- `TerminalView` 渲染逻辑

### 9.2 集成测试

- 完整启动流程（mock tmux）
- Worktree 切换
- 键盘输入透传

### 9.3 手动测试

- 真实 tmux session 创建/attach
- 大仓库性能测试
- 长时间运行稳定性
