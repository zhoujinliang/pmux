# Terminal Engine Phase 4: Integration & Polish

## Objective

Integrate all components, replace legacy terminal handling, and verify performance. Clean up deprecated code and ensure TUI applications work correctly.

## Success Criteria

- [ ] All panes use TerminalEngine (no legacy paths)
- [ ] TUI apps (vim, Claude Code, lazygit) work correctly
- [ ] CPU usage < 10% during normal operation
- [ ] No polling loops in terminal path
- [ ] All tests pass

## Tasks

### T1. Migrate All Pane Creation to TerminalEngine

**File:** `src/ui/app_root.rs` (and related)

Find all places that create TerminalView and ensure they use the new engine:

```rust
// OLD:
let term_bridge = Arc::new(Mutex::new(TermBridge::new(cols, rows)));
let buffer = TerminalBuffer::Term(term_bridge.clone());
let view = TerminalView::with_buffer(&pane_id, &title, buffer);

// NEW:
let (tx, rx) = flume::unbounded();
let engine = Arc::new(TerminalEngine::new(cols, rows, rx));

// Spawn PTY reader with the engine's channel
let pty_reader = spawn_pty_reader(master_fd, tx);

// Store engine in pane state
let pane_state = PaneState {
    pane_id: pane_id.clone(),
    engine: Some(engine.clone()),
    view: TerminalView::with_engine(&pane_id, &title, engine),
};
```

**Files to Check:**
- `src/ui/app_root.rs` - `start_tmux_session()` equivalent for local PTY
- `src/workspace_manager.rs` - workspace switching
- `src/split_tree.rs` - pane creation for splits

**Acceptance:**
- All TerminalView creation uses TerminalEngine
- No direct TermBridge creation in UI code

### T2. Remove Deprecated visible_lines() Usage

**File:** `src/status_detector.rs`, `src/ui/app_root.rs`

Find all uses of `visible_lines()` and migrate:

```rust
// status_detector.rs OLD:
let content = term_bridge.visible_lines().join("\n");

// NEW:
let content = {
    let term = engine.terminal();
    let renderable = term.renderable_content();
    renderable.display_iter()
        .map(|cell| cell.c)
        .collect::<String>()
};
```

**Search for:**
```bash
grep -rn "visible_lines" src/
```

**Acceptance:**
- Zero non-test uses of `visible_lines()`
- Status detection works with new API

### T3. Clean Up Old Polling Code

**File:** `src/runtime/status_publisher.rs`

The status publisher currently polls. If we're moving to event-driven:

```rust
// Option 1: Keep but increase interval significantly
const STATUS_POLL_INTERVAL: Duration = Duration::from_secs(2); // was 500ms

// Option 2: Remove polling entirely, use process events only
// See Phase C of runtime-completion design.md
```

For this phase, just increase the interval to reduce CPU:

```rust
// In status_publisher.rs:
thread::sleep(Duration::from_secs(2)); // was 500ms
```

**Acceptance:**
- Status polling interval >= 2 seconds
- Or: event-driven status (if implemented)

### T4. Implement TUI Detection in UI

**File:** `src/ui/terminal_view.rs`

Ensure TUI detection works with new renderable_content:

```rust
impl TerminalView {
    fn is_tui_active(&self) -> bool {
        match &self.buffer {
            TerminalBuffer::Engine(engine) => {
                let term = engine.terminal();
                term.mode().contains(TermMode::ALT_SCREEN)
            }
            _ => false,
        }
    }
}
```

**Acceptance:**
- `is_tui_active()` uses TerminalEngine
- Cursor hidden when vim/Claude Code active
- Cursor shown in normal shell

### T5. Add Performance Metrics (Optional)

**File:** `src/terminal/engine.rs`

Add debug metrics:

```rust
pub struct TerminalEngine {
    terminal: Arc<Mutex<Term<VoidListener>>>,
    processor: Arc<Mutex<Processor>>,
    byte_rx: flume::Receiver<Vec<u8>>,

    // Metrics (debug builds only)
    #[cfg(debug_assertions)]
    metrics: EngineMetrics,
}

#[cfg(debug_assertions)]
struct EngineMetrics {
    bytes_processed: AtomicU64,
    frames_rendered: AtomicU64,
    last_log: Instant,
}

impl TerminalEngine {
    pub fn log_metrics(&self) {
        #[cfg(debug_assertions)]
        {
            let bytes = self.metrics.bytes_processed.load(Ordering::Relaxed);
            let frames = self.metrics.frames_rendered.load(Ordering::Relaxed);
            let ratio = bytes as f64 / frames as f64;
            log::debug!("TerminalEngine: {} bytes/frame over {} frames", ratio, frames);
        }
    }
}
```

**Acceptance:**
- Metrics show batching is working (high bytes/frame ratio)

### T6. TUI Application Test Suite

**File:** `tests/tui_compatibility.rs` (new)

```rust
#[test]
fn test_vim_resizing() {
    // 1. Create terminal
    // 2. Send "vim\n"
    // 3. Wait for alternate screen
    // 4. Resize
    // 5. Verify vim redraws (no corruption)
}

#[test]
fn test_claude_code_cursor() {
    // 1. Create terminal
    // 2. Send "claude\n"
    // 3. Wait for alternate screen
    // 4. Send arrow keys
    // 5. Verify cursor position tracks correctly
}

#[test]
fn test_lazygit_navigation() {
    // 1. Create terminal in git repo
    // 2. Send "lazygit\n"
    // 3. Navigate with arrow keys
    // 4. Verify screen updates correctly
}
```

**Acceptance:**
- Tests pass (may need manual verification for some)
- Documentation of supported TUI apps

### T7. Final Integration Test

**File:** `tests/terminal_engine_integration.rs`

Complete end-to-end test:

```rust
#[test]
fn test_terminal_engine_end_to_end() {
    use std::time::Duration;

    // 1. Setup
    let temp_dir = tempfile::tempdir().unwrap();
    let (tx, rx) = flume::unbounded();

    // 2. Create PTY
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }).unwrap();

    let master_fd = pair.master.file_descriptor().as_raw_fd();
    let mut writer = pair.master.take_writer().unwrap();

    // 3. Spawn shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.cwd(temp_dir.path());
    let _child = pair.slave.spawn_command(cmd).unwrap();

    // 4. Spawn reader
    let reader_handle = spawn_pty_reader(master_fd, tx);

    // 5. Create engine
    let engine = TerminalEngine::new(80, 24, rx);

    // 6. Write command
    writer.write_all(b"echo 'hello world'\n").unwrap();
    writer.flush().unwrap();

    // 7. Wait and process
    std::thread::sleep(Duration::from_millis(500));
    engine.advance_bytes();

    // 8. Verify output
    let term = engine.terminal();
    let content = term.renderable_content();
    let text: String = content.display_iter().map(|c| c.c).collect();
    assert!(text.contains("hello world"));

    // 9. Cleanup
    drop(writer);
    drop(pair.master);
    reader_handle.join().ok();
}
```

**Acceptance:**
- Integration test passes
- No deadlocks
- Clean shutdown

### T8. Performance Benchmark

**File:** `benches/terminal_engine.rs` (new)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_frame_processing(c: &mut Criterion) {
    c.bench_function("process_10kb_frame", |b| {
        let (tx, rx) = flume::unbounded();
        let engine = TerminalEngine::new(80, 24, rx);

        // Send 10KB of typical terminal output
        let data = vec![b'x'; 10_240];
        tx.send(data).unwrap();

        b.iter(|| {
            engine.advance_bytes();
            black_box(&engine);
        });
    });
}

criterion_group!(benches, benchmark_frame_processing);
criterion_main!(benches);
```

**Acceptance:**
- Benchmark runs
- Frame processing < 1ms for 10KB

### T9. Documentation Updates

**Files:**
- `CLAUDE.md` - Update architecture section
- `docs/plans/README.md` - Mark old plans as superseded
- Add `docs/terminal_engine.md` - New architecture doc

**Acceptance:**
- Documentation reflects new architecture
- Clear migration guide from old API

### T10. Deprecation Cleanup

**After verification, remove:**

1. `TermBridge::visible_lines()` - delete or mark #[deprecated]
2. `TermBridge::visible_lines_with_colors()` - delete or mark #[deprecated]
3. Old polling code in status_publisher (if event-driven replacement ready)
4. Any tmux-specific code in UI layer (should be in backend only)

**Acceptance:**
- `cargo check` clean
- No dead code warnings

## Verification Checklist

### Functionality
- [ ] vim opens and edits files
- [ ] Claude Code runs with correct cursor
- [ ] lazygit works (if installed)
- [ ] neovim works
- [ ] Scrollback preserved in all apps
- [ ] Resize works in all apps

### Performance
- [ ] CPU < 10% during idle
- [ ] CPU < 20% during heavy output (e.g., `yes` command)
- [ ] No visible lag when typing
- [ ] Smooth scrolling

### Code Quality
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] No `visible_lines` usage in production code
- [ ] No polling in frame path

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Input latency | < 1ms | Instrumentation |
| Frame time | < 16ms | Debug metrics |
| CPU (idle) | < 5% | Activity Monitor |
| CPU (TUI) | < 10% | Activity Monitor |
| Memory | < 100MB | Activity Monitor |
| Test pass rate | 100% | `cargo test` |

## Rollback Plan

If issues occur:

1. **Phase 4.1:** Keep old code behind feature flag
   ```rust
   #[cfg(feature = "legacy-terminal")]
   pub use term_bridge::TermBridge;
   ```

2. **Phase 4.2:** Runtime backend selection
   ```rust
   pub enum TerminalBackend {
       Legacy,  // Old implementation
       Engine, // New implementation
   }
   ```

3. **Phase 4.3:** Emergency revert to commit before Phase 1

## Notes

- This is a significant refactoring - test thoroughly
- The frame loop architecture is the key to performance
- TUI detection is critical for cursor behavior
- SIGWINCH handling is critical for resize correctness

## Completion

After Phase 4:
- pmux has a world-class terminal engine
- Performance matches Ghostty/Alacritty/Zed
- TUI apps work flawlessly
- Code is clean and maintainable
