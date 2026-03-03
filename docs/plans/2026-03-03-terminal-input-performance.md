# Terminal Input & Rendering Performance Plan

> **For Claude:** Direct implementation on main branch. Run regression tests after each phase. No TDD for performance work — measure before/after instead.

**Goal:** Bring terminal input latency and UI responsiveness to parity with native system terminals (iTerm2, Alacritty).

**Architecture:** Three-phase fix targeting the full pipeline: input → processing → rendering. Each phase is independently shippable and testable.

**Tech Stack:** Rust, GPUI, alacritty_terminal, flume channels

**Root Causes Identified:**
1. tmux-cc `send_input` blocks UI thread with synchronous PTY write + flush per keystroke
2. Every PTY output chunk triggers 2x `cx.notify()` → full AppRoot re-render
3. `detect_links()` and `search()` run full grid scans on every render frame
4. `terminal.take_dirty()` exists but is never checked — always repaints
5. Per-cell `clone()` in paint (1920+ clones/frame for 80x24)
6. `shape_line(..., None)` instead of `Some(cell_width)` for fixed-width grid

---

## Phase 1: Async Input + Event Batching + Dirty Checking

**Impact:** Fixes typing lag (Bug 1) and reduces render frequency by ~10x (Bug 2 partial).

### Task 1.1: Async Input Channel for tmux-cc

**Files:**
- Modify: `src/runtime/backends/tmux_control_mode.rs`

**Problem:** `TmuxControlModeRuntime::send_input()` calls `send_input_via_send_keys()` → `send_command()` synchronously on the UI thread. Each `send_command()` does `pty_writer.lock()` + `writeln!` + `flush()`.

**Step 1: Add input channel and writer thread to TmuxControlModeRuntime**

In `TmuxControlModeRuntime` struct, add a field:

```rust
/// Input channel for async send_input (like LocalPtyAgent pattern)
input_tx: flume::Sender<(PaneId, Vec<u8>)>,
```

In `TmuxControlModeRuntime::new()`, after creating `pty_writer`, spawn a writer thread:

```rust
let (input_tx, input_rx) = flume::unbounded::<(PaneId, Vec<u8>)>();
let pty_writer_for_thread = rt.pty_writer.clone();
let session_name_for_thread = session_name.to_string();
thread::spawn(move || {
    loop {
        // Block until first input arrives
        let (first_pane, first_bytes) = match input_rx.recv() {
            Ok(v) => v,
            Err(_) => break,
        };
        // Batch: collect all immediately available inputs
        let mut batch: Vec<(PaneId, Vec<u8>)> = vec![(first_pane, first_bytes)];
        while let Ok(item) = input_rx.try_recv() {
            batch.push(item);
        }
        // Process batch: group by pane_id, send via send-keys
        let mut writer = match pty_writer_for_thread.lock() {
            Ok(w) => w,
            Err(_) => break,
        };
        for (pane_id, bytes) in &batch {
            if bytes.is_empty() {
                continue;
            }
            // Inline the send-keys logic (same as send_input_via_send_keys)
            // but write multiple commands before a single flush
            let commands = build_send_keys_commands(pane_id, bytes);
            for cmd in &commands {
                if writeln!(writer, "{}", cmd).is_err() {
                    break;
                }
            }
        }
        let _ = writer.flush();
    }
});
```

**Step 2: Extract `build_send_keys_commands` as a pure function**

Move the body of `send_input_via_send_keys` into a standalone function that returns `Vec<String>` of tmux commands instead of writing them:

```rust
/// Build tmux send-keys commands for the given input bytes.
/// Pure function — no I/O, no locks.
fn build_send_keys_commands(pane_id: &str, bytes: &[u8]) -> Vec<String> {
    let mut commands = Vec::new();
    if bytes.is_empty() {
        return commands;
    }
    let mut i = 0;
    while i < bytes.len() {
        let run_start = i;
        while i < bytes.len() && bytes[i] >= 0x20 && bytes[i] < 0x7f {
            i += 1;
        }
        if run_start < i {
            let text = String::from_utf8_lossy(&bytes[run_start..i]);
            let escaped = text.replace('\'', "'\\''");
            commands.push(format!("send-keys -l -t {} '{}'", pane_id, escaped));
        }
        if i >= bytes.len() { break; }

        if bytes[i] == 0x1b && i + 2 < bytes.len() && bytes[i + 1] == b'[' {
            let key = match bytes[i + 2] {
                b'A' => { i += 3; "Up" }
                b'B' => { i += 3; "Down" }
                b'C' => { i += 3; "Right" }
                b'D' => { i += 3; "Left" }
                b'H' => { i += 3; "Home" }
                b'F' => { i += 3; "End" }
                b'1'..=b'9' if i + 3 < bytes.len() && bytes[i + 3] == b'~' => {
                    let k = match bytes[i + 2] {
                        b'2' => "IC",
                        b'3' => "DC",
                        b'5' => "PPage",
                        b'6' => "NPage",
                        _ => { i += 1; "Escape" }
                    };
                    if k != "Escape" { i += 4; }
                    k
                }
                _ => { i += 1; "Escape" }
            };
            commands.push(format!("send-keys -t {} {}", pane_id, key));
        } else {
            let key = match bytes[i] {
                0x0d | 0x0a => { i += 1; "Enter" }
                0x09 => { i += 1; "Tab" }
                0x08 | 0x7f => { i += 1; "BSpace" }
                0x1b => { i += 1; "Escape" }
                b @ 0x01..=0x1a => {
                    let letter = (b'a' + b - 1) as char;
                    i += 1;
                    commands.push(format!("send-keys -t {} C-{}", pane_id, letter));
                    continue;
                }
                _ => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() && (bytes[i] & 0xC0) == 0x80 { i += 1; }
                    let text = String::from_utf8_lossy(&bytes[start..i]);
                    let escaped = text.replace('\'', "'\\''");
                    commands.push(format!("send-keys -l -t {} '{}'", pane_id, escaped));
                    continue;
                }
            };
            commands.push(format!("send-keys -t {} {}", pane_id, key));
        }
    }
    commands
}
```

**Step 3: Update `send_input` to use the channel**

```rust
fn send_input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
    if bytes.is_empty() {
        return Ok(());
    }
    self.input_tx
        .send((pane_id.clone(), bytes.to_vec()))
        .map_err(|e| RuntimeError::Backend(format!("input channel: {}", e)))
}
```

Keep `send_command` for non-input operations (resize, split, etc.).

**Step 4: Verify**

```bash
RUSTUP_TOOLCHAIN=stable cargo test tmux_control_mode
```

---

### Task 1.2: Batch PTY Output Events Before cx.notify()

**Files:**
- Modify: `src/ui/app_root.rs` (lines ~692-719 and ~798-824)

**Problem:** The PTY output loop calls `entity.update(cx, |_, cx| cx.notify())` on every single chunk. During fast output (agent streaming, `ls -la`), this fires hundreds of redraws/sec.

**Fix:** Drain all pending chunks before notifying, like Okena does.

In both `setup_local_terminal` and `setup_pane_terminal_output`, replace the output processing loop:

```rust
cx.spawn(async move |entity, cx| {
    loop {
        // Wait for first chunk
        let chunk = match rx.recv_async().await {
            Ok(c) => c,
            Err(_) => break,
        };
        // Process first chunk
        terminal_for_output.process_output(&chunk);
        ext.feed(&chunk);

        // Drain all immediately available chunks (batch)
        while let Ok(next_chunk) = rx.try_recv() {
            terminal_for_output.process_output(&next_chunk);
            ext.feed(&next_chunk);
        }

        // Status detection once per batch (not per chunk)
        let shell_info = ShellPhaseInfo {
            phase: ext.shell_phase(),
            last_post_exec_exit_code: None,
        };
        let content_str = ext.take_content().0;
        if let Some(ref pub_) = status_publisher {
            let _ = pub_.check_status(
                &pane_target_clone,
                crate::status_detector::ProcessStatus::Running,
                Some(shell_info),
                &content_str,
            );
        }

        // Single notify for the entire batch
        let _ = entity.update(cx, |_, cx| cx.notify());
        if let Some(ref tae) = term_area_entity {
            let _ = cx.update_entity(tae, |_, cx| cx.notify());
        }
    }
})
.detach();
```

**Key change:** The `while let Ok(next_chunk) = rx.try_recv()` drains all buffered chunks before any UI notification.

**Step: Verify**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 1.3: Only Notify TerminalAreaEntity from PTY Loop

**Files:**
- Modify: `src/ui/app_root.rs` (both output loops)

**Problem:** The PTY loop notifies both AppRoot AND TerminalAreaEntity. AppRoot re-render is expensive (full layout with sidebar, tabbar, etc.). Only TerminalAreaEntity needs to update for terminal content changes.

**Fix:** Remove the AppRoot notify from the PTY output loop. Keep only the TerminalAreaEntity notify:

```rust
// Only notify TerminalAreaEntity — AppRoot doesn't need redraw for terminal content
if let Some(ref tae) = term_area_entity {
    let _ = cx.update_entity(tae, |_, cx| cx.notify());
}
```

AppRoot should only be notified when status actually changes (the StatusPublisher → EventBus path already handles this).

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 1.4: Skip Repaint When Terminal Not Dirty

**Files:**
- Modify: `src/ui/terminal_view.rs` (lines 144-175)

**Problem:** `TerminalView::render()` always constructs and returns a `TerminalElement` even when terminal content hasn't changed.

**Fix:** Check `terminal.take_dirty()` is already available. We should use it in the TerminalElement's paint to early-return. However, since GPUI manages Element lifecycle and we can't skip paint, the better approach is to cache search results and link detection (Task 1.5).

Actually the correct fix here is: don't call `terminal.search()` and `terminal.detect_links()` on every render. Move to Task 1.5.

---

### Task 1.5: Cache Search Results and Link Detection

**Files:**
- Modify: `src/ui/terminal_view.rs`
- Modify: `src/terminal/terminal_core.rs`

**Problem:** `terminal.search(q)` and `terminal.detect_links()` do full grid scans on every render frame, even when nothing changed.

**Fix in terminal_core.rs:** Add cached results with dirty-based invalidation.

```rust
pub struct Terminal {
    // ... existing fields ...
    cached_links: Mutex<Option<Vec<DetectedLink>>>,
    cached_search: Mutex<Option<(String, Vec<SearchMatch>)>>,
}
```

Add methods:

```rust
/// Get links, using cache if content hasn't changed since last call.
pub fn detect_links_cached(&self) -> Vec<DetectedLink> {
    if !self.dirty.load(Ordering::Relaxed) {
        if let Some(cached) = self.cached_links.lock().as_ref() {
            return cached.clone();
        }
    }
    let links = self.detect_links();
    *self.cached_links.lock() = Some(links.clone());
    links
}

/// Get search results, using cache if query and content unchanged.
pub fn search_cached(&self, query: &str) -> Vec<SearchMatch> {
    if !self.dirty.load(Ordering::Relaxed) {
        if let Some((cached_q, cached_r)) = self.cached_search.lock().as_ref() {
            if cached_q == query {
                return cached_r.clone();
            }
        }
    }
    let results = self.search(query);
    *self.cached_search.lock() = Some((query.to_string(), results.clone()));
    results
}
```

Clear caches when dirty is consumed:

```rust
pub fn take_dirty(&self) -> bool {
    let was_dirty = self.dirty.swap(false, Ordering::Relaxed);
    if was_dirty {
        *self.cached_links.lock() = None;
        *self.cached_search.lock() = None;
    }
    was_dirty
}
```

**Fix in terminal_view.rs:** Use cached versions:

```rust
let matches = self
    .search_query
    .as_ref()
    .map(|q| terminal.search_cached(q))
    .unwrap_or_default();
// ...
let links = terminal.detect_links_cached();
```

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo test terminal_core
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 1.6: Run Phase 1 Regression Tests

```bash
cd /Users/matt.chow/workspace/pmux
RUSTUP_TOOLCHAIN=stable cargo build
bash tests/regression/run_all.sh --skip-build
```

**Expected:** All 5 regression tests pass. Manual verification: type in terminal, latency should be noticeably reduced.

---

## Phase 2: GPUI InputHandler + Remaining Optimizations

**Impact:** Text input reaches Zed-level efficiency. Eliminates redundant work in status detection.

### Task 2.1: Implement TerminalInputHandler

**Files:**
- Create: `src/terminal/terminal_input_handler.rs`
- Modify: `src/terminal/mod.rs`
- Modify: `src/terminal/terminal_element.rs`

**Problem:** All text input goes through `on_key_down` → `key_to_bytes` → `send_input`. GPUI's InputHandler path (used by Zed and Okena) is more efficient for text: it handles IME composition, avoids key event overhead, and gets called directly by the platform layer.

**Step 1: Create TerminalInputHandler**

```rust
// src/terminal/terminal_input_handler.rs
//! GPUI InputHandler for terminal text input.
//! Text characters go through this path (efficient, IME-compatible).
//! Special keys (arrows, function keys, etc.) still use key_to_bytes.

use gpui::*;
use std::ops::Range;
use std::sync::Arc;

pub struct TerminalInputHandler {
    send_input: Arc<dyn Fn(&[u8]) + Send + Sync>,
}

impl TerminalInputHandler {
    pub fn new(send_input: Arc<dyn Fn(&[u8]) + Send + Sync>) -> Self {
        Self { send_input }
    }
}

impl InputHandler for TerminalInputHandler {
    fn text_for_range(
        &mut self,
        _range: Range<usize>,
        _adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<SelectedTextRange> {
        Some(SelectedTextRange {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _window: &mut Window, _cx: &mut App) -> Option<Range<usize>> {
        None
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut App) {}

    fn replace_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        _cx: &mut App,
    ) {
        if text.is_empty() {
            return;
        }
        // Filter macOS function key range (U+F700–U+F8FF)
        let filtered: String = text
            .chars()
            .filter(|c| !('\u{F700}'..='\u{F8FF}').contains(c))
            .collect();
        if filtered.is_empty() {
            return;
        }
        // Handle special characters
        let mut bytes = Vec::new();
        for c in filtered.chars() {
            match c {
                '\r' | '\n' => bytes.push(b'\r'),
                '\u{8}' => bytes.push(0x7f), // backspace
                _ => {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    bytes.extend_from_slice(s.as_bytes());
                }
            }
        }
        (self.send_input)(&bytes);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _range: Option<Range<usize>>,
        _new_text: &str,
        _new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut App,
    ) {
        // IME composition — for now, ignore marked text
    }

    fn bounds_for_range(
        &mut self,
        _range: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        None
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<usize> {
        None
    }
}
```

**Step 2: Register InputHandler in TerminalElement::paint()**

In `terminal_element.rs`, add at the end of `paint()`:

```rust
// Register input handler for text input (IME path)
if self.focused {
    let runtime_send = self.on_input.clone();
    if let Some(send_fn) = runtime_send {
        let input_handler = crate::terminal::terminal_input_handler::TerminalInputHandler::new(send_fn);
        window.handle_input(&self.focus_handle, input_handler, cx);
    }
}
```

Add `on_input` field to `TerminalElement`:

```rust
pub struct TerminalElement {
    // ... existing fields ...
    on_input: Option<Arc<dyn Fn(&[u8]) + Send + Sync>>,
}

impl TerminalElement {
    pub fn with_input_handler(mut self, f: Arc<dyn Fn(&[u8]) + Send + Sync>) -> Self {
        self.on_input = Some(f);
        self
    }
}
```

**Step 3: Update key_to_bytes to return None for text chars**

In `src/terminal/input.rs`, the text-producing section (lines 101-112) should return `None` when `key_char` is present and no modifiers are active — so GPUI routes text through InputHandler instead:

```rust
// Text-producing keystrokes: let InputHandler handle them
// Only send via key_to_bytes if Alt is pressed (ESC prefix)
if let Some(ref ch) = keystroke.key_char {
    if !ch.is_empty() {
        if mods.alt {
            let mut bytes = vec![0x1b];
            bytes.extend_from_slice(ch.as_bytes());
            return Some(bytes);
        }
        // Return None — InputHandler will handle regular text
        return None;
    }
}
```

**Step 4: Wire input handler in terminal_view.rs**

In `terminal_view.rs` where `TerminalElement` is constructed, add the input handler:

```rust
// Build send_input closure
let runtime_for_input = /* need to pass runtime + pane_id to TerminalView */;
let pane_for_input = self.pane_id.clone();
let send_fn: Arc<dyn Fn(&[u8]) + Send + Sync> = Arc::new(move |bytes: &[u8]| {
    if let Some(ref rt) = runtime_for_input {
        let _ = rt.send_input(&pane_for_input, bytes);
    }
});
elem = elem.with_input_handler(send_fn);
```

Note: This requires threading the runtime into TerminalView. Add `runtime: Option<Arc<dyn AgentRuntime>>` to `TerminalBuffer::Terminal` variant.

**Step 5: Update mod.rs**

```rust
pub mod terminal_input_handler;
```

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
RUSTUP_TOOLCHAIN=stable cargo test terminal
```

---

### Task 2.2: Throttle Status Detection

**Files:**
- Modify: `src/ui/app_root.rs` (both PTY output loops)

**Problem:** `StatusDetector::detect()` runs 20+ regex matches on every PTY output batch. Most batches during fast typing don't change status.

**Fix:** Only run status detection every N milliseconds or when shell phase changes.

Add a timestamp check before status detection in the output loop:

```rust
use std::time::{Duration, Instant};

// Before the loop:
let mut last_status_check = Instant::now();
let status_check_interval = Duration::from_millis(200);

// Inside the loop, replace the status detection block:
let now = Instant::now();
let phase_changed = ext.shell_phase() != last_phase;
if phase_changed || now.duration_since(last_status_check) >= status_check_interval {
    last_status_check = now;
    last_phase = ext.shell_phase();
    let shell_info = ShellPhaseInfo {
        phase: ext.shell_phase(),
        last_post_exec_exit_code: None,
    };
    let content_str = ext.take_content().0;
    if let Some(ref pub_) = status_publisher {
        let _ = pub_.check_status(
            &pane_target_clone,
            crate::status_detector::ProcessStatus::Running,
            Some(shell_info),
            &content_str,
        );
    }
}
```

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 2.3: Run Phase 2 Regression Tests

```bash
cd /Users/matt.chow/workspace/pmux
RUSTUP_TOOLCHAIN=stable cargo build
bash tests/regression/run_all.sh --skip-build
```

**Expected:** All 5 regression tests pass. Manual test: typing in terminal should feel as responsive as iTerm2. IME input (Chinese, Japanese) should work.

---

## Phase 3: Rendering Pipeline Optimization

**Impact:** Reduces per-frame CPU cost by ~60%, especially for large terminals.

### Task 3.1: Use Fixed-Width shape_line

**Files:**
- Modify: `src/terminal/terminal_rendering.rs` (line 66)

**Problem:** `shape_line(..., None)` lets GPUI auto-calculate glyph positions. For a fixed-width terminal grid, `Some(cell_width)` tells the text system to use monospace alignment, enabling better caching.

**Fix:** Change `BatchedTextRun::paint()`:

```rust
let shaped = window
    .text_system()
    .shape_line(self.text.clone().into(), font_size, &[run_style], Some(cell_width));
```

This is a one-line change. The `cell_width` parameter needs to be passed into `paint()`:

```rust
pub fn paint(
    &self,
    origin: Point<Pixels>,
    cell_width: Pixels,
    line_height: Pixels,
    font_size: Pixels,
    window: &mut Window,
    cx: &mut App,
) {
    // ... existing code ...
    let shaped = window
        .text_system()
        .shape_line(self.text.clone().into(), font_size, &[run_style], Some(cell_width));
    // ...
}
```

`cell_width` is already passed to paint — just change `None` to `Some(cell_width)`.

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 3.2: Avoid Per-Cell Clone in Paint

**Files:**
- Modify: `src/terminal/terminal_element.rs` (line 242)

**Problem:** `let cell = grid[point].clone();` allocates for every cell. The Cell type contains a `char`, `Color` (enum), and `Flags` — all Copy-compatible fields can be read without clone.

**Fix:** Use reference instead of clone:

```rust
let cell = &grid[point];
```

Then replace all `cell.field` accesses — they should work the same since we're reading, not moving. The only issue is if `cell.c`, `cell.fg`, `cell.bg` are not Copy. Check alacritty_terminal's Cell type:
- `c: char` — Copy
- `fg: Color` — implements Copy
- `bg: Color` — implements Copy
- `flags: Flags` — implements Copy

So using a reference is safe. Replace line 242:

```rust
// Before:
let cell = grid[point].clone();

// After:
let cell = &grid[point];
```

No other changes needed — all field accesses work through a reference.

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 3.3: Skip Blank Cells in Paint Loop

**Files:**
- Modify: `src/terminal/terminal_element.rs`

**Problem:** Space characters without decorations still go through the full color resolution and TextRun creation, even though they produce no visible output.

**Fix:** Add early-continue for blank cells (Okena pattern):

```rust
let ch = cell.c;

if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
    continue;
}

// Skip blank cells with no decorations (space/null, default bg, no underline/strikeout)
let has_decorations = cell.flags.contains(Flags::UNDERLINE)
    || cell.flags.contains(Flags::STRIKEOUT);
if (ch == ' ' || ch == '\0') && is_default_bg(&cell.bg) && !has_decorations {
    // Flush current runs
    if let Some(run) = current_run.take() {
        text_runs.push(run);
    }
    if let Some(rect) = current_bg.take() {
        layout_rects.push(rect);
    }
    continue;
}
```

Move the `WIDE_CHAR_SPACER` check before any other processing (it's already first, keep it).

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 3.4: Avoid String Clone in shape_line

**Files:**
- Modify: `src/terminal/terminal_rendering.rs` (line 66)

**Problem:** `self.text.clone().into()` allocates a new SharedString on every paint call.

**Fix:** Use `SharedString::from` with a reference or pre-convert:

```rust
let text: SharedString = self.text.clone().into();
let shaped = window
    .text_system()
    .shape_line(text, font_size, &[run_style], Some(cell_width));
```

This is minor but reduces allocation in the hot loop. If `SharedString` supports `From<&str>`:

```rust
let shaped = window
    .text_system()
    .shape_line(SharedString::from(self.text.as_str()), font_size, &[run_style], Some(cell_width));
```

Check GPUI's SharedString API — if it requires ownership, keep the clone.

---

### Task 3.5: Reduce Font Clone in Paint Loop

**Files:**
- Modify: `src/terminal/terminal_element.rs` (lines 290-295)

**Problem:** `state.font_bold_italic.clone()` etc. clone a Font struct (contains SharedString + enums) for every cell.

**Fix:** Pre-compute font references and use index-based lookup:

```rust
let fonts = [&state.font, &state.font_bold, &state.font_italic, &state.font_bold_italic];
// ...
let font_idx = match (cell.flags.contains(Flags::BOLD), cell.flags.contains(Flags::ITALIC)) {
    (false, false) => 0,
    (true, false) => 1,
    (false, true) => 2,
    (true, true) => 3,
};
let font = fonts[font_idx];
```

Then in the `TextRun` construction, clone only when creating the run (not per cell):

```rust
let text_run = TextRun {
    len: ch.len_utf8(),
    font: font.clone(), // still needs clone for TextRun, but lookup is cheaper
    // ...
};
```

The clone is still needed for TextRun, but the match + clone is marginally cheaper than the original. The bigger win is if we can batch cells with the same font (which `BatchedTextRun::can_append` already does).

---

### Task 3.6: Prepaint: Cache Cell Size Measurement

**Files:**
- Modify: `src/terminal/terminal_element.rs` (lines 126-192)

**Problem:** `prepaint()` calls `shape_line("│", ...)` on every frame to measure cell size. This is an expensive text system call.

**Fix:** Cache cell dimensions and only recompute when font or size changes.

```rust
pub struct TerminalElementState {
    // ... existing fields ...
}

// Add a module-level cache (or per-element cache via global state)
use std::sync::OnceLock;
static CELL_SIZE_CACHE: OnceLock<(Pixels, Pixels)> = OnceLock::new();
```

Actually, since `TerminalElement` is recreated each render, use a thread-local or static cache:

```rust
use std::cell::Cell as StdCell;
thread_local! {
    static CACHED_CELL_SIZE: StdCell<Option<(Pixels, Pixels)>> = const { StdCell::new(None) };
}
```

In `prepaint`, check cache first:

```rust
let (cell_width, line_height) = CACHED_CELL_SIZE.with(|cache| {
    if let Some(cached) = cache.get() {
        return cached;
    }
    // ... existing shape_line measurement ...
    let result = (cell_width, line_height);
    cache.set(Some(result));
    result
});
```

This is safe because font/size don't change at runtime (Menlo 14pt is hardcoded).

**Verify:**

```bash
RUSTUP_TOOLCHAIN=stable cargo check
```

---

### Task 3.7: Run Phase 3 Regression Tests

```bash
cd /Users/matt.chow/workspace/pmux
RUSTUP_TOOLCHAIN=stable cargo build
bash tests/regression/run_all.sh --skip-build
```

**Expected:** All 5 regression tests pass. Visual rendering should be identical. Performance should be noticeably smoother during fast scrolling and large output.

---

## Summary of All Changes

| File | Phase | Change |
|------|-------|--------|
| `src/runtime/backends/tmux_control_mode.rs` | 1 | Async input channel + writer thread |
| `src/ui/app_root.rs` | 1, 2 | Batch PTY events, remove AppRoot notify from PTY loop, throttle status detection |
| `src/terminal/terminal_core.rs` | 1 | Cached search/links with dirty invalidation |
| `src/ui/terminal_view.rs` | 1, 2 | Use cached search/links, wire InputHandler |
| `src/terminal/terminal_input_handler.rs` | 2 | NEW: GPUI InputHandler for text input |
| `src/terminal/input.rs` | 2 | Return None for text chars (let InputHandler handle) |
| `src/terminal/terminal_element.rs` | 2, 3 | Register InputHandler, ref instead of clone, skip blanks, cache cell size |
| `src/terminal/terminal_rendering.rs` | 3 | Fixed-width shape_line |
| `src/terminal/mod.rs` | 2 | Export new module |

## Verification Checklist

After all phases:
- [ ] `cargo test` passes
- [ ] `tests/regression/run_all.sh` passes
- [ ] Typing latency ≤ 16ms (one frame)
- [ ] Fast output (`yes` command) doesn't freeze UI
- [ ] Worktree switching responds within 500ms
- [ ] IME input works (Chinese/Japanese)
- [ ] Search still works
- [ ] Link detection still works
- [ ] Status detection still works (may be delayed by ~200ms)
