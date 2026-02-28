# 输入路径 Zed 风格重构方案

> 参考 Zed 集成终端（`crates/terminal_view`, `crates/terminal`）的键盘输入处理方式。

---

## 1. Zed 的做法（核心差异）

### 1.1 输入路径

```
key_down (UI 主线程)
  → process_keystroke
  → try_keystroke / to_esc_str
  → term.input(bytes)
  → write_to_pty
  → pty_tx.notify(input)     ← 仅 channel 发送，立即返回
```

- **同步**：从 key_down 到 `pty_tx.notify()` 全程在 UI 主线程、**无 spawn、无 blocking::unblock**
- **PTY 写**：`pty_tx` 是 Alacritty event loop 的 channel，**实际 write 在 IO 线程**完成
- **本质**：UI 只做 channel send；I/O 已由 writer 线程承担

### 1.2 相关文件

| Zed 文件 | 职责 |
|----------|------|
| `terminal_view.rs` | key_down, process_keystroke, commit_text |
| `terminal.rs` | write_to_pty, input, try_keystroke |
| `mappings/keys.rs` | to_esc_str（key → xterm 序列） |

---

## 2. pmux 当前问题

### 2.1 多余异步层

```rust
// app_root.rs - 当前
cx.spawn(async move |_entity, _cx| {
    let result = blocking::unblock(move || rt.send_input(&target, &bytes)).await;
    ...
}).detach();
```

- `send_input` 内部仅是 `flume::Sender::send()`，**无 I/O**
- Tmux / LocalPty 的 writer 线程已负责 PTY write
- 额外 `spawn` + `blocking::unblock` 增加调度和上下文切换开销

### 2.2 与 Zed 的对比

| 环节 | Zed | pmux 现状 |
|------|-----|-----------|
| key → bytes | 同步 | 同步 ✓ |
| bytes → channel | 同步 `pty_tx.notify()` | **异步** spawn+blocking |
| channel → PTY write | IO 线程 | writer 线程 ✓ |

---

## 3. 方案概览

### 3.1 原则（对齐 Zed）

1. **key_down 内同步调用 send_input**：不再 spawn、不再 blocking::unblock
2. **保持 channel + writer 线程**：Runtime 架构不变，writer 仍负责 PTY write
3. **输出链路优化**（可选 Phase 2）：由 16ms tick 改为事件驱动

---

## 4. Task 1：去掉 spawn + blocking（P0）

**目标**：使输入路径与 Zed 一致，在主线程直接调用 `send_input`。

**文件**：`src/ui/app_root.rs`

**修改前：**
```rust
cx.spawn(async move |_entity, _cx| {
    let result = blocking::unblock(move || rt.send_input(&target, &bytes)).await;
    if let Err(e) = result { eprintln!("pmux: send_input failed: {}", e); }
}).detach();
```

**修改后：**
```rust
if let Err(e) = runtime.send_input(target, &bytes) {
    eprintln!("pmux: send_input failed: {}", e);
}
```

**前置条件**：
- `runtime` 和 `target` 已在 match 分支内可用，无需 move
- `bytes` 来自 `key_to_xterm_escape`，可传 `&[u8]`

**验收**：快速连打无明显卡顿；vim/fzf 方向键、Ctrl 组合响应正常。

---

## 5. Task 2：Tmux PTY 预热（P1，可选）

**问题**：首次向某 pane 输入时，`write_to_pane_pty` 会执行 `tmux display -p -t target #{pane_tty}`，约 50–100ms。

**思路**：在 `focus_pane` 或切换 pane 时预先解析 PTY 路径并写入 cache，首次 send_input 即命中。

**文件**：`src/runtime/backends/tmux.rs`

**步骤**：
1. 在 `focus_pane` 中调用 `get_pane_tty_path_standalone` 并 `write_to_pane_pty` 一次空字节（或打开并缓存 fd）
2. 或新增 `prewarm_pty(pane_id)`，在 `setup_pane_terminal_output` 时调用

---

## 6. Task 3：输出改为事件驱动（P2，可选）

**问题**：当前用 16ms 定时 tick 调用 `engine.advance_bytes()`，回显可延迟 0–16ms。

**Zed**：Alacritty event loop 用 `select`/`epoll` 等，有数据即处理；输出 batch 约 4ms 或 100 events。

**思路**：
1. 用 `rx.recv_timeout(Duration::from_millis(4))` 替代 `sleep(16)`，有数据立即处理
2. 或 `tokio::sync::mpsc` + `select!`：recv vs 短 timer，数据优先

**文件**：`src/ui/app_root.rs` 中 `setup_local_terminal`、`setup_pane_terminal_output` 的 spawn 循环

---

## 7. 实施顺序

| 优先级 | Task | 预估 | 收益 |
|--------|------|------|------|
| P0 | Task 1：去掉 spawn+blocking | 10 分钟 | 去除多余异步，输入路径与 Zed 一致 |
| P1 | Task 2：Tmux PTY 预热 | 30 分钟 | 消除“第一个键慢” |
| P2 | Task 3：输出事件驱动 | 1–2 小时 | 降低回显延迟 |

**建议**：先完成 Task 1，验证主观延迟改善，再视情况做 Task 2/3。

---

## 8. 参考

- Zed terminal input: `crates/terminal_view/src/terminal_view.rs` (key_down, process_keystroke)
- Zed write_to_pty: `crates/terminal/src/terminal.rs` (write_to_pty, input)
- pmux 当前实现: `src/ui/app_root.rs` (handle_key_down), `src/runtime/backends/tmux.rs` (send_input)
