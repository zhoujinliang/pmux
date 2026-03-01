# gpui-terminal 重构 + 默认 tmux 实施计划

> **For Claude:** Use TDD when implementing. Consider `subagent-driven-development` for parallel tasks (Phase 1 stream adapters can run in parallel).

**Goal:** 用 gpui-terminal 替换自建终端管线，默认 backend 改为 tmux，保持 status detection 与双 backend 能力。

**Architecture:** subscribe_output() → TeePipe → (RuntimeReader → gpui_terminal::TerminalView, ContentExtractor → StatusPublisher)。输入经 gpui-terminal 的 Write 流由 RuntimeWriter 转发到 runtime.send_input()。

**Tech Stack:** Rust, GPUI, gpui-terminal, portable-pty, flume

---

## Phase 1: 依赖与 Stream 适配器

### Task 1: 添加 gpui-terminal 依赖并验证构建

**Files:**
- Modify: Cargo.toml

**Step 1: 添加依赖**
在 `Cargo.toml` 的 `[dependencies]` 中添加：

```toml
gpui-terminal = "0.1"
```

**Step 2: 运行构建**
Run: `RUSTUP_TOOLCHAIN=stable cargo check`
Expected: 若 gpui 版本冲突（gpui-terminal 要求 0.2.2，pmux 用 git），会报错。

**Step 3a: 若构建通过**
直接进入 Task 2。

**Step 3b: 若 gpui 冲突**
使用 patch 覆盖 gpui-terminal 的 gpui 来源：

```toml
[patch.crates-io]
gpui = { git = "https://github.com/zed-industries/zed" }
gpui_platform = { git = "https://github.com/zed-industries/zed", features = ["font-kit"] }
```

或 fork gpui-terminal，在 fork 的 Cargo.toml 中改用 `gpui = { git = "..." }`。

**Step 4: 验证**
Run: `RUSTUP_TOOLCHAIN=stable cargo check`
Expected: PASS

**Step 5: Commit**
```
git add Cargo.toml Cargo.lock
git commit -m "chore: add gpui-terminal dependency"
```

---

### Task 2: RuntimeReader（TDD）

**Files:**
- Create: src/terminal/stream_adapter.rs
- Modify: src/terminal/mod.rs

**Step 1: 写失败测试**
在 `src/terminal/stream_adapter.rs` 末尾添加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_reader_reads_from_flume() {
        let (tx, rx) = flume::unbounded();
        tx.send(b"hello".to_vec()).unwrap();
        tx.send(b" world".to_vec()).unwrap();
        drop(tx);

        let mut reader = RuntimeReader::new(rx);
        let mut buf = [0u8; 32];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..5], b"hello");

        let n2 = reader.read(&mut buf).unwrap();
        assert_eq!(n2, 6);
        assert_eq!(&buf[..6], b" world");
    }
}
```

**Step 2: 验证失败**
Run: `cargo test stream_adapter::tests::test_runtime_reader_reads_from_flume`
Expected: FAIL（RuntimeReader 未定义）

**Step 3: 最小实现**
在 `src/terminal/stream_adapter.rs` 中实现：

```rust
use std::io::{Read, Result as IoResult};
use std::sync::Arc;

/// Wraps flume::Receiver<Vec<u8>> as std::io::Read for gpui-terminal.
pub struct RuntimeReader {
    rx: flume::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
}

impl RuntimeReader {
    pub fn new(rx: flume::Receiver<Vec<u8>>) -> Self {
        Self { rx, buf: Vec::new(), pos: 0 }
    }
}

impl Read for RuntimeReader {
    fn read(&mut self, out: &mut [u8]) -> IoResult<usize> {
        while self.pos >= self.buf.len() {
            match self.rx.recv() {
                Ok(chunk) => {
                    self.buf = chunk;
                    self.pos = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = (self.buf.len() - self.pos).min(out.len());
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}
```

在 `src/terminal/mod.rs` 中添加：
```rust
pub mod stream_adapter;
pub use stream_adapter::RuntimeReader;
```

**Step 4: 验证通过**
Run: `cargo test stream_adapter::tests::test_runtime_reader_reads_from_flume`
Expected: PASS

**Step 5: Commit**
```
git add src/terminal/stream_adapter.rs src/terminal/mod.rs
git commit -m "feat(terminal): add RuntimeReader adapter for gpui-terminal"
```

---

### Task 3: RuntimeWriter（TDD）

**Files:**
- Modify: src/terminal/stream_adapter.rs

**Step 1: 写失败测试**
在 `stream_adapter.rs` 的 tests 模块中添加：

```rust
#[test]
fn test_runtime_writer_forwards_to_send_input() {
    use crate::runtime::agent_runtime::{AgentRuntime, PaneId, RuntimeError};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct MockRuntime {
        sent: AtomicU64,
    }
    impl AgentRuntime for MockRuntime {
        fn backend_type(&self) -> &'static str { "mock" }
        fn send_input(&self, _: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
            self.sent.fetch_add(bytes.len() as u64, Ordering::SeqCst);
            Ok(())
        }
        fn send_key(&self, _: &PaneId, _: &str, _: bool) -> Result<(), RuntimeError> { Ok(()) }
        fn resize(&self, _: &PaneId, _: u16, _: u16) -> Result<(), RuntimeError> { Ok(()) }
        fn subscribe_output(&self, _: &PaneId) -> Option<flume::Receiver<Vec<u8>>> { None }
        fn capture_initial_content(&self, _: &PaneId) -> Option<Vec<u8>> { None }
        fn list_panes(&self, _: &crate::runtime::agent_runtime::AgentId) -> Vec<PaneId> { vec![] }
        fn focus_pane(&self, _: &PaneId) -> Result<(), RuntimeError> { Ok(()) }
        fn split_pane(&self, _: &PaneId, _: bool) -> Result<PaneId, RuntimeError> { Err(RuntimeError::Backend("".into())) }
        fn get_pane_dimensions(&self, _: &PaneId) -> (u16, u16) { (80, 24) }
        fn open_diff(&self, _: &std::path::Path, _: Option<&PaneId>) -> Result<String, RuntimeError> { Err(RuntimeError::Backend("".into())) }
        fn open_review(&self, _: &std::path::Path) -> Result<String, RuntimeError> { Err(RuntimeError::Backend("".into())) }
        fn kill_window(&self, _: &str) -> Result<(), RuntimeError> { Ok(()) }
        fn session_info(&self) -> Option<(String, String)> { None }
    }

    let rt = Arc::new(MockRuntime { sent: AtomicU64::new(0) });
    let pane_id = PaneId::from("%0");
    let mut writer = RuntimeWriter::new(rt.clone(), pane_id.clone());
    writer.write_all(b"abc").unwrap();
    writer.flush().unwrap();
    assert_eq!(rt.sent.load(Ordering::SeqCst), 3);
}
```

**Step 2: 验证失败**
Run: `cargo test stream_adapter::tests::test_runtime_writer_forwards_to_send_input`
Expected: FAIL

**Step 3: 实现 RuntimeWriter**
在 stream_adapter.rs 中添加：

```rust
use std::io::{Write, Result as IoResult};
use crate::runtime::agent_runtime::{AgentRuntime, PaneId};

pub struct RuntimeWriter {
    runtime: Arc<dyn AgentRuntime>,
    pane_id: PaneId,
}

impl RuntimeWriter {
    pub fn new(runtime: Arc<dyn AgentRuntime>, pane_id: PaneId) -> Self {
        Self { runtime, pane_id }
    }
}

impl Write for RuntimeWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.runtime.send_input(&self.pane_id, buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> IoResult<()> { Ok(()) }
}
```

**Step 4: 验证通过**
Run: `cargo test stream_adapter::tests::test_runtime_writer_forwards_to_send_input`
Expected: PASS

**Step 5: Commit**
```
git add src/terminal/stream_adapter.rs
git commit -m "feat(terminal): add RuntimeWriter adapter"
```

---

### Task 4: TeePipe（TDD）

**Files:**
- Modify: src/terminal/stream_adapter.rs

**Step 1: 写失败测试**
```rust
#[test]
fn test_tee_pipe_fans_out_bytes() {
    let (tx, rx) = flume::unbounded();
    let (rx1, rx2) = tee_output(rx);
    tx.send(b"x".to_vec()).unwrap();
    tx.send(b"y".to_vec()).unwrap();
    drop(tx);

    let a: Vec<Vec<u8>> = rx1.try_iter().collect();
    let b: Vec<Vec<u8>> = rx2.try_iter().collect();
    assert_eq!(a, vec![b"x".to_vec(), b"y".to_vec()]);
    assert_eq!(b, vec![b"x".to_vec(), b"y".to_vec()]);
}
```

**Step 2: 验证失败**
Run: `cargo test stream_adapter::tests::test_tee_pipe_fans_out_bytes`
Expected: FAIL

**Step 3: 实现 tee_output**
```rust
/// Spawns a thread that forwards each chunk from `rx` to two new receivers.
/// Use one for gpui-terminal, one for ContentExtractor.
pub fn tee_output(rx: flume::Receiver<Vec<u8>>) -> (flume::Receiver<Vec<u8>>, flume::Receiver<Vec<u8>>) {
    let (tx1, rx1) = flume::unbounded();
    let (tx2, rx2) = flume::unbounded();
    std::thread::spawn(move || {
        while let Ok(chunk) = rx.recv() {
            let _ = tx1.send(chunk.clone());
            let _ = tx2.send(chunk);
        }
    });
    (rx1, rx2)
}
```

**Step 4: 验证通过**
Run: `cargo test stream_adapter::tests::test_tee_pipe_fans_out_bytes`
Expected: PASS

**Step 5: Commit**
```
git add src/terminal/stream_adapter.rs
git commit -m "feat(terminal): add tee_output for status pipeline"
```

---

### Task 5: ContentExtractor（TDD）

**Files:**
- Create: src/terminal/content_extractor.rs
- Modify: src/terminal/mod.rs

**Step 1: 写失败测试**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell_integration::ShellPhase;

    #[test]
    fn test_extracts_osc133_phase() {
        let mut ext = ContentExtractor::new();
        let st = b"\x1b]133;A\x1b\\";  // PromptStart
        ext.feed(st);
        assert_eq!(ext.shell_phase(), ShellPhase::Prompt);
    }

    #[test]
    fn test_extracts_visible_text() {
        let mut ext = ContentExtractor::new();
        ext.feed(b"hello\r\n");
        let (text, _) = ext.take_content();
        assert!(text.contains("hello"));
    }
}
```

**Step 2: 验证失败**
Run: `cargo test content_extractor::tests::`
Expected: FAIL

**Step 3: 最小实现**
ContentExtractor 复用 `Osc133Parser` 解析相位；用简化逻辑累积可打印字符作为 content（过滤 CSI/OSC 等）。实现略，参考 `src/shell_integration.rs` 的 Osc133Parser。

**Step 4: 验证通过**
Run: `cargo test content_extractor::`
Expected: PASS

**Step 5: Commit**
```
git add src/terminal/content_extractor.rs src/terminal/mod.rs
git commit -m "feat(terminal): add ContentExtractor for status pipeline"
```

---

## Phase 2: 默认 Backend 改为 tmux

### Task 6: DEFAULT_BACKEND 改为 tmux

**Files:**
- Modify: src/runtime/backends/mod.rs:24
- Modify: src/config.rs (default_backend)

**Step 1: 修改 backends**
在 `src/runtime/backends/mod.rs` 中：
```rust
pub const DEFAULT_BACKEND: &str = "tmux";
```

**Step 2: 修改 config**
在 `src/config.rs` 的 `default_backend()`：
```rust
fn default_backend() -> String {
    "tmux".to_string()
}
```

**Step 3: 运行现有测试**
Run: `RUSTUP_TOOLCHAIN=stable cargo test runtime::backends::`
Expected: 若有测试断言 default 为 "local"，需更新为 "tmux"。

**Step 4: 更新断言**
修改 `test_resolve_backend_config_overrides_default` 等，确保 default 为 tmux 时测试仍通过。

**Step 5: Commit**
```
git add src/runtime/backends/mod.rs src/config.rs
git commit -m "feat(config): default backend to tmux"
```

---

## Phase 3: 单 Pane gpui-terminal 集成

### Task 7: 在 setup_local_terminal 中接入 gpui-terminal

**Files:**
- Modify: src/ui/app_root.rs（setup_local_terminal, attach_runtime 内调用链）
- Modify: src/ui/split_pane_container.rs
- Modify: src/ui/terminal_view.rs 或新建 GpuiTerminalPane

**前置:** 需确认 gpui-terminal 的 TerminalView 创建方式；其 API 为 `TerminalView::new(pty_writer, pty_reader, config, cx)`。

**Step 1: 构造 Reader/Writer**
在 `setup_local_terminal` 中：
- 若 `subscribe_output` 返回 Some(rx)，调用 `tee_output(rx)` 得 (rx1, rx2)
- `RuntimeReader::new(rx1)` 作 reader
- `RuntimeWriter::new(runtime.clone(), pane_target.clone())` 作 writer

**Step 2: 创建 gpui_terminal::TerminalView**
将 reader/writer 传入 gpui-terminal，构造 TerminalConfig（cols, rows, font, colors 等），创建 view。

**Step 3: 替换 TerminalBuffer**
原 `TerminalBuffer::Term(engine, cache)` 改为持 gpui_terminal view 或将其直接嵌入 layout。TerminalBuffer 枚举可新增变体 `GpuiTerminal(Entity<...>)` 或简化为直接渲染 gpui_terminal。

**Step 4: ContentExtractor + StatusPublisher**
对 rx2 的字节调用 `ContentExtractor::feed`，在 content 更新时调用 `StatusPublisher::check_status`（需传入 shell_info 与 content）。

**Step 5: 验证**
Run: `RUSTUP_TOOLCHAIN=stable cargo run`，添加 workspace，确认单 pane 显示与输入正常。

**Step 6: Commit**
```
git add src/ui/app_root.rs src/ui/split_pane_container.rs ...
git commit -m "feat(terminal): wire gpui-terminal in single-pane flow"
```

---

## Phase 4: Multi-pane 与 Resize

### Task 8: setup_pane_terminal_output 接入 gpui-terminal

**Files:**
- Modify: src/ui/app_root.rs

**Step 1:** 对每个 pane 独立调用 tee_output、RuntimeReader、RuntimeWriter，创建 gpui_terminal view。
**Step 2:** 对接 gpui-terminal 的 `with_resize_callback`，内部调用 `runtime.resize(pane_id, cols, rows)`。
**Step 3:** SplitPaneContainer 中为每个 pane 渲染对应 gpui_terminal view。
**Step 4:** 验证 split、focus、resize。

**Step 5: Commit**
```
git commit -m "feat(terminal): multi-pane gpui-terminal support"
```

---

## Phase 5: 清理旧终端代码

### Task 9: 删除废弃模块并更新 design.md

**Files:**
- Delete: src/terminal/engine.rs
- Delete: src/terminal/renderable_snapshot.rs
- Delete: src/terminal/term_bridge.rs
- Delete: src/ui/terminal_rendering.rs
- Delete: src/ui/terminal_element.rs
- Delete: src/ui/terminal_renderer/ (整个目录)
- Modify: src/terminal/mod.rs, src/lib.rs, 所有引用上述模块的文件
- Modify: design.md

**Step 1:** 删除文件，更新 mod.rs 与 re-exports。
**Step 2:** 修复所有 `use crate::terminal::engine` 等引用。
**Step 3:** `cargo build` 和 `cargo test` 全部通过。
**Step 4: 更新 design.md**
修改以下节：
- **§5 技术栈**：`alacritty_terminal` 改为「gpui-terminal 内部使用」；新增 gpui-terminal；删除 terminal_rendering、term_bridge 等
- **§6.2 PTY Streaming**：流程图改为 `subscribe_output → TeePipe → (RuntimeReader → gpui_terminal, ContentExtractor → StatusPublisher)`；输入经 RuntimeWriter → send_input
- **默认 backend**：注明默认 tmux，可 config/env 切换

**Step 5: Commit**
```
git add -A design.md
git commit -m "refactor(terminal): remove legacy engine, use gpui-terminal; update design.md"
```

---

## Phase 6: 可选 — tmux 不可用时 Fallback

### Task 10: tmux 不可用时 fallback 到 local

**Files:**
- Modify: src/runtime/backends/mod.rs (create_runtime_from_env)
- Modify: src/ui/app_root.rs (错误提示)

**Step 1:** 在 `create_runtime_from_env` 中，当 backend 为 tmux 且 `TmuxRuntime::attach` 或创建失败时，若配置允许 fallback，则尝试 `create_runtime` (local)。
**Step 2:** 在 UI 显示 "tmux 不可用，已回退到 local" 或类似提示。
**Step 3: Commit**
```
git commit -m "feat(backends): fallback to local when tmux unavailable"
```

---

## Phase 7: 回归与功能测试

### Task 11: 运行回归和功能测试套件

**Files:**
- 无代码改动，仅执行测试

**Step 1: 运行 Rust 单元/集成测试**
Run: `RUSTUP_TOOLCHAIN=stable cargo test`
Expected: 全部 PASS（部分 macOS 上 gpui_macros 可能 SIGBUS，可单独跑 `cargo test --lib`）

**Step 2: 运行回归测试**
Run: `bash tests/regression/run_all.sh`（从仓库根目录执行）
Expected: 核心回归用例通过（窗口可见性、Sidebar 状态、光标位置、ANSI 颜色等）

**Step 3: 运行功能测试**
Run: `bash tests/functional/run_all.sh`（从仓库根目录执行）
Expected: Window、Workspace、Terminal、Pane、Input、Status 等模块测试通过

**Step 4: 运行完整套件（可选，一键执行回归+功能）**
Run: `bash tests/run_full_suite.sh --quick`（从仓库根目录执行）
Expected: 回归 + 功能测试通过；`--quick` 跳过性能与 E2E

**Step 5: 记录结果**
若任一失败，记录用例名与日志路径（如 `tests/regression/results/`、`tests/functional/results/`），修复后重新执行。

**Step 6: Commit**
若此前有修复：`git add -A && git commit -m "fix: address regression/functional test failures"`

---

## 验收标准

1. 单/多 pane 在 local 与 tmux 下均正常显示与输入
2. StatusPublisher 正确检测 agent 状态（OSC 133 + 文本 fallback）
3. Resize、focus、split 行为正常
4. 默认 backend 为 tmux，可经 config/env 切换
5. 无 tmux 时能 fallback 或给出明确错误
6. design.md 已更新，反映 gpui-terminal 架构与默认 backend
7. 回归测试与功能测试套件全部通过

---

## 依赖顺序

Phase 1 (Tasks 1–5) 可部分并行（2/3/4 可同时进行）。Phase 2 可在 Phase 1 完成后独立进行。Phase 3 依赖 Phase 1+2。Phase 4 依赖 Phase 3。Phase 5 依赖 Phase 4（含 design.md 更新）。Phase 6 可选。Phase 7（Task 11）为最后一步，在 Phase 5 完成后执行。
