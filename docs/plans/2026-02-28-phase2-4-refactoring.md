# Phase 2-4: Complete Architecture Refactoring

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the remaining refactoring phases: implement BackendPane trait, remove text-based status detection, and eliminate polling loops.

**Architecture:** 
- Phase 2: Define BackendPane trait as the stable interface for PTY operations, implemented by TmuxPane and LocalPtyPane
- Phase 3: Remove status_detector.rs, replace with process lifecycle events
- Phase 4: Replace frame_tick polling with event-driven PTY output

**Tech Stack:** Rust, flume channels, tokio, portable_pty

---

## Background

Phase 1 completed: UI decoupled from backend implementations. Now we complete the remaining violations:

| Phase | Issue | Design Rule |
|-------|-------|-------------|
| 2 | Missing BackendPane trait | Data model incomplete |
| 3 | Text parsing for status | RULE 3, RULE 4 |
| 4 | frame_tick polling | Event-driven required |

---

## Phase 2: Implement BackendPane Trait

### Task 1: Define BackendPane Trait

**Files:**
- Create: `src/runtime/backend_pane.rs`

**Step 1: Write the trait definition**

Create `src/runtime/backend_pane.rs`:

```rust
//! BackendPane trait - stable interface for PTY operations.
//!
//! This trait abstracts the PTY layer so different backends (tmux, local_pty)
//! can be swapped without changing the terminal engine.

use std::sync::Arc;
use crate::runtime::agent_runtime::RuntimeError;

pub type PaneId = String;

/// BackendPane - stable interface for PTY operations.
///
/// RULE 2: Terminal Engine doesn't know Agent concept.
/// RULE 3: Backend doesn't parse terminal text.
/// This trait only handles raw byte streams.
pub trait BackendPane: Send + Sync {
    /// Subscribe to PTY output stream.
    /// Returns a receiver that yields raw bytes from the PTY.
    fn subscribe_output(&self) -> flume::Receiver<Vec<u8>>;

    /// Write bytes to the PTY (user input).
    fn write(&self, bytes: &[u8]) -> Result<(), RuntimeError>;

    /// Resize the PTY.
    fn resize(&self, cols: u16, rows: u16) -> Result<(), RuntimeError>;

    /// Kill the PTY process.
    fn kill(&self) -> Result<(), RuntimeError>;

    /// Get the pane ID.
    fn pane_id(&self) -> &PaneId;

    /// Get current dimensions.
    fn dimensions(&self) -> (u16, u16);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_pane_trait_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn BackendPane>>();
    }
}
```

**Step 2: Add to runtime/mod.rs**

Add to `src/runtime/mod.rs`:

```rust
pub mod backend_pane;
pub use backend_pane::BackendPane;
```

**Step 3: Verify compilation**

Run: `cargo build --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/runtime/backend_pane.rs src/runtime/mod.rs
git commit -m "feat(runtime): define BackendPane trait"
```

---

### Task 2: Implement BackendPane for LocalPtyRuntime

**Files:**
- Modify: `src/runtime/backends/local_pty.rs`

**Step 1: Add BackendPane import**

Add to imports:
```rust
use crate::runtime::backend_pane::BackendPane;
```

**Step 2: Implement BackendPane trait**

Add at end of file:

```rust
impl BackendPane for LocalPtyRuntime {
    fn subscribe_output(&self) -> flume::Receiver<Vec<u8>> {
        let pane_id = self.pane_id();
        self.subscribe_output(&pane_id.to_string()).unwrap_or_else(|| {
            flume::unbounded().1
        })
    }

    fn write(&self, bytes: &[u8]) -> Result<(), RuntimeError> {
        self.send_input(&self.pane_id, bytes)
    }

    fn resize(&self, cols: u16, rows: u16) -> Result<(), RuntimeError> {
        AgentRuntime::resize(self, &self.pane_id, cols, rows)
    }

    fn kill(&self) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn pane_id(&self) -> &PaneId {
        &self.pane_id
    }

    fn dimensions(&self) -> (u16, u16) {
        AgentRuntime::get_pane_dimensions(self, &self.pane_id)
    }
}
```

**Step 3: Verify compilation**

Run: `cargo build --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/runtime/backends/local_pty.rs
git commit -m "feat(local_pty): implement BackendPane trait"
```

---

### Task 3: Add tests for BackendPane

**Files:**
- Modify: `src/runtime/backends/local_pty.rs`

**Step 1: Add test**

```rust
#[cfg(test)]
mod backend_pane_tests {
    use super::*;
    use crate::runtime::backend_pane::BackendPane;
    use std::path::PathBuf;

    #[test]
    fn test_local_pty_implements_backend_pane() {
        fn assert_backend_pane<T: BackendPane>() {}
        assert_backend_pane::<LocalPtyRuntime>();
    }
}
```

**Step 2: Run test**

Run: `cargo test backend_pane_tests --lib`
Expected: PASS

**Step 3: Commit**

```bash
git add src/runtime/backends/local_pty.rs
git commit -m "test(local_pty): add BackendPane trait test"
```

---

## Phase 3: Remove Text-Based Status Detection

### Task 4: Remove status_detector.rs

**Files:**
- Delete: `src/status_detector.rs`
- Modify: `src/lib.rs`

**Step 1: Find all usages**

Run: `grep -rn "status_detector" src/`
Note all files that import it.

**Step 2: Remove imports**

Remove `mod status_detector;` from `src/lib.rs` and any `use crate::status_detector` statements.

**Step 3: Delete the file**

Run: `rm src/status_detector.rs`

**Step 4: Verify compilation**

Run: `cargo build --lib`
Expected: Compilation errors about missing status_detector - note them.

**Step 5: Fix compilation errors**

For each file that used status_detector:
- If used for status display, use AgentStatus::Unknown or Idle as default
- If used in tests, remove or update the test

**Step 6: Commit**

```bash
git add -A
git commit -m "refactor: remove status_detector.rs (text-based status detection)"
```

---

### Task 5: Update status_publisher to use lifecycle events

**Files:**
- Modify: `src/runtime/status_publisher.rs`

**Step 1: Read current implementation**

Check if `update_from_content()` exists and how it's used.

**Step 2: Remove text parsing logic**

Remove any methods that parse terminal content to detect status.

**Step 3: Add lifecycle-based status**

If needed, add method to receive ProcessEvent:

```rust
pub enum ProcessEvent {
    Started,
    Exited { code: i32 },
    Signal { signal: i32 },
}

impl StatusPublisher {
    pub fn on_process_event(&self, pane_id: &str, event: ProcessEvent) {
        match event {
            ProcessEvent::Started => {
                self.update_status(pane_id, AgentStatus::Running);
            }
            ProcessEvent::Exited { code } => {
                if code == 0 {
                    self.update_status(pane_id, AgentStatus::Idle);
                } else {
                    self.update_status(pane_id, AgentStatus::Error);
                }
            }
            ProcessEvent::Signal { .. } => {
                self.update_status(pane_id, AgentStatus::Error);
            }
        }
    }
}
```

**Step 4: Commit**

```bash
git add src/runtime/status_publisher.rs
git commit -m "refactor(status_publisher): use lifecycle events instead of text parsing"
```

---

## Phase 4: Eliminate Polling Loops

### Task 6: Identify polling locations

**Files:**
- `src/ui/app_root.rs` - frame_tick
- `src/runtime/backends/local_pty.rs` - process monitoring

**Step 1: Find frame_tick**

Run: `grep -n "frame_tick" src/ui/app_root.rs`

**Step 2: Find polling loops**

Run: `grep -n "loop\|poll\|sleep" src/runtime/backends/local_pty.rs`

**Step 3: Document findings**

Note which lines need to be changed.

---

### Task 7: Replace frame_tick polling with event-driven output

**Files:**
- Modify: `src/ui/app_root.rs`

**Context:** frame_tick runs every 16ms to process PTY bytes. Instead, PTY output should trigger UI updates via channel.

**Step 1: Understand current flow**

The current frame_tick likely:
1. Checks for new bytes from PTY
2. Feeds to terminal engine
3. Triggers repaint

**Step 2: Replace with channel-based approach**

The PTY already has `subscribe_output()` returning a flume::Receiver. The UI should:
1. Spawn a task that listens on the receiver
2. On receiving bytes, feed to engine and call `cx.notify()`

**Step 3: Verify**

Run: `cargo build --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/ui/app_root.rs
git commit -m "refactor(ui): replace frame_tick polling with event-driven PTY output"
```

---

### Task 8: Replace process monitoring polling

**Files:**
- Modify: `src/runtime/backends/local_pty.rs`

**Context:** Process monitoring currently uses `loop { sleep; try_wait }`. Should use async wait or signal.

**Step 1: Find the polling loop**

Look for `try_wait` or similar patterns.

**Step 2: Replace with tokio::signal or async wait**

If process monitoring is needed, use:
- `tokio::signal::ctrl_c()` for signals
- `tokio::process::Child` for async wait

**Step 3: Verify**

Run: `cargo build --lib`
Expected: PASS

**Step 4: Commit**

```bash
git add src/runtime/backends/local_pty.rs
git commit -m "refactor(local_pty): replace process polling with event-driven monitoring"
```

---

## Verification

### Task 9: Final verification

**Step 1: Check for remaining polling**

Run: `grep -rn "loop {" src/runtime/ src/ui/ | grep -v test`

**Step 2: Check for text parsing**

Run: `grep -rn "Regex\|regex" src/runtime/ | grep -v test`

**Step 3: Check BackendPane usage**

Run: `grep -rn "BackendPane" src/`

**Step 4: Run all tests**

Run: `cargo test --lib`

**Step 5: Build release**

Run: `cargo build --release`

---

## Summary

After completing all tasks:

- [ ] BackendPane trait defined and implemented
- [ ] status_detector.rs removed
- [ ] Status from lifecycle events, not text parsing
- [ ] No polling loops in runtime or UI
- [ ] All tests pass
- [ ] Release build succeeds