# design.md 与当前代码差距分析

> 基于 `design.md`、`openspec/changes/runtime-completion/design.md` 及代码库的深入分析

## 一、design.md 成功标志（Section 9）检查

| 成功标志 | 当前状态 | 说明 |
|----------|----------|------|
| 关闭 pmux UI，agent 继续运行 | ⚠️ 部分满足 | **tmux backend**：session 独立于 pmux 进程，关闭窗口后 tmux 继续运行 ✓；**local PTY**：关闭即终止，设计预期。需确保关闭时 `app.quit()` 不杀 tmux（当前已满足，只 quit pmux） |
| vim / TUI 在 pmux 内完全正常 | ⚠️ 基本满足 | `terminal_view.rs` 已实现 `should_show_cursor()`：TUI 激活时隐藏 pmux 光标，避免覆盖。alternate screen 由 alacritty_terminal 处理 ✓ |
| 无 polling loop | ⚠️ 部分满足 | **StatusPublisher**：已改为事件驱动，无 500ms 轮询 ✓。**终端内容**：`setup_local_terminal` 有 16ms 循环调用 `engine.advance_bytes()`，本质是 render tick 从 flume channel 取数据，非 capture-pane 轮询。设计意图的「无轮询」主要针对 status 和 terminal snapshot，当前已达标 |
| UI 不包含 tmux 调用 | ✅ 满足 | UI 层 (`app_root.rs` 等) 仅通过 `AgentRuntime` API 操作；tmux 调用全在 `runtime/backends/tmux.rs` |
| 新 backend 可在不改 UI 情况下接入 | ✅ 满足 | `create_runtime_from_env` + `AgentRuntime` trait，local/tmux 可切换 |

---

## 二、未完成或与设计不符的工作

### 1. recover() 未实现（P0）

**design.md Section 10**：`recover(agent_ids)` 用于 pmux 重启后按 state 映射 attach/spawn。

**runtime-completion design**：`recover_runtime(backend, state)` 已实现，tmux 可 attach。

**当前实现**：`try_recover_then_switch` 和 `try_recover_then_start` 为**空实现**，始终返回 `false`：

```rust:531:540:src/ui/app_root.rs
fn try_recover_then_switch(...) -> bool { false }
fn try_recover_then_start(...) -> bool { false }
```

**影响**：即使用 tmux backend，每次重启都会新建 session，无法 attach 到已有 session。

**待做**：
- 在 `try_recover_then_switch` 中：`RuntimeState::load()` → 找到对应 worktree → 若 `backend == "tmux"` 调用 `recover_runtime` → attach 成功则设置 terminal 并返回 `true`
- 在 `try_recover_then_start` 中：repo 无 worktree 时，若 state 中有 tmux session 则尝试 recover

---

### 2. config.json 不支持 backend 配置（P1）✅ 已完成

**design.md Section 12**：「通过 config.json 或环境变量指定」；**runtime-completion**：`config.json` 中 `"backend": "tmux"`。

**实现**：`Config` 已增加 `backend` 字段；`resolve_backend` 优先级 env > config > default；StatusBar 显示当前 backend；加载时校验非法值并 fallback。

---

### 3. Agent 状态仍依赖文本解析为主（P2）

**design.md Section 12**：主来源为 process lifecycle；WaitingInput 由 PTY blocking 或内部状态机；Error 由 exit code + stderr；文本解析仅 fallback。

**runtime-completion**：要「移除 status_publisher 轮询」，实现基于进程的 `ProcessEvent`（Started/Running/WaitingInput/Exited/Crashed）。

**当前实现**：
- StatusPublisher 已无轮询，改为内容变化时 `check_status`
- 但 `setup_local_terminal` 中固定传 `ProcessStatus::Running`，未接入真实进程状态
- 状态主要依赖 `StatusDetector` 文本 + OSC 133 shell phase

**待做**：
- LocalPtyRuntime 需能提供进程状态（pid、exit、blocking 等）
- 将 `ProcessStatus` 从 runtime 传给 StatusPublisher，而不是写死 `Running`

---

### 4. 16ms 循环的性质（P2，可选）

**design.md**：「无 polling loop」指 status 与 terminal snapshot 的定时轮询。

**当前实现**：16ms 循环用于 `engine.advance_bytes()` + `cx.notify()`，数据来自 flume channel（runtime 推送），非定时拉取。更接近「render tick」，但仍是固定间隔循环。

**建议**：若需严格「无任何定时循环」，可考虑基于 channel 的 `recv` 或 `select!` 驱动，收到数据再 `cx.notify()`；需权衡复杂度和响应延迟。

---

### 5. 快捷键与菜单占位（P1）

**design.md Section 11**：⌘⇧R 打开 Review。

**当前实现**：`open_review` 已实现（tmux 开 nvim diffview）。但 `main.rs` 中：
- `toggle_sidebar_from_menu`：仅 `println!`，未实际切换
- `select_workspace_from_menu`：仅 `println!`，未打开选择器

**待做**：菜单动作需与 AppRoot 联动（如通过 `cx.dispatch_action` 或 callback）。

---

### 6. TabBar–Sidebar 双向同步（tabbar-worktree-integration）

**openspec tabbar-worktree-integration**：
- Sidebar 点击 worktree → TabBar 激活对应 tab
- TabBar 点击 tab → Sidebar 选中对应项

**当前实现**：Sidebar 点击会设 `pending_worktree_selection`，`process_pending_worktree_selection` 调用 `switch_to_worktree`，即 Sidebar → 内容 已打通。但当前架构中「TabBar」指 pane tabs（同一 worktree 内多 pane），不是 worktree 列表；worktree 选择主要通过 Sidebar 完成。若设计中的 TabBar 指「worktree tabs」，则缺少 TabBar → Sidebar 的反向同步逻辑。

**建议**：确认 tabbar-worktree 设计中 TabBar 语义（pane vs worktree），再补全双向同步。

---

### 7. 其他设计与实现差异

| 项目 | 设计 | 当前 | 说明 |
|------|------|------|------|
| PtyHandle trait | 设计中有 `trait PtyHandle` | 未显式定义 | 功能由 AgentRuntime 覆盖，可接受 |
| subscribe_state | AgentRuntime 应有 `subscribe_state()` | 未在 trait 中 | 状态经 EventBus 发布，实现方式不同但效果类似 |
| restart/recover API | 设计中有 `restart(agent_id)`, `recover(agent_ids)` | 仅有 `recover_runtime` 内部用 | 对外未暴露统一 `restart`/`recover` API |
| LocalPtyAgent vs LocalPtyRuntime | runtime-completion 中 LocalPtyAgent 支持多 pane | 代码中两者并存 | LocalPtyRuntime 单 pane；多 pane 用 LocalPtyAgent，需确认实际使用的入口 |

---

## 三、实施优先级建议

| 优先级 | 项目 | 预估 | 备注 |
|--------|------|------|------|
| P0 | 实现 `try_recover_then_switch` / `try_recover_then_start`，调用 `recover_runtime` | 0.5 天 | tmux 持久化体验关键 |
| P1 | Config 增加 `backend` 字段并接入 | 0.5 天 | ✅ 已完成 |
| P1 | 菜单动作与 AppRoot 联动 | 0.5 天 | 完善基础 UX |
| P2 | 进程状态接入 StatusPublisher | 1–2 天 | 减少对文本解析依赖 |
| P2 | 确认 TabBar–Sidebar 语义并补全同步 | 0.5 天 | 取决于产品定义 |

---

## 四、已正确实现且符合设计的部分

- AgentRuntime trait 与 tmux/local_pty 双 backend
- EventBus + StatusPublisher 事件驱动状态发布
- PTY streaming（pipe-pane / local PTY）
- xterm escape 键盘输入
- split_pane、open_diff、open_review、kill_window
- TUI 光标按 `is_tui_active` 隐藏
- RuntimeState 持久化与 `save_runtime_state`
- UI 不直接调用 tmux，全部经 Runtime API
- `recover_runtime` 在 backends 层已实现（tmux attach）
