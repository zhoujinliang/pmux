# Terminal Engine Phase 1: Core Architecture

## Objective

Build the foundational Terminal Engine structures: PtyReader thread, byte stream buffering, and TerminalState management using alacritty_terminal as the single source of truth.

## Success Criteria

- [ ] PTY reader thread uses blocking I/O with 65536 byte buffer
- [ ] Byte channel batches PTY output (no per-byte processing)
- [ ] TerminalEngine struct owns Term + Processor
- [ ] No visible_lines() or snapshot text parsing
- [ ] All state lives in alacritty_terminal::Term

## Architecture

```
┌────────────────────┐
│   PTY Reader Thread│  ← blocking read(fd), 64KB buffer
│  (spawn_pty_reader)│
└─────────┬──────────┘
          │ Vec<u8> chunks
          ▼
┌────────────────────┐
│  Byte Channel      │  ← flume::Receiver<Vec<u8>>
│  (batch, no async) │
└─────────┬──────────┘
          ▼
┌────────────────────┐
│  TerminalEngine    │
│  ├─ terminal: Term │  ← alacritty_terminal state
│  ├─ processor: VTE │  ← ansi::Processor
│  └─ byte_rx        │
└────────────────────┘
```

## Tasks

### T1. Create `terminal/engine.rs` Module

**File:** `src/terminal/engine.rs` (new)

Create the core TerminalEngine struct:

```rust
use alacritty_terminal::event::VoidListener;
use alacritty_terminal::term::{Term, Config};
use alacritty_terminal::vte::ansi::Processor;
use alacritty_terminal::grid::Dimensions;
use std::sync::{Arc, Mutex};

pub struct TerminalEngine {
    terminal: Arc<Mutex<Term<VoidListener>>>,
    processor: Arc<Mutex<Processor>>,
    byte_rx: flume::Receiver<Vec<u8>>,
}

impl TerminalEngine {
    pub fn new(
        columns: usize,
        screen_lines: usize,
        byte_rx: flume::Receiver<Vec<u8>>,
    ) -> Self {
        // Create Term with dimensions
        // Create Processor
        // Store byte_rx
    }

    /// Process all pending bytes from PTY channel
    pub fn advance_bytes(&self) {
        // Drain byte_rx with try_recv()
        // For each chunk, call processor.advance() on terminal
    }

    /// Get reference to terminal for rendering
    pub fn terminal(&self) -> std::sync::MutexGuard<'_, Term<VoidListener>> {
        self.terminal.lock().unwrap()
    }

    /// Resize terminal (separate from PTY resize)
    pub fn resize(&self, columns: usize, screen_lines: usize) {
        // Lock terminal and resize
    }
}
```

**Acceptance:**
- Compiles with `cargo check`
- Has unit test for creation
- Has unit test for advance_bytes

### T2. Create PTY Reader Thread

**File:** `src/terminal/pty_reader.rs` (new)

Extract and improve the PTY reader from `local_pty.rs`:

```rust
use std::os::fd::RawFd;
use std::thread;
use libc::{read, c_void};

/// Spawn a blocking PTY reader thread.
/// Returns the thread handle (can be ignored if daemon-like).
pub fn spawn_pty_reader(
    master_fd: RawFd,
    tx: flume::Sender<Vec<u8>>,
) -> thread::JoinHandle<()> {
    std::thread::spawn(move || {
        const BUF_SIZE: usize = 65536;
        let mut buf = vec![0u8; BUF_SIZE];

        loop {
            let n = unsafe {
                read(master_fd, buf.as_mut_ptr() as *mut c_void, BUF_SIZE)
            };

            if n <= 0 {
                // EOF or error - channel closed means terminal shutting down
                break;
            }

            let bytes = buf[..n as usize].to_vec();
            if tx.send(bytes).is_err() {
                break; // Receiver dropped
            }
        }
    })
}
```

**Key Changes from Current:**
- Buffer size: 65536 (was 4096)
- Direct libc::read (most efficient)
- No tokio, no async runtime
- Batch send entire chunks

**Acceptance:**
- Uses 65536 byte buffer
- No tokio dependencies
- Unit test with mock fd (if possible) or integration test

### T3. Refactor LocalPtyRuntime to Use Engine

**File:** `src/runtime/backends/local_pty.rs`

Modify `LocalPtyRuntime` to integrate with `TerminalEngine`:

```rust
pub struct LocalPtyRuntime {
    worktree_path: PathBuf,
    pane_id: PaneId,
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    // NEW: TerminalEngine instead of just output_rx
    engine: Arc<TerminalEngine>,
    pty_reader_handle: Option<thread::JoinHandle<()>>,
    cols: AtomicU16,
    rows: AtomicU16,
    _child: Arc<Mutex<Option<Box<dyn portable_pty::Child + Send + Sync>>>>,
}
```

**Constructor Changes:**
1. Create `(tx, rx)` channel
2. Spawn PTY reader with `spawn_pty_reader()` using raw fd
3. Create `TerminalEngine::new(cols, rows, rx)`
4. Store engine and reader handle

**Get Raw FD from portable_pty:**
```rust
// portable_pty::MasterPty has a `file_descriptor()` method
// that returns a trait object with `as_raw_fd()`
let fd = master.file_descriptor().as_raw_fd();
```

**Acceptance:**
- LocalPtyRuntime creates and owns TerminalEngine
- PTY reader thread spawned with correct fd
- Existing tests still pass

### T4. Update TermBridge to Use renderable_content

**File:** `src/terminal/term_bridge.rs`

Replace `visible_lines()` and `visible_lines_with_colors()` with `renderable_content()`:

```rust
use alacritty_terminal::term::RenderableContent;

/// Get renderable content for frame loop rendering
pub fn renderable_content(&self) -> RenderableContent<'_> {
    let term = self.term.lock().unwrap();
    term.renderable_content()
}

/// Get cursor info from renderable content
pub fn cursor_info(&self) -> Option<CursorPosition> {
    let term = self.term.lock().unwrap();
    let content = term.renderable_content();
    Some(content.cursor)
}
```

**Deprecate old methods (keep for now, mark with TODO):**
```rust
#[deprecated(note = "Use renderable_content instead")]
pub fn visible_lines(&self) -> Vec<String> { ... }
```

**Acceptance:**
- New `renderable_content()` method works
- Old methods still exist but marked deprecated
- Tests updated to use new method

### T5. Export Engine Module

**File:** `src/terminal/mod.rs`

```rust
pub mod engine;
pub mod pty_reader;

pub use engine::TerminalEngine;
pub use pty_reader::spawn_pty_reader;
```

**Acceptance:**
- `use crate::terminal::TerminalEngine;` works from other modules

## Verification

### Build
```bash
cargo check
cargo test terminal::
```

### Integration Test
Create `tests/terminal_engine_integration.rs`:

```rust
#[test]
fn test_terminal_engine_processes_pty_output() {
    // 1. Create PTY
    // 2. Create engine with channel
    // 3. Spawn reader
    // 4. Write "hello" to PTY
    // 5. Call engine.advance_bytes()
    // 6. Verify terminal contains "hello"
}
```

## Dependencies

- `alacritty_terminal` (already in Cargo.toml)
- `flume` (already in Cargo.toml)
- `portable-pty` (already in Cargo.toml)
- `libc` (add if not present)

## Notes

- This phase establishes the foundation. No UI changes yet.
- Frame loop comes in Phase 2.
- Current `visible_lines()` usage in UI will be migrated in Phase 2.
