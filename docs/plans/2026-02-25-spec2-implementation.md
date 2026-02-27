# Spec 2 Implementation Plan: Sidebar + Terminal Rendering

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** After selecting a workspace, show a sidebar with worktrees and a terminal that renders tmux pane output with color, with keyboard input passthrough.

**Architecture:** AppRoot gains a `WorkspaceState` holding worktrees + selected pane. A background thread polls `tmux capture-pane -e` every 50ms, feeds output to `alacritty_terminal::Term`, and notifies GPUI to re-render the character grid. Click on terminal area activates input passthrough mode; click on sidebar deactivates it.

**Tech Stack:** Rust, GPUI (zed-industries), alacritty-terminal (crates.io), tmux CLI

---

### Task 1: Add alacritty-terminal dependency

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add dependency**

In `Cargo.toml` under `[dependencies]`, add:
```toml
alacritty-terminal = "0.24"
```

**Step 2: Verify it compiles**

```bash
cargo check
```
Expected: no errors (the crate resolves).

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add alacritty-terminal dependency"
```

---

### Task 2: WorkspaceState — data model

**Files:**
- Create: `src/workspace_state.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing tests**

Create `src/workspace_state.rs`:

```rust
use std::path::PathBuf;
use crate::worktree::WorktreeInfo;

pub struct WorkspaceState {
    pub repo_path: PathBuf,
    pub repo_name: String,
    pub tmux_session: String,
    pub worktrees: Vec<WorktreeInfo>,
    pub selected_index: usize,
    pub pane_ids: Vec<String>,   // parallel to worktrees
    pub input_focused: bool,
}

impl WorkspaceState {
    pub fn new(repo_path: PathBuf) -> Self {
        let repo_name = repo_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let tmux_session = format!("sdlc-{}", repo_name);
        Self {
            repo_path,
            repo_name,
            tmux_session,
            worktrees: Vec::new(),
            selected_index: 0,
            pane_ids: Vec::new(),
            input_focused: false,
        }
    }

    pub fn active_pane_id(&self) -> Option<&str> {
        self.pane_ids.get(self.selected_index).map(|s| s.as_str())
    }

    pub fn select_worktree(&mut self, index: usize) {
        if index < self.worktrees.len() {
            self.selected_index = index;
            self.input_focused = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_state_new() {
        let state = WorkspaceState::new(PathBuf::from("/home/user/myproject"));
        assert_eq!(state.repo_name, "myproject");
        assert_eq!(state.tmux_session, "sdlc-myproject");
        assert_eq!(state.selected_index, 0);
        assert!(!state.input_focused);
    }

    #[test]
    fn test_active_pane_id_empty() {
        let state = WorkspaceState::new(PathBuf::from("/tmp/repo"));
        assert!(state.active_pane_id().is_none());
    }

    #[test]
    fn test_active_pane_id_with_panes() {
        let mut state = WorkspaceState::new(PathBuf::from("/tmp/repo"));
        state.pane_ids = vec!["%0".to_string(), "%1".to_string()];
        assert_eq!(state.active_pane_id(), Some("%0"));
        state.selected_index = 1;
        assert_eq!(state.active_pane_id(), Some("%1"));
    }

    #[test]
    fn test_select_worktree() {
        let mut state = WorkspaceState::new(PathBuf::from("/tmp/repo"));
        state.worktrees = vec![
            WorktreeInfo::new(PathBuf::from("/tmp/repo"), "main", "abc"),
            WorktreeInfo::new(PathBuf::from("/tmp/repo-feat"), "feat-x", "def"),
        ];
        state.input_focused = true;
        state.select_worktree(1);
        assert_eq!(state.selected_index, 1);
        assert!(!state.input_focused); // focus cleared on switch
    }

    #[test]
    fn test_select_worktree_out_of_bounds() {
        let mut state = WorkspaceState::new(PathBuf::from("/tmp/repo"));
        state.select_worktree(99); // should not panic
        assert_eq!(state.selected_index, 0);
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test workspace_state
```
Expected: FAIL — module not found.

**Step 3: Add module to lib.rs**

In `src/lib.rs`, add:
```rust
pub mod workspace_state;
```

**Step 4: Run tests to verify they pass**

```bash
cargo test workspace_state
```
Expected: 5 tests pass.

**Step 5: Commit**

```bash
git add src/workspace_state.rs src/lib.rs
git commit -m "feat: add WorkspaceState data model"
```

---

### Task 3: Input key mapping

**Files:**
- Create: `src/input_handler.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing tests**

Create `src/input_handler.rs`:

```rust
/// Convert a GPUI key name to a tmux send-keys string.
/// Returns None if the key should be handled by pmux (app shortcut).
pub fn key_to_tmux(key: &str, modifiers_cmd: bool) -> Option<String> {
    // pmux shortcuts: Cmd+B, Cmd+N, Cmd+W — intercept
    if modifiers_cmd {
        return None;
    }
    let tmux_key = match key {
        "enter" | "return" => "Enter",
        "backspace" => "BSpace",
        "escape" => "Escape",
        "tab" => "Tab",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "home" => "Home",
        "end" => "End",
        "pageup" => "PPage",
        "pagedown" => "NPage",
        other => return Some(other.to_string()),
    };
    Some(tmux_key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_key() {
        assert_eq!(key_to_tmux("enter", false), Some("Enter".to_string()));
    }

    #[test]
    fn test_backspace_key() {
        assert_eq!(key_to_tmux("backspace", false), Some("BSpace".to_string()));
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(key_to_tmux("up", false), Some("Up".to_string()));
        assert_eq!(key_to_tmux("down", false), Some("Down".to_string()));
        assert_eq!(key_to_tmux("left", false), Some("Left".to_string()));
        assert_eq!(key_to_tmux("right", false), Some("Right".to_string()));
    }

    #[test]
    fn test_escape_tab() {
        assert_eq!(key_to_tmux("escape", false), Some("Escape".to_string()));
        assert_eq!(key_to_tmux("tab", false), Some("Tab".to_string()));
    }

    #[test]
    fn test_cmd_key_intercepted() {
        // Cmd+anything → None (pmux handles it)
        assert_eq!(key_to_tmux("b", true), None);
        assert_eq!(key_to_tmux("n", true), None);
    }

    #[test]
    fn test_regular_char_passthrough() {
        assert_eq!(key_to_tmux("a", false), Some("a".to_string()));
        assert_eq!(key_to_tmux("z", false), Some("z".to_string()));
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test input_handler
```
Expected: FAIL — module not found.

**Step 3: Add module to lib.rs**

```rust
pub mod input_handler;
```

**Step 4: Run tests to verify they pass**

```bash
cargo test input_handler
```
Expected: 6 tests pass.

**Step 5: Commit**

```bash
git add src/input_handler.rs src/lib.rs
git commit -m "feat: add key-to-tmux mapping for input passthrough"
```

---

### Task 4: Terminal content via alacritty_terminal

**Files:**
- Modify: `src/ui/terminal_view.rs`

**Step 1: Write failing tests**

Add to `src/ui/terminal_view.rs`:

```rust
use alacritty_terminal::{
    event::{Event, EventListener},
    event_loop::Notifier,
    grid::Dimensions,
    term::{Config as TermConfig, Term},
    vte::ansi::Processor,
};

/// Minimal event listener (no-op)
struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _event: Event) {}
}

/// Parse ANSI-escaped terminal output into a grid of cells.
/// Returns Vec of rows, each row is Vec of (char, fg_rgb, bg_rgb).
pub fn parse_ansi_to_grid(
    ansi_bytes: &[u8],
    cols: usize,
    rows: usize,
) -> Vec<Vec<(char, [u8; 3], [u8; 3])>> {
    use alacritty_terminal::grid::Dimensions;

    struct Size { cols: usize, rows: usize }
    impl Dimensions for Size {
        fn columns(&self) -> usize { self.cols }
        fn screen_lines(&self) -> usize { self.rows }
    }

    let size = Size { cols, rows };
    let config = TermConfig::default();
    let mut term = Term::new(config, &size, NoopListener);
    let mut processor = Processor::new();

    for &byte in ansi_bytes {
        processor.advance(&mut term, byte);
    }

    let mut grid = Vec::new();
    for row_idx in 0..rows {
        let mut row = Vec::new();
        for col_idx in 0..cols {
            let cell = &term.grid()[alacritty_terminal::index::Point::new(
                alacritty_terminal::index::Line(row_idx as i32),
                alacritty_terminal::index::Column(col_idx),
            )];
            let ch = cell.c;
            // Default colors as fallback
            let fg = [204u8, 204, 204];
            let bg = [30u8, 30, 30];
            row.push((ch, fg, bg));
        }
        grid.push(row);
    }
    grid
}

#[cfg(test)]
mod terminal_parse_tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let input = b"hello";
        let grid = parse_ansi_to_grid(input, 80, 24);
        assert_eq!(grid.len(), 24);
        assert_eq!(grid[0][0].0, 'h');
        assert_eq!(grid[0][1].0, 'e');
        assert_eq!(grid[0][2].0, 'l');
    }

    #[test]
    fn test_parse_empty() {
        let grid = parse_ansi_to_grid(b"", 80, 24);
        assert_eq!(grid.len(), 24);
        // Empty cells should be space
        assert_eq!(grid[0][0].0, ' ');
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test terminal_parse_tests
```
Expected: FAIL — alacritty_terminal API not found (need to check exact API).

**Step 3: Adjust API to match alacritty-terminal 0.24**

The exact API may differ. Check with:
```bash
cargo doc --open -p alacritty-terminal
```

Adjust `parse_ansi_to_grid` to use the correct types. The key pattern is:
1. Create a `Term<NoopListener>` with a size implementing `Dimensions`
2. Feed bytes through `Processor::advance`
3. Read cells from `term.grid()`

**Step 4: Run tests to verify they pass**

```bash
cargo test terminal_parse_tests
```
Expected: 2 tests pass.

**Step 5: Commit**

```bash
git add src/ui/terminal_view.rs
git commit -m "feat: add ANSI parsing via alacritty_terminal"
```

---

### Task 5: Terminal poller (background thread)

**Files:**
- Create: `src/terminal_poller.rs`
- Modify: `src/lib.rs`

**Step 1: Write failing tests**

Create `src/terminal_poller.rs`:

```rust
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Snapshot of terminal content for a pane
#[derive(Clone, Default)]
pub struct PaneSnapshot {
    pub pane_id: String,
    pub content: String,
    pub content_hash: u64,
}

impl PaneSnapshot {
    pub fn new(pane_id: &str, content: String) -> Self {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        let hash = hasher.finish();
        Self {
            pane_id: pane_id.to_string(),
            content,
            content_hash: hash,
        }
    }

    pub fn has_changed(&self, other: &PaneSnapshot) -> bool {
        self.content_hash != other.content_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_hash_same_content() {
        let a = PaneSnapshot::new("%0", "hello world".to_string());
        let b = PaneSnapshot::new("%0", "hello world".to_string());
        assert!(!a.has_changed(&b));
    }

    #[test]
    fn test_snapshot_hash_different_content() {
        let a = PaneSnapshot::new("%0", "hello".to_string());
        let b = PaneSnapshot::new("%0", "world".to_string());
        assert!(a.has_changed(&b));
    }

    #[test]
    fn test_snapshot_empty() {
        let a = PaneSnapshot::new("%0", "".to_string());
        let b = PaneSnapshot::new("%0", "".to_string());
        assert!(!a.has_changed(&b));
    }
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test terminal_poller
```
Expected: FAIL — module not found.

**Step 3: Add module to lib.rs**

```rust
pub mod terminal_poller;
```

**Step 4: Run tests to verify they pass**

```bash
cargo test terminal_poller
```
Expected: 3 tests pass.

**Step 5: Commit**

```bash
git add src/terminal_poller.rs src/lib.rs
git commit -m "feat: add terminal poller with hash-based change detection"
```

---

### Task 6: Wire AppRoot — load worktrees on workspace open

**Files:**
- Modify: `src/ui/app_root.rs`

**Step 1: Add WorkspaceState to AppRoot**

In `src/ui/app_root.rs`, add import and field:

```rust
use crate::workspace_state::WorkspaceState;
use crate::worktree::discover_worktrees;
use crate::tmux::Session;

pub struct AppRoot {
    state: AppState,
    workspace_manager: WorkspaceManager,
    workspace_state: Option<WorkspaceState>,  // NEW
}
```

Update `AppRoot::new()` to initialize `workspace_state: None`.

**Step 2: Add `load_workspace` method**

```rust
impl AppRoot {
    fn load_workspace(&mut self, path: PathBuf) {
        let mut ws = WorkspaceState::new(path.clone());

        // Ensure tmux session exists
        let session = Session::new(&ws.repo_name);
        if let Err(e) = session.ensure() {
            eprintln!("Warning: tmux session error: {}", e);
        }

        // Discover worktrees
        match discover_worktrees(&path) {
            Ok(worktrees) => {
                ws.worktrees = worktrees;
            }
            Err(e) => {
                eprintln!("Warning: worktree discovery failed: {}", e);
                // Fall back to single entry for the repo itself
            }
        }

        self.workspace_state = Some(ws);
    }
}
```

Call `self.load_workspace(path.clone())` inside `handle_select_workspace` after the workspace is validated and added.

**Step 3: Write tests**

Add to the `#[cfg(test)]` block in `app_root.rs`:

```rust
#[test]
fn test_workspace_state_none_initially() {
    let app = create_test_app_root();
    assert!(app.workspace_state.is_none());
}
```

**Step 4: Run all tests**

```bash
cargo test
```
Expected: all existing tests + new test pass.

**Step 5: Commit**

```bash
git add src/ui/app_root.rs
git commit -m "feat: load WorkspaceState when workspace is opened"
```

---

### Task 7: Sidebar GPUI render

**Files:**
- Modify: `src/ui/sidebar.rs`

**Step 1: Add GPUI Render impl**

The existing `sidebar.rs` has data structs but no GPUI rendering. Add a proper GPUI component:

```rust
use gpui::*;
use gpui::prelude::*;
use crate::worktree::WorktreeInfo;

pub struct Sidebar {
    pub repo_name: String,
    pub worktrees: Vec<WorktreeInfo>,
    pub selected_index: usize,
}

impl Sidebar {
    pub fn new(repo_name: String, worktrees: Vec<WorktreeInfo>) -> Self {
        Self { repo_name, worktrees, selected_index: 0 }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(250.))
            .h_full()
            .bg(rgb(0x1a1a1a))
            .border_r_1()
            .border_color(rgb(0x333333))
            .flex()
            .flex_col()
            // Repo name header
            .child(
                div()
                    .px(px(12.))
                    .py(px(10.))
                    .text_color(rgb(0xaaaaaa))
                    .text_size(px(12.))
                    .child(SharedString::from(format!("📁 {}", self.repo_name)))
            )
            // Worktree list
            .children(
                self.worktrees.iter().enumerate().map(|(i, wt)| {
                    let is_selected = i == self.selected_index;
                    let branch = wt.short_branch_name().to_string();
                    let icon = if is_selected { "●" } else { "○" };
                    let bg = if is_selected { rgb(0x2a2a2a) } else { rgb(0x1a1a1a) };

                    div()
                        .id(ElementId::Integer(i))
                        .px(px(12.))
                        .py(px(8.))
                        .bg(bg)
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _ev, _win, cx| {
                            this.selected_index = i;
                            cx.notify();
                        }))
                        .flex()
                        .gap(px(6.))
                        .child(
                            div().text_color(rgb(0x44cc44)).child(icon)
                        )
                        .child(
                            div()
                                .text_color(rgb(0xcccccc))
                                .text_size(px(13.))
                                .child(SharedString::from(branch))
                        )
                })
            )
    }
}
```

**Step 2: Run all tests**

```bash
cargo test
```
Expected: all tests pass (GPUI render is not unit-tested, just needs to compile).

**Step 3: Commit**

```bash
git add src/ui/sidebar.rs
git commit -m "feat: implement Sidebar GPUI render with worktree list"
```

---

### Task 8: TerminalView GPUI render

**Files:**
- Modify: `src/ui/terminal_view.rs`

**Step 1: Add GPUI Render impl**

Add a `TerminalView` GPUI entity that holds the parsed grid and renders it:

```rust
use gpui::*;
use gpui::prelude::*;

pub struct TerminalView {
    pub pane_id: Option<String>,
    pub grid: Vec<Vec<(char, [u8; 3], [u8; 3])>>,
    pub input_focused: bool,
    pub cols: usize,
    pub rows: usize,
}

impl TerminalView {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            pane_id: None,
            grid: vec![vec![(' ', [30, 30, 30], [30, 30, 30]); cols]; rows],
            input_focused: false,
            cols,
            rows,
        }
    }

    pub fn update_from_ansi(&mut self, ansi_bytes: &[u8]) {
        self.grid = crate::ui::terminal_view::parse_ansi_to_grid(ansi_bytes, self.cols, self.rows);
    }
}

impl Render for TerminalView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = if self.input_focused {
            rgb(0x0066cc)  // blue border when focused
        } else {
            rgb(0x333333)
        };

        div()
            .flex_1()
            .h_full()
            .bg(rgb(0x1e1e1e))
            .border_1()
            .border_color(border_color)
            .font_family("Menlo")
            .text_size(px(13.))
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _ev, _win, cx| {
                this.input_focused = true;
                cx.notify();
            }))
            .children(
                self.grid.iter().map(|row| {
                    div()
                        .flex()
                        .children(row.iter().map(|(ch, _fg, _bg)| {
                            div()
                                .text_color(rgb(0xcccccc))
                                .child(ch.to_string())
                        }))
                })
            )
    }
}
```

**Step 2: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

**Step 3: Commit**

```bash
git add src/ui/terminal_view.rs
git commit -m "feat: implement TerminalView GPUI render with focus mode"
```

---

### Task 9: Wire AppRoot render — show Sidebar + TerminalView

**Files:**
- Modify: `src/ui/app_root.rs`

**Step 1: Update render_workspace_view**

Replace the placeholder `render_workspace_view` with the real layout:

```rust
fn render_workspace_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_row()
        .child(/* Sidebar entity */)
        .child(/* TerminalView entity */)
}
```

Since `Sidebar` and `TerminalView` are now GPUI entities, `AppRoot` needs to hold `Entity<Sidebar>` and `Entity<TerminalView>` as fields, created in `new()` or `load_workspace()`.

Add to `AppRoot`:
```rust
sidebar: Option<Entity<Sidebar>>,
terminal_view: Option<Entity<TerminalView>>,
```

In `load_workspace`, after discovering worktrees:
```rust
let sidebar = cx.new(|_| Sidebar::new(ws.repo_name.clone(), ws.worktrees.clone()));
let terminal_view = cx.new(|_| TerminalView::new(220, 50));
self.sidebar = Some(sidebar);
self.terminal_view = Some(terminal_view);
```

Note: `load_workspace` needs `cx: &mut Context<Self>` parameter for this.

**Step 2: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

**Step 3: Commit**

```bash
git add src/ui/app_root.rs
git commit -m "feat: wire Sidebar and TerminalView into AppRoot layout"
```

---

### Task 10: Background polling thread

**Files:**
- Modify: `src/ui/app_root.rs`

**Step 1: Start polling in load_workspace**

After creating entities, start a background task that polls tmux every 50ms:

```rust
// In load_workspace, after creating terminal_view entity:
if let Some(pane_id) = ws.active_pane_id().map(|s| s.to_string()) {
    let terminal_view = self.terminal_view.clone().unwrap();
    cx.spawn(async move |_entity, cx| {
        loop {
            cx.background_executor().timer(Duration::from_millis(50)).await;

            // Capture pane output
            if let Ok(content) = crate::tmux::capture_pane(&pane_id) {
                terminal_view.update(cx, |tv, cx| {
                    tv.update_from_ansi(content.as_bytes());
                    cx.notify();
                }).ok();
            }
        }
    }).detach();
}
```

**Step 2: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

**Step 3: Manual test**

```bash
cargo run
```
- Select a git repo as workspace
- Sidebar should show worktrees
- Terminal area should show tmux pane output
- Click terminal → type → output appears in tmux

**Step 4: Commit**

```bash
git add src/ui/app_root.rs
git commit -m "feat: start background polling thread for terminal updates"
```

---

### Task 11: Keyboard input passthrough

**Files:**
- Modify: `src/ui/app_root.rs`

**Step 1: Handle KeyDown in AppRoot render**

In the workspace view render, add key handler when `input_focused`:

```rust
div()
    .size_full()
    .flex()
    .when(input_focused, |el| {
        el.on_key_down(cx.listener(|this, event: &KeyDownEvent, _win, cx| {
            let key_str = format!("{:?}", event.keystroke.key).to_lowercase();
            let is_cmd = event.keystroke.modifiers.command;

            if let Some(tmux_key) = crate::input_handler::key_to_tmux(&key_str, is_cmd) {
                if let Some(pane_id) = this.workspace_state
                    .as_ref()
                    .and_then(|ws| ws.active_pane_id())
                {
                    let _ = crate::tmux::send_keys(pane_id, &tmux_key);
                }
            }
        }))
    })
    .child(sidebar)
    .child(terminal_view)
```

**Step 2: Run all tests**

```bash
cargo test
```
Expected: all tests pass.

**Step 3: Manual test**

```bash
cargo run
```
- Click terminal area → type `ls` + Enter → output appears
- Click sidebar → typing stops going to terminal

**Step 4: Commit**

```bash
git add src/ui/app_root.rs
git commit -m "feat: keyboard input passthrough to tmux pane"
```

---

### Task 12: Final cleanup and CLAUDE.md

**Files:**
- Create: `CLAUDE.md`

**Step 1: Run full test suite**

```bash
cargo test
```
Expected: all tests pass (target: 30+).

**Step 2: Fix any compiler warnings**

```bash
cargo clippy
```
Fix any warnings flagged.

**Step 3: Create CLAUDE.md**

```bash
# See separate CLAUDE.md creation task
```

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: Spec 2 complete — Sidebar + terminal rendering + input passthrough"
```

---

## Acceptance Criteria

- [ ] Selecting a workspace creates/attaches tmux session `sdlc-{repo}`
- [ ] Sidebar shows all worktrees with branch names
- [ ] Terminal area renders tmux pane output with ANSI colors
- [ ] Clicking terminal area activates input passthrough
- [ ] Typing in focused terminal sends keys to tmux pane
- [ ] Clicking sidebar deactivates input passthrough
- [ ] All tests pass (`cargo test`)
