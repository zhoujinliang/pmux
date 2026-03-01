# pmux Tmux 集成架构分析

> 对比 cmux、Zed，识别结构性问题和改进方向

## 一、当前 pmux 架构

### 1.1 数据流概览

```
[Keyboard] → AppRoot → key_to_xterm_escape() → runtime.send_input()
                              ↓
                    TmuxRuntime: Enter 用 send-keys，其余用 direct PTY write
                              ↓
                    [writer thread] → tmux display -p #{pane_tty} → write(fd, bytes)

[Output]   tmux pipe-pane -o → dd of=fifo → [reader thread] → flume → TerminalEngine (VTE)
                    ↑
            Bootstrap: capture-pane -p -e → inject 到 channel (含 \n→\r\n 归一化)
```

### 1.2 核心组件

| 组件 | 职责 | 问题点 |
|------|------|--------|
| **TmuxRuntime** | 通过 `tmux` CLI 操作 session/window/pane | 黑盒，无法直接访问内部状态 |
| **subscribe_output** | pipe-pane + fifo + capture-pane bootstrap | 拼接逻辑复杂，易出现 cursor 错位 |
| **send_input** | 普通键 → PTY write；Enter → send-keys | 双路径，Enter 特例（Bug2 workaround） |
| **TerminalEngine** | alacritty Term + VTE，维护自己的 cursor | 与 tmux 内部 cursor 无同步 |
| **SplitNode / split_tree** | pmux 的 layout 树 | 与 tmux 实际 pane 结构独立，resize 需双向同步 |
| **active_pane_target** | 输入路由目标 | 多处维护，易不一致 |

### 1.3 已暴露的 Bug 线索（代码内注释）

- **Bug2**：`send-keys Enter` 与 direct PTY 混用，pipe-pane 对 Enter 的时序敏感；`cat` 4KB 缓冲导致 ls 无输出，改用 `dd bs=1`
- **H_cursor_mid**：capture-pane 注入后光标跑到屏幕中间；需 trim trailing newlines、`\n`→`\r\n`、skip 空白/过长 capture
- **H_ls_no_output**：多种无输出/锁竞争场景的 debug 标签

---

## 二、结构性问题

### 2.1 双重状态，无法强一致

| pmux 侧 | tmux 侧 | 同步方式 | 风险 |
|---------|---------|----------|------|
| SplitNode (layout) | tmux pane grid | pmux 发起 split/resize | tmux 被外部 attach 时 layout 漂移 |
| TerminalEngine cursor | tmux 内部 cursor | 无 | capture-pane 注入导致 cursor 错位 |
| ResizeController (cols/rows) | tmux pane 尺寸 | resize-pane | 切换 workspace 时 last_dims 可能过时 |
| active_pane_target | tmux focused pane | select-pane | 用户 tmux attach 切换 focus 时不同步 |

pmux 把 tmux 当「外部进程」用 CLI 驱动，无法订阅 tmux 内部事件，只能单向推送。

### 2.2 输出管道是「拼接」出来的

```
capture-pane (历史) ──┬── inject (trim, \n→\r\n, skip 启发式) ──┬── flume
                     │                                          │
pipe-pane (新输出) ──┴──────────────────────────────────────────┴──→ TerminalEngine
```

- `pipe-pane` 只给**新输出**，没有历史
- `capture-pane` 给的是**当前屏幕**，不含 escape 语义的完整序列
- 注入逻辑依赖启发式（`has_real_content`, `leading_newlines`, `too_long`），容易误判
- `\n`/`\r`/`\r\n` 归一化是补丁链，cursor 位置依赖 VTE 对 CR/LF 的解释

### 2.3 输入路径分裂

- **普通键**：flume → writer thread → `tmux display -p #{pane_tty}` → open PTY → write
- **Enter**：`tmux send-keys -t target Enter`（新进程）

Enter 走 send-keys 是因为 direct PTY write 在某种时序下导致 pipe-pane 漏掉命令输出。这说明「PTY 直接写」与「tmux 对 PTY 的读」之间存在竞争或缓冲差异。

### 2.4 窗口/光标控制分散

- **窗口**：`ensure_session_and_window`、`split_pane`、`focus_pane`、`resize` 都通过 `tmux` 子进程
- **光标**：pmux 只渲染 TerminalEngine 的 cursor，不向 tmux 发送 CUP 等定位序列；click-to-prompt 的 `click_to_prompt()` 返回 (line, col)，但**没有代码把点击位置转换成发送给 tmux 的 CUP 序列**
- **焦点**：`focus_pane` 调用 `select-pane`，但 pmux 的 `active_pane_target` 与 tmux 的 focus 可能不同步（例如用户 `tmux attach` 切换）

### 2.5 pane_target 语义混杂

- **tmux**：`session:window.%pane_id`（如 `pmux-repo:main.%0`）
- **local**：`local:/path/to/worktree` 或 `local:/path:main`
- **AgentId** vs **PaneId**：`list_panes(agent_id)` 中 `agent_id` 为空时用 window，否则用 agent_id 当 target，语义不统一

---

## 三、cmux 的架构（参考）

根据 [cmux.dev](https://www.cmux.dev/)：

- **不是基于 tmux**，而是用 **libghostty** 作为终端渲染库
- 类比：像用 WebKit 做 webview 一样，cmux 把 libghostty 当「终端 view」用
- 垂直 tab、split pane、通知 ring 都是**应用内实现**，不依赖外部 multiplexer
- 直接 spawn shell/process，PTY 由 libghostty 管理

**与 pmux 的差异：**

| 维度 | pmux | cmux |
|------|------|------|
| 终端渲染 | alacritty_terminal (Rust) | libghostty (Zig) |
| 多路复用 | 外部 tmux | 内置，无 tmux |
| Session 持久化 | 靠 tmux 自带 | 无（或自建） |
| 输入/输出 | 经 tmux 间接 | 直接 PTY |

cmux 的代价是：没有「关闭应用后 session 仍存活」的能力（除非自己实现）；换来的是**单一数据源**，无 tmux/pmux 状态分裂。

---

## 四、Zed 的终端（参考）

- Zed 是编辑器，内置 terminal
- 直接 PTY spawn shell，无 tmux
- 一 tab 一 PTY，逻辑简单

与 pmux 目标不同：pmux 需要多 worktree、多 agent、session 持久化，所以引入 tmux 是合理的，但集成方式导致了结构性问题。

---

## 五、改进方向（按投入排序）

### 5.1 短期：稳住现有架构

1. **统一 Enter 路径**：要么全用 send-keys，要么全用 PTY write，并查清 pipe-pane 漏输出的根因
2. **明确 cursor 策略**：要么不 inject capture-pane，纯 pipe-pane（从空白开始）；要么 inject 时同时发 CUP 把 VTE cursor 设到正确位置
3. **收敛 active_pane_target**：单一来源，所有消费方只读

### 5.2 中期：减少状态分裂

1. **用 tmux 作为唯一 layout 源**：从 `list-panes -F` 等拉取 pane 树，用其驱动 pmux 的 split_tree，而不是 pmux 自己维护再反向推给 tmux
2. **或反过来**：pmux 作为唯一源，tmux 仅作「执行层」；每次布局变更后，用 `tmux list-panes` 校验一致性，不一致时以 pmux 为准重建
3. **考虑 tmux 的 control mode**：若存在，可用更结构化的方式订阅事件，减少轮询和竞态

### 5.3 长期：架构选项

| 选项 | 描述 | 代价 |
|------|------|------|
| **A. 放弃 tmux** | 像 cmux，自建 PTY + 多 pane，用数据库或文件做 session 持久化 | 重写 runtime 层 |
| **B. tmux 作为可选 backend** | 保留 local_pty 为主路径，tmux 仅给「需要 detach」的用户 | 双轨维护 |
| **C. 嵌入 libghostty** | 若 Rust 有绑定，用 libghostty 替代 alacritty_terminal，统一渲染和 PTY 模型 | 依赖生态 |
| **D. 深入 tmux 协议** | 若 tmux 有 control socket / protocol，直接对话而非 CLI | 需研究 tmux 源码 |

---

## 六、结论

当前 pmux 的 tmux 集成存在**结构性**问题：

1. **双状态**：pmux 与 tmux 各维护 layout、cursor、focus，无法强一致
2. **输出拼接**：capture-pane + pipe-pane 的拼接依赖启发式，cursor 易错位
3. **输入分裂**：Enter 与普通键走不同路径，暴露底层竞态
4. **控制分散**：窗口、光标、输入、输出四条线，缺乏统一抽象

cmux 选择「不用 tmux，自建终端」规避了这些问题；pmux 若坚持 tmux 集成，需要要么**以 tmux 为唯一真相**并主动同步，要么**以 pmux 为唯一真相**并把 tmux 降级为执行器，避免两套状态并存。在此基础上，再考虑用 control mode、统一 Enter 路径、简化 bootstrap 等具体改动。
