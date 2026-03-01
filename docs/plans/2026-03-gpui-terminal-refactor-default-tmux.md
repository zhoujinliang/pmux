# gpui-terminal 重构 + 默认 tmux 方案

> Brainstorming 方案：用 gpui-terminal 替换自建终端管线，默认 backend 改为 tmux

---

## 一、目标

1. 用 **gpui-terminal** 替换当前 TerminalEngine + TerminalView + terminal_rendering 自建管线
2. 默认 backend 从 `local` 改为 `tmux`
3. 保持 AgentRuntime 接口不变，tmux/local 双 backend 并存
4. 保持 status detection、shell integration 等能力

---

## 二、当前架构 vs 目标架构

### 2.1 当前

```
subscribe_output() → flume rx → TerminalEngine (alacritty Term + VTE)
                                        ↓
                              advance_bytes() 轮询
                                        ↓
                    TerminalView (Render) → terminal_rendering → GPUI
                                        ↑
                    content_for_status_detection() ← StatusPublisher
```

### 2.2 目标

```
subscribe_output() → Tee → rx1 → RuntimeReader (impl Read) → gpui_terminal::TerminalView
                       → rx2 → ContentPipe → StatusPublisher (content + OSC 133)

Keyboard → runtime.send_input() ← RuntimeWriter (impl Write) ← gpui_terminal
```

---

## 三、关键适配点

### 3.1 I/O 桥接

gpui-terminal 需要 `Read` + `Write` 流，AgentRuntime 提供 `subscribe_output() -> Receiver` 和 `send_input(pane_id, bytes)`。

| 适配器 | 职责 |
|--------|------|
| **RuntimeReader** | `impl Read`，内部持 `flume::Receiver<Vec<u8>>`，read 时从 recv 取数据填满 buf |
| **RuntimeWriter** | `impl Write`，持 `(Arc<dyn AgentRuntime>, PaneId)`，write 时调用 `send_input` |
| **TeePipe** | 将 `subscribe_output()` 的单一 rx 复制为两路，一路给 gpui-terminal，一路给 status 管线 |

### 3.2 tmux Bootstrap

tmux 的 `subscribe_output` 已在内部注入 `capture_initial_content`，我们的 RuntimeReader 直接消费即可，无需额外处理。

### 3.3 Status 检测

当前 `content_for_status_detection()` 从 TerminalEngine 的 grid 提取文本。gpui-terminal 不暴露 grid。

**方案**：Tee 一路字节流到轻量 **ContentExtractor**：
- 解析 OSC 133（ShellPhaseInfo）
- 从可见输出提取文本（简单 VT 解析或按行累积）
- 调用 `StatusPublisher.check_status(..., content)`

可复用现有 `Osc133Parser`，文本提取需新建或复用简化逻辑。

### 3.4 GPUI 版本兼容

- gpui-terminal: `gpui = "0.2.2"`（crates.io）
- pmux: `gpui = { git = "zed-industries/zed" }`

**方案**：
1. 先 `cargo add gpui-terminal` 验证是否能通过
2. 若有冲突，fork gpui-terminal，改为依赖 pmux 同源的 gpui
3. 或提 PR 让 gpui-terminal 支持 gpui path override

---

## 四、默认 Backend 改为 tmux

| 位置 | 改动 |
|------|------|
| `runtime/backends/mod.rs` | `DEFAULT_BACKEND: "local"` → `"tmux"` |
| `config.rs` | `default_backend()` 返回 `"tmux"` |
| `resolve_backend` fallback | 无效值 fallback 从 `"local"` 改为 `"tmux"`（可选，建议保持 local 更安全） |

**注意**：首次启动无 tmux 时需有清晰错误提示；或保留「tmux 不可用时自动回退 local」的逻辑（需评估）。

---

## 五、可删除/简化的模块

| 模块 | 处理 |
|------|------|
| `terminal/engine.rs` | 删除或保留为 legacy，gpui-terminal 内部用 alacritty_terminal |
| `terminal/renderable_snapshot.rs` | 删除 |
| `terminal/term_bridge.rs` | 删除 |
| `ui/terminal_rendering.rs` | 删除（style-run batching 等） |
| `ui/terminal_element.rs` | 删除或大幅简化 |
| `ui/terminal_renderer/*` | 删除（build_frame, row_cache, layout_grid 等） |
| `TerminalBuffer` | 重构：不再持 engine，改为持 `gpui_terminal::TerminalView` 或直接嵌在 layout 中 |
| `TerminalView` | 替换为 gpui_terminal::TerminalView，或薄封装 |

---

## 六、分 Phase 实施

### Phase 1：依赖与兼容（1–2 天）

- [ ] 添加 gpui-terminal 依赖，解决与 gpui 版本冲突
- [ ] 新建 `src/terminal/stream_adapter.rs`：RuntimeReader, RuntimeWriter, TeePipe
- [ ] 新建 `src/terminal/content_extractor.rs`：从字节流提取 text + OSC 133 供 StatusPublisher 使用
- [ ] 单元测试：stream adapter 正确转发，content extractor 正确解析

### Phase 2：单 pane 替换（2–3 天）

- [ ] 新建 `GpuiTerminalPane` 或改造 `TerminalView`，内部使用 `gpui_terminal::TerminalView`
- [ ] 修改 `setup_local_terminal`：构造 RuntimeReader/Writer，创建 gpui_terminal view
- [ ] 接入 TeePipe + ContentExtractor，保持 StatusPublisher 工作
- [ ] 修改 keyboard 输入：确保写入 RuntimeWriter 而非直接调用 send_input（gpui-terminal 会处理）
- [ ] 验证 local 与 tmux 双 backend 下单 pane 行为

### Phase 3：Multi-pane 与 resize（1–2 天）

- [ ] 修改 `setup_pane_terminal_output`，每个 pane 独立 Reader/Writer/Tee
- [ ] 对接 gpui-terminal 的 resize callback 与 `runtime.resize()`
- [ ] SplitPaneContainer 中为每个 pane 渲染 gpui_terminal view
- [ ] 验证 focus、split、resize 行为

### Phase 4：清理与默认 backend（1 天）

- [ ] 删除 engine、renderable_snapshot、term_bridge、terminal_rendering、terminal_element、terminal_renderer
- [ ] 修改 DEFAULT_BACKEND 与 default_backend() 为 tmux
- [ ] 更新文档、CLAUDE.md
- [ ] 全量回归：local + tmux，单/多 pane，status，diff overlay

### Phase 5（可选）：回退策略

- [ ] tmux 不可用时（未安装、启动失败）自动 fallback 到 local
- [ ] 配置或 UI 中允许用户显式选择 backend

---

## 七、风险与缓解

| 风险 | 缓解 |
|------|------|
| gpui-terminal 与 gpui 版本不兼容 | 提前验证；必要时 fork 或等上游支持 |
| Status 检测精度下降 | ContentExtractor 需充分测试；必要时保留 tmux capture-pane 作为 status 备用源 |
| gpui-terminal 无 scrollback 导航 | 文档说明；后续可提 issue 或 PR |
| 默认 tmux 导致无 tmux 用户无法启动 | 自动 fallback 到 local + 清晰错误提示 |

---

## 八、文件变更清单（预估）

| 操作 | 路径 |
|------|------|
| 新增 | `src/terminal/stream_adapter.rs` |
| 新增 | `src/terminal/content_extractor.rs` |
| 修改 | `Cargo.toml`（gpui-terminal，可选 fork） |
| 修改 | `src/runtime/backends/mod.rs`（DEFAULT_BACKEND） |
| 修改 | `src/config.rs`（default_backend） |
| 修改 | `src/ui/app_root.rs`（setup_local_terminal, setup_pane_terminal_output, attach_runtime） |
| 修改 | `src/ui/split_pane_container.rs`（渲染 gpui_terminal view） |
| 修改 | `src/ui/terminal_view.rs`（可能简化为 gpui_terminal 的薄封装或直接替换） |
| 删除 | `src/terminal/engine.rs` |
| 删除 | `src/terminal/renderable_snapshot.rs` |
| 删除 | `src/terminal/term_bridge.rs` |
| 删除 | `src/ui/terminal_rendering.rs` |
| 删除 | `src/ui/terminal_element.rs` |
| 删除 | `src/ui/terminal_renderer/*` |

---

## 九、验收标准

1. 单 pane、多 pane 在 local 与 tmux 下均正常显示与输入
2. StatusPublisher 正确检测 agent 状态（含 OSC 133 与文本 fallback）
3. Resize、focus、split 行为符合预期
4. 默认 backend 为 tmux，且可通过 config/env 切换
5. 无 tmux 时能 fallback 到 local 或给出明确错误提示
