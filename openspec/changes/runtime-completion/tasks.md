## Phase A: P0 核心功能

### A1. 修复 TUI 光标支持

- A1.1 分析当前光标禁用原因（terminal_view.rs:136）
- A1.2 研究 TermBridge 如何检测 alternate screen 模式
- A1.3 实现 `is_tui_active()` 检测逻辑
- A1.4 修改光标渲染逻辑：TUI 活动时隐藏，普通终端时显示
- A1.5 测试 vim 打开文件时的光标表现
- A1.6 测试 Claude Code 的光标/选择高亮
- A1.7 修复光标位置计算偏移问题
- A1.8 验证 focus/unfocus 状态切换时 cursor 行为正确

### A2. 实现 split_pane (local_pty)

- [x] A2.1 设计 `LocalPtyAgent` 多 pane 管理结构
- [x] A2.2 修改 `LocalPtyRuntime` 支持创建多个实例
- [x] A2.3 实现 `split_pane` 方法，创建新 PTY
- [x] A2.4 为新 pane 生成唯一 ID 格式
- [x] A2.5 在 AppRoot 中为 split 创建 TerminalBuffer
- [x] A2.6 集成 StatusPublisher 注册新 pane
- [x] A2.7 修复 ⌘D (垂直分屏) 快捷键
- [x] A2.8 修复 ⌘⇧D (水平分屏) 快捷键
- [x] A2.9 修复 ⌘⌥方向键 pane 焦点切换
- [x] A2.10 测试分屏后各 pane 独立工作
- [x] A2.11 测试分屏后状态检测正常

## Phase B: P1 完整功能

### B1. 实现 open_diff / open_review

- [x] B1.1 设计 diff 显示方案（terminal 输出 vs 内置 UI）
- [x] B1.2 实现 `open_diff` 基础逻辑
- [x] B1.3 检测 main 分支，避免空 diff
- [x] B1.4 执行 `git diff main...HEAD --color=always`
- [x] B1.5 在新 pane 或当前 pane 显示 diff
- [x] B1.6 实现 `open_review`（复用或扩展 open_diff）
- [x] B1.7 修复 ⌘⇧D 快捷键打开 diff
- [x] B1.8 修复 ⌘⇧R 快捷键打开 review
- [x] B1.9 测试 diff 内容正确显示
- [x] B1.10 测试关闭 diff pane 后回到原工作区

### B2. 实现 TmuxRuntime Backend

- [x] B2.1 创建 `src/runtime/backends/tmux.rs` (legacy) + `tmux_control_mode.rs` (current)
- [x] B2.2 实现 `TmuxControlModeRuntime` 结构体（consolidated from tmux + tmux-cc）
- [x] B2.3 实现 `AgentRuntime` trait for `TmuxControlModeRuntime`
- [x] B2.4 实现 `send_input` via control mode `send-keys -l`
- [x] B2.5 实现 `subscribe_output` via `%output` events (control mode)
- [x] B2.6 实现 `split_pane` via `split-window` command
- [x] B2.7 实现 `open_diff` via split + nvim diffview
- [x] B2.8 实现 `kill_window` via `kill-window` command
- [x] B2.9 修改 backends/mod.rs 支持 tmux backend（"tmux" = tmux-cc）
- [x] B2.10 添加 backend 选择逻辑（resolve_backend: env > config > default "tmux"）

### B3. Backend 选择与配置

- [x] B3.1 在 Config 中添加 `backend` 字段
- [x] B3.2 实现 backend 选择逻辑（env PMUX_BACKEND > config.backend > default "tmux"）
- [x] B3.3 修改 `create_runtime_from_env` 根据配置创建对应 backend
- [x] B3.4 更新依赖检测：tmux backend 检查 tmux_available()，不可用时 fallback 到 local
- [x] B3.5 添加 `PMUX_BACKEND` 环境变量支持
- [x] B3.6 测试 backend 切换正常工作
- [x] B3.7 测试 tmux backend session 持久化（tmux_cc_e2e.sh）
- [x] B3.8 测试 local_pty 无需 tmux 依赖（smoke_ls_pwd.sh with PMUX_BACKEND=local）

## Phase C: P2 优化完善

### C1. 实现 recover() Session 恢复

- [x] C1.1 分析 RuntimeState 现有数据结构
- [x] C1.2 完善 `try_recover_then_switch` 实现
- [x] C1.3 完善 `try_recover_then_start` 实现
- [x] C1.4 实现 tmux session attach 逻辑（via TmuxControlModeRuntime::new with existing session）
- [x] C1.5 实现 pane 状态恢复
- [x] C1.6 测试重启后 session 恢复（tmux_cc_e2e.sh Test 5）
- C1.7 测试多 worktree 场景恢复
- [x] C1.8 处理恢复失败时的 fallback 逻辑

### C2. 移除状态轮询

- C2.1 分析当前 StatusPublisher 轮询逻辑
- C2.2 设计基于进程生命周期的状态检测方案
- C2.3 实现子进程状态监控
- C2.4 实现 ProcessEvent 定义
- C2.5 实现 waiting input 检测逻辑
- C2.6 修改 EventBus 发布 AgentStateChange
- C2.7 移除 StatusPublisher 轮询循环
- C2.8 验证状态变化实时更新
- C2.9 测试 Error 状态立即触发（不 debounce）

### C3. 代码清理

- C3.1 清理 app_root.rs 中的 tmux 注释
- C3.2 清理 deps.rs 中的 tmux 相关代码（local_pty 模式不需要）
- C3.3 清理 terminal_view.rs 中的 tmux 注释
- C3.4 更新 agent_status.rs 注释（不再特指 tmux pane）
- C3.5 检查并清理所有残留 tmux 引用
- C3.6 更新文档和注释

## 验收清单

### 功能验收

- 验收 A1: vim 中光标正常显示和操作
- 验收 A2: 分屏功能完整可用（创建、切换、关闭）
- 验收 B1: Diff/Review 功能正常
- 验收 B2: tmux backend 可用
- 验收 B3: backend 切换正常
- 验收 C1: 重启后 session 恢复
- 验收 C2: 状态检测无轮询

### 代码验收

- `rg "tmux::" src/ui/` 无结果
- `rg "interval_ms\|sleep.*500" src/runtime/` 无结果（或仅为 backend 内部实现）
- `cargo test` 通过
- `cargo run` 正常启动
- 多 workspace/worktree 测试通过

### 性能验收

- 状态变化延迟 < 100ms
- 分屏操作流畅无卡顿
- TUI 应用响应正常

