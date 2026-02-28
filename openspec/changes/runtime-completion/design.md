# Runtime 完善设计文档

## 1. 架构目标

完成 design.md 定义的成功标志：

```
- [ ] 关闭 pmux UI，agent 继续运行
- [ ] vim / TUI 在 pmux 内完全正常
- [ ] 无 polling loop
- [ ] UI 不包含 tmux 调用
- [ ] 新 backend 可在不改 UI 情况下接入
```

## 2. 当前状态分析

### 2.1 已实现

| 组件 | 状态 | 说明 |
|------|------|------|
| AgentRuntime trait | ✅ | 已定义核心接口 |
| EventBus | ✅ | flume-based 发布订阅 |
| LocalPtyRuntime | ✅ | 基础 PTY 后端 |
| PTY Streaming | ✅ | pipe-pane 模式 |
| xterm escape | ✅ | 键盘输入转换 |
| TermBridge | ✅ | alacritty_terminal 集成 |

### 2.2 未实现

| 组件 | 状态 | 影响 |
|------|------|------|
| split_pane | ❌ | 无法分屏 |
| open_diff | ❌ | 无法查看 diff |
| open_review | ❌ | 无法 review |
| TUI 光标 | ❌ | vim/claude code 显示异常 |
| tmux backend | ❌ | 关闭 UI 进程终止 |
| recover() | ❌ | 重启无法恢复 session |
| 进程状态检测 | ❌ | 仍在轮询文本 |

## 3. 详细设计

### 3.1 Pane 分屏 (P0)

**LocalPtyRuntime 实现方案：**

由于 local PTY 是单 pane 设计，分屏需要在 Runtime 上层实现：

```rust
// 方案：一个 Agent 管理多个 LocalPtyRuntime
pub struct LocalPtyAgent {
    worktree_path: PathBuf,
    panes: Vec<Arc<LocalPtyRuntime>>, // 每个 pane 一个 PTY
    active_pane: usize,
}

impl AgentRuntime for LocalPtyAgent {
    fn split_pane(&self, pane_id: &PaneId, vertical: bool) -> Result<PaneId, RuntimeError> {
        // 创建新的 LocalPtyRuntime
        let new_runtime = LocalPtyRuntime::new(&self.worktree_path, cols, rows)?;
        let new_pane_id = new_runtime.pane_id().to_string();
        self.panes.push(Arc::new(new_runtime));
        Ok(new_pane_id)
    }
}
```

**UI 集成：**
- 保持现有 `SplitPaneContainer` 和 `SplitNode` 结构
- `split_tree.rs` 已支持多 pane 布局
- 需要为每个新 pane 创建对应的 TerminalBuffer

### 3.2 TUI 光标修复 (P0)

**问题分析：**

```rust
// terminal_view.rs:136
let show_cursor = false; // 光标被禁用
```

光标被禁用的原因：
1. 光标位置计算与 TUI 应用渲染冲突
2. Claude Code 等应用自己渲染光标/高亮
3. 我们的光标覆盖会破坏 TUI 显示

**解决方案：**

```rust
impl TerminalView {
    /// 检测当前是否是 TUI 应用（通过检测 alternate screen 模式）
    fn is_tui_active(&self) -> bool {
        // 通过检测终端状态或特定转义序列
    }
}

// 在 render 中：
let show_cursor = !self.is_tui_active() && self.is_focused && self.cursor_visible;
```

### 3.3 Diff/Review 实现 (P1)

**Local PTY 方案：**

```rust
fn open_diff(&self, worktree: &Path, pane_id: Option<&PaneId>) -> Result<String, RuntimeError> {
    // 1. 检查是否是 main 分支
    // 2. 执行 `git diff main...HEAD` 获取变更
    // 3. 创建新的 pane 显示 diff 输出
    // 4. 或者使用内置 diff 渲染（不依赖 nvim）
    
    // 简化方案：直接在新 pane 中运行 git diff
    let diff_command = format!("git diff main...HEAD --color=always");
    // 发送到指定 pane
    self.send_input(pane_id.unwrap_or(&self.primary_pane_id()), diff_command.as_bytes())?;
    Ok("Diff displayed in terminal".to_string())
}

fn open_review(&self, worktree: &Path) -> Result<String, RuntimeError> {
    // 与 open_diff 类似，但添加交互支持
    // 或使用内置 review UI（GPUI 渲染）
    self.open_diff(worktree, None)
}
```

### 3.4 Tmux Backend 实现 (P1)

**必要性：**
- Local PTY 无法做到 session 持久化
- 用户期望关闭 UI 后 agent 继续运行

**TmuxRuntime 设计：**

```rust
pub struct TmuxRuntime {
    session_name: String,
    window_name: String,
    // 复用现有 tmux 模块功能
}

impl AgentRuntime for TmuxRuntime {
    fn send_input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
        // tmux send-keys
    }
    
    fn subscribe_output(&self, pane_id: &PaneId) -> Option<flume::Receiver<Vec<u8>>> {
        // tmux pipe-pane -o
    }
    
    fn split_pane(&self, pane_id: &PaneId, vertical: bool) -> Result<PaneId, RuntimeError> {
        // tmux split-window
    }
    
    fn open_diff(&self, worktree: &Path, pane_id: Option<&PaneId>) -> Result<String, RuntimeError> {
        // 创建新 window 运行 nvim diffview
    }
}
```

**Backend 选择配置：**

```json
// config.json
{
  "backend": "tmux",  // "local_pty" | "tmux"
  "tmux": {
    "session_prefix": "pmux-"
  }
}
```

### 3.5 Session 恢复 (P2)

**状态保存：**

```rust
// state.rs 已定义结构
pub struct WorktreeState {
    pub branch: String,
    pub path: PathBuf,
    pub agent_id: String,
    pub pane_ids: Vec<String>,
    pub backend: String,           // "local" | "tmux"
    pub backend_session_id: String,
    pub backend_window_id: String,
}
```

**恢复逻辑：**

```rust
fn try_recover_then_switch(&mut self, workspace_path: &Path, worktree_path: &Path, branch_name: &str, cx: &mut Context<Self>) -> bool {
    let state = RuntimeState::load().ok()?;
    let workspace = state.find_workspace(workspace_path)?;
    let worktree = workspace.worktrees.iter().find(|w| w.path == worktree_path)?;
    
    match worktree.backend.as_str() {
        "tmux" => {
            // attach 到现有 tmux session
            let runtime = TmuxRuntime::attach(
                &worktree.backend_session_id,
                &worktree.backend_window_id,
            )?;
            self.runtime = Some(Arc::new(runtime));
            true
        }
        "local" => {
            // local pty 无法恢复，返回 false 重新创建
            false
        }
        _ => false,
    }
}
```

### 3.6 移除状态轮询 (P2)

**当前实现：**

```rust
// status_publisher.rs
pub fn start<F>(&mut self, capture_fn: F) {
    // 500ms 轮询
    thread::spawn(move || {
        loop {
            for pane_id in pane_ids {
                let content = capture_fn(&pane_id);  // 获取终端内容
                let status = detector.detect(&content);  // 文本解析
                // 发布事件...
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}
```

**目标实现：**

```rust
// 1. 基于进程生命周期
pub enum ProcessEvent {
    Started { pid: u32 },
    Running { pid: u32 },
    WaitingInput { pid: u32 },  // 通过 PTY blocking 检测
    Exited { pid: u32, code: Option<i32> },
    Crashed { pid: u32, signal: i32 },
}

// 2. 通过 PTY 检测等待输入状态
fn detect_waiting_input(pty: &MasterPty) -> bool {
    // 检测 PTY 是否在等待读取（blocking）
    // 或通过分析 shell prompt 状态
}

// 3. Event Bus 发布
impl LocalPtyRuntime {
    fn spawn_process_monitor(&self) {
        // 监控子进程状态变化
        // 发布 AgentStateChange 事件到 EventBus
    }
}
```

## 4. 实施顺序

```
Phase A (P0 - 核心功能)
├── 1. 修复 TUI 光标显示
├── 2. 实现 split_pane (local_pty 多 pane)
└── 3. 测试分屏与 TUI 兼容性

Phase B (P1 - 完整功能)
├── 4. 实现 open_diff / open_review (local_pty 方案)
├── 5. 实现 TmuxRuntime backend
├── 6. 添加 backend 选择配置
└── 7. 测试 session 持久化

Phase C (P2 - 优化)
├── 8. 实现 recover() session 恢复
├── 9. 移除 status_publisher 轮询
├── 10. 实现基于进程的状态检测
└── 11. 清理代码中的 tmux 残留注释
```

## 5. 验证标准

| 检查项 | 验证方法 |
|--------|----------|
| 分屏功能 | ⌘D / ⌘⇧D 创建 pane，⌘⌥方向键切换焦点 |
| TUI 支持 | vim 打开文件，光标位置正确，无渲染问题 |
| Diff 视图 | ⌘⇧D 打开 diff，显示 main..HEAD 变更 |
| Session 持久化 | 关闭 pmux，进程继续运行，重新打开可 attach |
| Session 恢复 | 重启 pmux，自动恢复之前的 session |
| 无轮询 | `rg "interval_ms\|sleep.*500" src/` 无结果 |
