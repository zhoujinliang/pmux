# tmux Hex Send-Keys Optimization

> **For Claude:** Use TDD when implementing. Consider `subagent-driven-development` for parallel tasks within each Phase.

**Goal:** Replace the multi-command `send-keys` path with a single `send-keys -H` (hex-encoded) command per input batch, reducing tmux command overhead from N commands to 1 per keystroke/paste batch.

**Architecture:** The writer thread already merges consecutive keystrokes into one `(pane_id, Vec<u8>)`. Currently it re-splits them into multiple `send-keys -l` / `send-keys -t %0 Enter` commands via `build_send_keys_commands`. Replace this with a single `send-keys -H -t %0 xx xx xx` command that sends raw bytes directly.

**Tech Stack:** Rust, tmux 2.6+ (`send-keys -H` flag)

**Verified:** `send-keys -H` works with tmux 3.6a for printable text, escape sequences (ESC `[` A = Up arrow), and control characters (0x0d = Enter, 0x03 = Ctrl-C).

---

## Overview

| Phase | Scope | Est. | Gate |
|-------|-------|------|------|
| **1** | Hex encoder + writer thread update | 30 min | Unit tests pass |
| **2** | Integration test + regression | 20 min | `cargo test` + regression 4/5 pass |

---

## Phase 1: Hex Encoder + Writer Thread

### Task 1.1: Add `build_hex_send_keys` function

**Files:**
- Modify: `src/runtime/backends/tmux_control_mode.rs`

**Step 1: Write failing unit tests**

Add these tests to the existing `#[cfg(test)] mod tests` block at the bottom of the file:

```rust
#[test]
fn test_build_hex_send_keys_empty() {
    let cmds = build_hex_send_keys("%0", &[]);
    assert!(cmds.is_empty());
}

#[test]
fn test_build_hex_send_keys_printable() {
    let cmds = build_hex_send_keys("%0", b"hello");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], "send-keys -H -t %0 68 65 6c 6c 6f");
}

#[test]
fn test_build_hex_send_keys_with_enter() {
    let cmds = build_hex_send_keys("%0", b"ls\r");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], "send-keys -H -t %0 6c 73 0d");
}

#[test]
fn test_build_hex_send_keys_escape_sequence() {
    // ESC [ A = Up arrow
    let cmds = build_hex_send_keys("%0", b"\x1b[A");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], "send-keys -H -t %0 1b 5b 41");
}

#[test]
fn test_build_hex_send_keys_ctrl_c() {
    let cmds = build_hex_send_keys("%0", b"\x03");
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], "send-keys -H -t %0 03");
}

#[test]
fn test_build_hex_send_keys_utf8() {
    // "你" = E4 BD A0
    let cmds = build_hex_send_keys("%0", "你".as_bytes());
    assert_eq!(cmds.len(), 1);
    assert_eq!(cmds[0], "send-keys -H -t %0 e4 bd a0");
}

#[test]
fn test_build_hex_send_keys_chunking() {
    // 600 bytes should produce 2 chunks (512 + 88)
    let data = vec![0x61u8; 600]; // 600 'a's
    let cmds = build_hex_send_keys("%0", &data);
    assert_eq!(cmds.len(), 2);
    // First chunk: 512 bytes → 512 hex pairs
    let hex_count_1 = cmds[0].trim_start_matches("send-keys -H -t %0 ")
        .split_whitespace().count();
    assert_eq!(hex_count_1, 512);
    // Second chunk: 88 bytes
    let hex_count_2 = cmds[1].trim_start_matches("send-keys -H -t %0 ")
        .split_whitespace().count();
    assert_eq!(hex_count_2, 88);
}
```

**Step 2: Run tests to verify failure**

```bash
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode::tests::test_build_hex -- --nocapture
```

Expected: FAIL — `build_hex_send_keys` not defined.

**Step 3: Write implementation**

Add this function right after the existing `build_send_keys_commands` function (around line 308):

```rust
/// Max raw bytes per single `send-keys -H` command.
/// 512 bytes → ~1600 hex chars + prefix ≈ 1650 bytes, well within tmux's line buffer.
const HEX_SEND_KEYS_CHUNK: usize = 512;

/// Build tmux `send-keys -H` commands for the given input bytes.
/// Encodes all bytes as hex, producing ONE command per chunk.
/// This is ~N× faster than `build_send_keys_commands` because tmux processes
/// one command instead of N (one per control character boundary).
fn build_hex_send_keys(pane_id: &str, bytes: &[u8]) -> Vec<String> {
    if bytes.is_empty() {
        return Vec::new();
    }
    bytes
        .chunks(HEX_SEND_KEYS_CHUNK)
        .map(|chunk| {
            let mut cmd = format!("send-keys -H -t {}", pane_id);
            for &b in chunk {
                use std::fmt::Write;
                write!(cmd, " {:02x}", b).unwrap();
            }
            cmd
        })
        .collect()
}
```

**Step 4: Run tests to verify pass**

```bash
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode::tests::test_build_hex -- --nocapture
```

Expected: all 7 tests PASS.

**Step 5: Commit**

```bash
git add src/runtime/backends/tmux_control_mode.rs
git commit -m "feat: add build_hex_send_keys for single-command input encoding"
```

---

### Task 1.2: Update writer thread to use hex send-keys

**Files:**
- Modify: `src/runtime/backends/tmux_control_mode.rs` (writer thread, lines ~487–530)

**Step 1: Replace `build_send_keys_commands` with `build_hex_send_keys` in the writer thread**

Find this block in the writer thread (around line 516–522):

```rust
// BEFORE (current code):
cmd_buf.clear();
for (pane_id, bytes) in &merged {
    for cmd in build_send_keys_commands(pane_id, bytes) {
        cmd_buf.extend_from_slice(cmd.as_bytes());
        cmd_buf.push(b'\n');
    }
}
```

Replace with:

```rust
// AFTER (hex send-keys):
cmd_buf.clear();
for (pane_id, bytes) in &merged {
    for cmd in build_hex_send_keys(pane_id, bytes) {
        cmd_buf.extend_from_slice(cmd.as_bytes());
        cmd_buf.push(b'\n');
    }
}
```

This is a one-line function name change. The rest of the writer thread (recv, merge, write_all, flush) stays exactly the same.

**Step 2: Verify build**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

Expected: compiles with no errors.

**Step 3: Run all unit tests**

```bash
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode::tests -- --nocapture
```

Expected: all tests pass. The runtime tests that require tmux (`test_control_mode_*`) should pass because `send-keys -H` is a valid tmux command.

**Step 4: Commit**

```bash
git add src/runtime/backends/tmux_control_mode.rs
git commit -m "perf: use hex send-keys in writer thread (N commands → 1 per batch)"
```

---

## Phase 2: Integration Test + Regression

### Task 2.1: Integration test — hex send-keys end-to-end

**Files:**
- Modify: `src/runtime/backends/tmux_control_mode.rs` (add test to `mod tests`)

**Step 1: Add integration test**

Add this test to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn test_hex_send_keys_roundtrip() {
    if !crate::runtime::backends::tmux_available() {
        eprintln!("skipping: tmux not available");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let rt = TmuxControlModeRuntime::new("pmux-test-hex", "main", Some(dir.path()), 80, 24)
        .expect("should create runtime");

    let panes = rt.list_panes(&String::new());
    let pane_id = panes.first().cloned().unwrap_or_else(|| "%0".to_string());

    // Subscribe to output
    let rx = rt.subscribe_output(&pane_id).expect("should get receiver");

    // Wait for shell prompt
    std::thread::sleep(std::time::Duration::from_millis(500));
    while rx.try_recv().is_ok() {} // drain initial output

    // Send "echo HEX_OK\r" via send_input (which now uses hex internally)
    let input = b"echo HEX_OK\r";
    rt.send_input(&pane_id, input).expect("send_input should work");

    // Wait and collect output
    std::thread::sleep(std::time::Duration::from_millis(1000));
    let mut output = Vec::new();
    while let Ok(chunk) = rx.try_recv() {
        output.extend_from_slice(&chunk);
    }

    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("HEX_OK"),
        "expected 'HEX_OK' in output, got: {}",
        output_str
    );

    // Cleanup
    let _ = Command::new("tmux")
        .args(["kill-session", "-t", "pmux-test-hex"])
        .output();
}
```

**Step 2: Run integration test**

```bash
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode::tests::test_hex_send_keys_roundtrip -- --nocapture
```

Expected: PASS — "HEX_OK" appears in the captured output.

**Step 3: Commit**

```bash
git add src/runtime/backends/tmux_control_mode.rs
git commit -m "test: add hex send-keys roundtrip integration test"
```

---

### Task 2.2: Full test suite + regression

**Step 1: Run cargo test**

```bash
RUSTUP_TOOLCHAIN=stable cargo test 2>&1 | tail -10
```

Expected: test result: ok.

**Step 2: Build release and run regression tests**

```bash
cd /Users/matt.chow/workspace/pmux
RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3
bash tests/regression/run_all.sh --skip-build
```

Expected: 4/5 pass (echo_output test is a known flake from before — screenshot timing issue).

**Step 3: Manual smoke test**

```bash
RUSTUP_TOOLCHAIN=stable cargo run
```

Verify:
- [ ] Open a workspace → terminal appears
- [ ] Type `ls` + Enter → output displays correctly
- [ ] Type `echo "hello world"` → correct output
- [ ] Fast typing (mash keyboard) → no dropped characters
- [ ] Ctrl+C → interrupts running process
- [ ] Up/Down arrows → navigate history
- [ ] Tab → autocomplete works
- [ ] Paste a long string (50+ chars) → appears correctly
- [ ] vim → full TUI works, insert/normal mode switching
- [ ] CJK input: `echo "你好"` → correct display

---

## What Changed (summary for review)

| Before | After |
|--------|-------|
| `build_send_keys_commands` splits bytes at control char boundaries → N commands per batch | `build_hex_send_keys` encodes all bytes as hex → 1 command per ≤512-byte chunk |
| Typing "hello\r" → 2 commands (`send-keys -l 'hello'` + `send-keys Enter`) | Typing "hello\r" → 1 command (`send-keys -H 68 65 6c 6c 6f 0d`) |
| Paste 100 chars with special chars → 5-10 commands | Paste 100 chars → 1 command |
| tmux processes N commands (parse, lookup, write) per input | tmux processes 1 command per input |

**Risk:** Low. The writer thread architecture is unchanged. Only the command format changes from named/literal to hex-encoded. `build_send_keys_commands` is kept for backward reference and `send_key` method.

**Requires:** tmux 2.6+ (released Oct 2017) for `-H` flag. tmux 3.6a verified.
