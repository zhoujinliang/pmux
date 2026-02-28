# Terminal Engine Phase 2: Frame Loop & Rendering

## Objective

Implement the frame loop (60fps) that batches PTY processing and triggers GPUI redraws. Migrate UI rendering to use `renderable_content()` instead of `visible_lines()`.

## Success Criteria

- [ ] Frame tick runs at ~60fps (16ms interval)
- [ ] All pending PTY bytes processed in frame tick (not on receive)
- [ ] UI renders via `renderable_content()` not `visible_lines()`
- [ ] Cursor drawn from renderable_content cursor info
- [ ] No more "every byte triggers repaint"

## Architecture

```
GPUI Frame Loop (16ms)
         │
         ▼
┌─────────────────────┐
│ AppRoot::frame_tick │
│  ├─ For each pane:  │
│  │   engine.advance_bytes() │
│  │   (drain all pending)   │
│  └─ cx.notify() → redraw    │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│  TerminalView::render    │
│  ├─ engine.renderable_content() │
│  ├─ iterate display_iter()      │
│  └─ draw_cursor(content.cursor) │
└─────────────────────┘
```

## Tasks

### T1. Add Frame Tick to AppRoot

**File:** `src/ui/app_root.rs`

Add a frame tick mechanism using GPUI's animation/interval system:

```rust
impl AppRoot {
    /// Called every frame (60fps) to process PTY data and trigger redraw
    fn frame_tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;

        // Process all pane engines
        for (_, pane_state) in &mut self.pane_states {
            if let Some(engine) = &pane_state.engine {
                // Drain all pending bytes
                engine.advance_bytes();
                changed = true;
            }
        }

        // Notify GPUI to redraw if any data was processed
        if changed {
            cx.notify();
        }

        // Schedule next frame
        cx.spawn(|this, mut cx| async move {
            cx.background_executor().timer(Duration::from_millis(16)).await;
            this.update(|root, window, cx| {
                root.frame_tick(window, cx);
            }).ok();
        }).detach();
    }
}
```

**Alternative:** Use GPUI's existing animation loop if available:
```rust
// In render() or window context:
window.on_animation_frame(|window, cx| {
    self.frame_tick(window, cx);
});
```

**Acceptance:**
- Frame tick runs approximately every 16ms
- Logs show batch processing (many bytes per frame, not 1:1)

### T2. Update TerminalView to Use renderable_content

**File:** `src/ui/terminal_view.rs`

Replace the current `LineContent` rendering with `RenderableContent`:

```rust
impl RenderOnce for TerminalView {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // Get renderable content from TermBridge
        let content = match &self.buffer {
            TerminalBuffer::Error(msg) => {
                // Render error state
                return self.render_error(msg);
            }
            TerminalBuffer::Term(term) => {
                term.lock().unwrap().renderable_content()
            }
        };

        // Use display_iter from renderable_content
        let mut line_elements: Vec<AnyElement> = Vec::new();
        let mut current_line: Vec<AnyElement> = Vec::new();
        let mut current_row: usize = 0;

        for cell in content.display_iter() {
            let row = cell.point.line.0 as usize;

            // Start new line when row changes
            if row != current_row {
                if !current_line.is_empty() {
                    line_elements.push(self.render_line(current_line, current_row, &content.cursor));
                }
                current_line = Vec::new();
                current_row = row;
            }

            // Create cell element
            let cell_el = self.render_cell(&cell);
            current_line.push(cell_el);
        }

        // Don't forget last line
        if !current_line.is_empty() {
            line_elements.push(self.render_line(current_line, current_row, &content.cursor));
        }

        // Render cursor if applicable
        if self.should_show_cursor() {
            line_elements = self.overlay_cursor(line_elements, &content.cursor);
        }

        div()
            .id("terminal-view")
            .size_full()
            .children(line_elements)
    }
}
```

**Helper methods:**
```rust
impl TerminalView {
    fn render_cell(&self, cell: &RenderableCell) -> AnyElement {
        let fg = rgb_u8(cell.fg.r, cell.fg.g, cell.fg.b);
        let bg = rgb_u8(cell.bg.r, cell.bg.g, cell.bg.b);

        div()
            .text_color(fg)
            .bg(bg)
            .font_family("Menlo")
            .text_size(px(12.))
            .child(SharedString::from(cell.c.to_string()))
            .into_any_element()
    }

    fn render_line(
        &self,
        cells: Vec<AnyElement>,
        row: usize,
        cursor: &CursorPosition,
    ) -> AnyElement {
        div()
            .h(px(LINE_HEIGHT))
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .children(cells)
            .into_any_element()
    }
}
```

**Acceptance:**
- Uses `display_iter()` from renderable_content
- No calls to `visible_lines()` or `visible_lines_with_colors()`
- ANSI colors preserved

### T3. Fix Cursor Rendering

**File:** `src/ui/terminal_view.rs`

Use cursor position from `RenderableContent`:

```rust
impl TerminalView {
    /// Draw cursor at the correct position from renderable_content
    fn overlay_cursor(
        &self,
        lines: Vec<AnyElement>,
        cursor: &CursorPosition,
    ) -> Vec<AnyElement> {
        let cursor_row = cursor.point.line.0 as usize;
        let cursor_col = cursor.point.column.0;

        // Create cursor element
        let cursor_el = div()
            .h(px(LINE_HEIGHT))
            .w(px(8.)) // Character width
            .bg(rgb(0x74ade8))
            .into_any_element();

        // Insert cursor into correct line
        // ... modify lines vector
        lines
    }
}
```

**Acceptance:**
- Cursor position matches alacritty_terminal's cursor
- Cursor hidden when TUI active (alternate screen)
- Cursor follows focus state

### T4. Remove Polling-Based Content Updates

**File:** `src/ui/app_root.rs`

Find and remove any timer-based content polling:

```rust
// REMOVE: Any code like this:
// cx.spawn(|this, mut cx| async move {
//     loop {
//         timer(Duration::from_millis(200)).await;
//         // poll content
//     }
// }).detach();
```

The frame tick now handles all updates:
- PTY bytes → channel → frame tick → advance_bytes() → render

**Acceptance:**
- No `interval_ms` or `sleep(200)` in UI code
- Only frame tick drives updates

### T5. Update PaneState to Hold Engine

**File:** `src/ui/app_root.rs` (or wherever PaneState is defined)

```rust
struct PaneState {
    pane_id: String,
    // OLD: term_bridge: Option<Arc<Mutex<TermBridge>>>,
    // NEW:
    engine: Option<Arc<TerminalEngine>>,
    view: TerminalView,
}
```

Update all references to use `engine` instead of direct TermBridge access.

**Acceptance:**
- All pane states use TerminalEngine
- Compiles without errors

## Verification

### Performance Test
```bash
# Run pmux with Claude Code
# Check CPU usage - should be much lower than before
# Activity Monitor: pmux CPU should be < 10% during normal use
```

### Visual Test
- [ ] vim opens with correct cursor position
- [ ] Claude Code selection/highlighting works
- [ ] Scrollback preserved
- [ ] Colors render correctly

### Code Review
```bash
# Ensure no visible_lines() usage in UI
grep -r "visible_lines" src/ui/
# Should return nothing (or only in deprecated code)

# Ensure frame-based rendering
grep -r "frame_tick\|advance_bytes" src/
# Should find the new code
```

## Migration Notes

1. `TermBridge::visible_lines()` → `TermBridge::renderable_content().display_iter()`
2. `TermBridge::visible_lines_with_colors()` → `TermBridge::renderable_content().display_iter()`
3. Cursor from manual calculation → `renderable_content().cursor`

## Common Pitfalls

1. **Don't** call `advance_bytes()` on every channel receive
2. **Don't** use `visible_lines()` - it misses scrollback
3. **Don't** calculate cursor position manually - use renderable_content
4. **Do** batch all PTY processing in frame tick
5. **Do** use `display_iter()` for complete terminal state

## Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| CPU during TUI | High | Low |
| Updates/sec | 1000s | 60 |
| Cursor accuracy | Offset | Exact |
| Scrollback | Broken | Working |
