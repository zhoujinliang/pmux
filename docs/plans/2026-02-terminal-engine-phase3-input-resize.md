# Terminal Engine Phase 3: Input & Resize Handling

## Objective

Implement direct PTY write for keyboard input (no async hop) and correct resize sequence (winsize → SIGWINCH → render).

## Success Criteria

- [ ] Keyboard input writes directly to PTY (same thread)
- [ ] No async/await in input path
- [ ] Resize: winsize → SIGWINCH → wait → render
- [ ] Claude Code cursor correct after resize
- [ ] vim/neovim resize correctly

## Architecture

### Input Path
```
Key Event (GPUI)
      │
      ▼ (same thread)
┌─────────────────┐
│ key_to_xterm()  │  ← convert to escape sequence
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ libc::write()   │  ← direct PTY write
│ (nonblocking)   │
└─────────────────┘
```

### Resize Sequence
```
UI Resize Event
      │
      ▼
┌─────────────────┐
│ pty.resize()    │  ← set winsize
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ kill(SIGWINCH)  │  ← notify process
└────────┬────────┘
         │
         ▼ (wait one frame)
┌─────────────────┐
│ engine.resize() │  ← resize alacritty_terminal
│ cx.notify()     │  ← trigger render
└─────────────────┘
```

## Tasks

### T1. Create Direct PTY Writer

**File:** `src/terminal/pty_writer.rs` (new)

```rust
use std::os::fd::RawFd;
use libc::{write, c_void};

/// Direct PTY writer for nonblocking writes.
pub struct PtyWriter {
    master_fd: RawFd,
}

impl PtyWriter {
    pub fn new(master_fd: RawFd) -> Self {
        Self { master_fd }
    }

    /// Write bytes directly to PTY.
    /// Must be called on the same thread (no async).
    pub fn write(&self, bytes: &[u8]) -> std::io::Result<usize> {
        let n = unsafe {
            write(self.master_fd, bytes.as_ptr() as *const c_void, bytes.len())
        };

        if n < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    /// Write all bytes (retry on partial write).
    pub fn write_all(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut written = 0;
        while written < bytes.len() {
            match self.write(&bytes[written..]) {
                Ok(0) => return Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "write returned 0"
                )),
                Ok(n) => written += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl Clone for PtyWriter {
    fn clone(&self) -> Self {
        Self {
            master_fd: self.master_fd,
        }
    }
}

unsafe impl Send for PtyWriter {}
unsafe impl Sync for PtyWriter {}
```

**Acceptance:**
- Direct libc::write (no async)
- Handles partial writes
- Thread-safe (Send + Sync)

### T2. Update Input Handler

**File:** `src/input_handler.rs` (or create if not exists)

Current input path analysis:
- Check `src/input/mod.rs` and xterm_escape.rs
- Input currently goes through `AgentRuntime::send_input()`

**New Input Path:**
```rust
pub struct InputHandler {
    pty_writer: PtyWriter,
}

impl InputHandler {
    pub fn new(pty_writer: PtyWriter) -> Self {
        Self { pty_writer }
    }

    /// Handle key event - called on main thread during event processing.
    pub fn on_key(&self, key: &KeyEvent) -> Result<(), InputError> {
        let bytes = key_to_xterm_escape(key)?;
        self.pty_writer.write_all(&bytes)?;
        Ok(())
    }
}
```

**Integration with GPUI:**
```rust
// In AppRoot or TerminalView:
impl TerminalView {
    fn handle_key_event(&mut self, event: &KeyEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Direct write - no spawn, no async
        if let Err(e) = self.input_handler.on_key(event) {
            log::error!("Input write failed: {}", e);
        }
        // No cx.notify() needed - PTY output will trigger next frame
    }
}
```

**Acceptance:**
- Key event → PTY write happens on same thread
- No `cx.spawn()` or `async` in input path
- < 1ms latency

### T3. Implement Resize with SIGWINCH

**File:** `src/runtime/backends/local_pty.rs`

Add proper resize sequence:

```rust
use libc::{kill, SIGWINCH, getpgrp};
use std::process::Child;

impl LocalPtyRuntime {
    pub fn resize_with_signal(
        &self,
        pane_id: &PaneId,
        cols: u16,
        rows: u16,
    ) -> Result<(), RuntimeError> {
        if pane_id != &self.pane_id {
            return Err(RuntimeError::PaneNotFound(pane_id.clone()));
        }

        // 1. Get child PID for signal
        let child_pid = {
            let guard = self._child.lock()
                .map_err(|e| RuntimeError::Backend(e.to_string()))?;
            guard.as_ref().map(|c| c.pid())
        };

        // 2. Resize PTY winsize
        self.resize_pty(cols, rows)?;

        // 3. Send SIGWINCH to process group
        if let Some(pid) = child_pid {
            unsafe {
                // Send to process group (negative PID)
                kill(-(pid as i32), SIGWINCH);
            }
        }

        // 4. Store new dimensions
        self.cols.store(cols, Ordering::SeqCst);
        self.rows.store(rows, Ordering::SeqCst);

        // 5. Resize the TerminalEngine (alacritty_terminal)
        if let Some(engine) = &self.engine {
            engine.resize(cols as usize, rows as usize);
        }

        Ok(())
    }

    fn resize_pty(&self, cols: u16, rows: u16) -> Result<(), RuntimeError> {
        let guard = self.master.lock()
            .map_err(|e| RuntimeError::Backend(e.to_string()))?;
        guard.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }).map_err(|e| RuntimeError::Backend(e.to_string()))
    }
}
```

**Note:** Need to get child PID. Check if `portable_pty::Child` has `pid()` method.

**Acceptance:**
- Resize sequence: winsize → SIGWINCH → engine.resize
- vim resizes correctly
- Claude Code cursor correct after resize

### T4. Handle Window Resize Events

**File:** `src/ui/app_root.rs`

Connect GPUI window resize to PTY resize:

```rust
impl AppRoot {
    fn on_window_resize(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Calculate new dimensions from window size
        let (cols, rows) = self.calculate_terminal_dimensions(window);

        // Resize all visible panes
        for (_, pane_state) in &mut self.pane_states {
            if let Some(runtime) = &pane_state.runtime {
                let pane_id = &pane_state.pane_id;
                if let Err(e) = runtime.resize_with_signal(pane_id, cols, rows) {
                    log::error!("Resize failed for pane {}: {}", pane_id, e);
                }
            }
        }

        // Render will happen automatically on next frame
    }

    fn calculate_terminal_dimensions(&self, window: &Window) -> (u16, u16) {
        // Calculate based on font size and window dimensions
        // Typical: 8px per char, 20px per line
        let size = window.viewport_size();
        let cols = (size.width.0 as u16 / 8).max(80);
        let rows = (size.height.0 as u16 / 20).max(24);
        (cols, rows)
    }
}
```

**Acceptance:**
- Window resize triggers PTY resize
- All panes updated
- No manual cx.notify() needed (frame loop handles it)

### T5. Add Nonblocking PTY Support

**File:** `src/terminal/pty_reader.rs`

Ensure PTY is nonblocking for reads (but we want blocking for efficiency in the reader thread):

```rust
// The reader thread SHOULD block - that's the point.
// But we need to ensure clean shutdown.

pub fn spawn_pty_reader(
    master_fd: RawFd,
    tx: flume::Sender<Vec<u8>>,
    shutdown_rx: flume::Receiver<()>,  // Add shutdown signal
) -> thread::JoinHandle<()> {
    std::thread::spawn(move || {
        const BUF_SIZE: usize = 65536;
        let mut buf = vec![0u8; BUF_SIZE];

        loop {
            // Check for shutdown (nonblocking)
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            // Blocking read - this is what we want
            let n = unsafe {
                read(master_fd, buf.as_mut_ptr() as *mut c_void, BUF_SIZE)
            };

            if n <= 0 {
                break;
            }

            let bytes = buf[..n as usize].to_vec();
            if tx.send(bytes).is_err() {
                break;
            }
        }
    })
}
```

**Acceptance:**
- Reader thread blocks on read (correct)
- Can be shut down cleanly
- No busy-waiting

## Verification

### Input Latency Test
```rust
#[test]
fn test_input_latency() {
    let start = Instant::now();
    input_handler.on_key(&KeyEvent::char('a')).unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(1));
}
```

### Resize Test
```bash
# 1. Open pmux with vim
# 2. Resize window
# 3. Verify vim redraws correctly
# 4. Verify cursor position correct
```

### Claude Code Test
```bash
# 1. Open Claude Code in pmux
# 2. Resize window multiple times
# 3. Check cursor alignment
# 4. Check selection highlighting
```

## Common Issues

### Issue: Cursor Wrong After Resize
**Cause:** Rendering before PTY processes SIGWINCH
**Fix:** Ensure render happens one frame after SIGWINCH

### Issue: Input Lag
**Cause:** Async hop in input path
**Fix:** Direct write in event handler

### Issue: Partial Writes
**Cause:** PTY buffer full
**Fix:** write_all() with retry loop

## Dependencies

- `libc` for `write()`, `kill()`, `SIGWINCH`
- No tokio in input path

## Notes

- Input latency is critical - any async adds 10-100ms
- Resize must signal the process, not just change buffer size
- Frame loop naturally debounces resize renders
