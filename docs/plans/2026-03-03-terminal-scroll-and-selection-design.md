# Terminal Scroll & Selection Design

Date: 2026-03-03

## Goal

Add scrollback navigation and text selection to both terminal backends (tmux control mode and local PTY). Also forward mouse events to terminal programs that request mouse input.

## Approach

Approach A: register mouse event handlers directly inside `TerminalElement::paint()` via `window.on_mouse_event()`. All scroll/selection state lives in alacritty_terminal's `Term` (already behind `Arc<Mutex<Term>>`). No backend changes needed — both tmux-cc and local PTY feed output into the same VTE grid.

## Features

- **Scroll**: mouse wheel/trackpad + Shift+PageUp/Down/Home/End
- **Selection**: click+drag (Simple), double-click (word/Semantic), triple-click (line/Lines), auto-copy to clipboard
- **Mouse reporting**: when a program enables mouse mode (vim, less, htop), forward mouse events as SGR escape sequences instead of handling them as scroll/selection

## Design

### 1. Terminal Core API (`terminal_core.rs`)

New methods on `Terminal`:

```rust
pub fn scroll_display(&self, lines: i32)       // positive = into history
pub fn scroll_to_bottom(&self)
pub fn display_offset(&self) -> usize

pub fn start_selection(&self, point: AlacPoint, ty: SelectionType)
pub fn update_selection(&self, point: AlacPoint)
pub fn clear_selection(&self)
pub fn selection_text(&self) -> Option<String>  // Term::selection_to_string()
```

SelectionType maps to alacritty_terminal's types: Simple (click-drag), Semantic (double-click word), Lines (triple-click line).

Auto-scroll behavior:
- New output (`process_output`): if user hasn't scrolled, stay at bottom; if scrolled up, stay put.
- User input (`send_input` path): auto-scroll to bottom.

### 2. Mouse Event Handling (`terminal_element.rs`)

Four handlers registered in `paint()` via `window.on_mouse_event()`:

| Event | Mouse mode OFF | Mouse mode ON |
|-------|---------------|---------------|
| ScrollWheel | `terminal.scroll_display(delta)` | SGR scroll escape → `on_input` |
| MouseDown(Left) | 1-click→Simple, 2→Semantic, 3→Lines | SGR press → `on_input` |
| MouseMove (drag) | `terminal.update_selection(point)` | SGR motion → `on_input` |
| MouseUp(Left) | `selection_text()` → clipboard | SGR release → `on_input` |

Coordinate translation helper:

```rust
fn pixel_to_grid(mouse: Point<Pixels>, origin: Point<Pixels>,
                  cell_w: Pixels, line_h: Pixels,
                  display_offset: i32, cols: usize, rows: usize) -> AlacPoint
```

Mouse mode detection: `TermMode` flags `MOUSE_REPORT_CLICK | MOUSE_DRAG | MOUSE_MOTION`. When any is set, events go to the program via SGR encoding.

SGR encoding (mode 1006):
- Press: `\x1b[<btn;col;rowM`
- Release: `\x1b[<btn;col;rowm`
- Scroll up: button 64, scroll down: button 65

Selection rendering: semi-transparent blue overlay on selected cells, painted between background rects and text (same pattern as search match highlighting).

### 3. Mouse Escape Sequence Encoding (`terminal/input.rs`)

New module-level functions for SGR mouse encoding:

```rust
pub fn mouse_button_press(button: u8, col: usize, row: usize) -> Vec<u8>
pub fn mouse_button_release(button: u8, col: usize, row: usize) -> Vec<u8>
pub fn mouse_motion(button: u8, col: usize, row: usize) -> Vec<u8>
pub fn mouse_scroll(up: bool, col: usize, row: usize) -> Vec<u8>
```

### 4. Keyboard Scroll Shortcuts (`app_root.rs`)

| Shortcut | Action |
|----------|--------|
| Shift+PageUp | Scroll up one page (rows - 2) |
| Shift+PageDown | Scroll down one page |
| Shift+Home | Scroll to top of history |
| Shift+End | Scroll to bottom |

Intercepted in `handle_key_down` before forwarding to runtime, like Cmd+B.

### 5. Clipboard Integration

On mouse-up with active selection: `cx.write_to_clipboard(ClipboardItem::new(text))`. Auto-copy, no Cmd+C needed.

### 6. Auto-scroll on Input

When the user types (any bytes sent through `runtime.send_input`), call `terminal.scroll_to_bottom()` to return to live output.

## Files Changed

1. `src/terminal/terminal_core.rs` — scroll/selection methods
2. `src/terminal/terminal_element.rs` — mouse handlers, selection painting, coord translation
3. `src/terminal/input.rs` — SGR mouse escape encoding
4. `src/ui/app_root.rs` — keyboard scroll shortcuts, scroll-on-input
5. `src/terminal/mod.rs` — re-export new types if needed

## Not In Scope

- Visible scroll bar (not requested)
- Shift+click to extend selection (future)
- Keyboard selection with Shift+Arrow (future)
- Tmux scrollback sync (client-side scrollback from VTE grid is sufficient)
