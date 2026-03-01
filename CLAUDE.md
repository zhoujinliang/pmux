# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

pmux is an AI Agent multi-branch development workbench - a native desktop GUI application for managing multiple AI agents working in parallel (one per git worktree), with real-time agent status monitoring, notifications, and quick diff review.

## Build and Test Commands

```bash
# Build and run the application (use stable toolchain if RUSTUP_TOOLCHAIN=esp is set)
RUSTUP_TOOLCHAIN=stable cargo run
# or: unset RUSTUP_TOOLCHAIN && cargo run

# Run all tests (SIGBUS in gpui_macros can occur on some macOS setups; use stable)
RUSTUP_TOOLCHAIN=stable cargo test

# Run a specific test by name
cargo test test_workspace_tab_creation

# Run tests in a specific module
cargo test workspace_manager::

# Build release version
cargo build --release

# Check code without building
cargo check
```

**Note:** If `RUSTUP_TOOLCHAIN=esp` is set (e.g. for ESP-IDF development), pmux must use `stable` to avoid proc-macro SIGBUS. The project's `rust-toolchain.toml` specifies `stable`; override the env when needed.

## Architecture

### Tech Stack
- **GPUI** - Zed editor's UI framework (native GPU-accelerated UI)
- **serde** - JSON serialization for config
- **rfd** - Cross-platform file dialogs
- **gpui-terminal** - Embedded terminal (VTE via alacritty_terminal internally)
- **thiserror** - Error handling

### Key Modules

**UI Layer (`src/ui/`)**
- `app_root.rs` - Root GPUI component, manages application state and view switching
- `sidebar.rs` - Left sidebar showing worktree list
- `tabbar.rs` - Tab bar for multi-workspace navigation
- `terminal_view.rs` - Terminal wrapper: GpuiTerminal (embedded gpui_terminal::TerminalView), Error, or Empty placeholder
- `terminal_area_entity.rs` - Terminal area with split panes; notifies on content changes
- `new_branch_dialog_ui.rs` - Modal dialog for branch creation

**Terminal Pipeline (`src/terminal/`)**
- `stream_adapter.rs` - RuntimeReader, RuntimeWriter, tee_output for gpui-terminal Read/Write
- `content_extractor.rs` - OSC 133 + visible text extraction for StatusPublisher

**State Management**
- `workspace_manager.rs` - Multi-workspace tab management (add/switch/close tabs)
- `app_state.rs` - Complete application state for persistence
- `config.rs` - Configuration persistence (~/.config/pmux/config.json)

**Runtime Backends (`src/runtime/backends/`)**
- `mod.rs` - Backend factory: resolve_backend(), create_runtime_from_env(), recover_runtime(); default = "local"
- `local_pty.rs` - LocalPtyAgent (default backend): direct PTY spawn, multi-pane, diff/review
- `tmux_control_mode.rs` - tmux -CC control mode: ControlModeParser (%output/%begin/%end/%exit), TmuxControlModeRuntime (persistence backend)
- `tmux.rs` - Legacy TmuxRuntime (pipe-pane + capture-pane), deprecated

**Tmux Integration (`src/tmux/`)**
- `session.rs` - Tmux session lifecycle management
- `pane.rs` - Pane operations (capture, send-keys)
- `window.rs` - Window management

**Git/Worktree**
- `worktree.rs` - Git worktree discovery and parsing
- `git_utils.rs` - Repository validation
- `worktree_manager.rs` - Worktree creation and management

**Shell Integration (`src/`)**
- `shell_integration.rs` - OSC 133 parser and shell state (MarkerKind, ShellPhase, ShellMarker, Osc133Parser)
- Flow: shell emits OSC 133 (A/B/C/D) → ContentExtractor.feed() parses via Osc133Parser → ShellPhaseInfo → StatusDetector

**Agent Status Detection (`src/`)**
- `agent_status.rs` - Agent status enumeration (Running, Waiting, Idle, Error, Unknown) with display properties
- `status_detector.rs` - Analyzes pane content to determine agent status; uses ShellPhaseInfo when OSC 133 available, falls back to text patterns
- `StatusPublisher` - Publishes agent state changes via Event Bus (reads from stream/Term buffers; no capture-pane)

**Input Handling**
- `app_root.rs` - handle_key_down forwards keys to runtime.send_input (or gpui_terminal when focused)
- Keyboard shortcuts intercepted at AppRoot (Cmd+B toggles sidebar)

## Development Patterns

### Test Organization
- Unit tests are inline in source files under `#[cfg(test)]` modules
- Integration tests are in `tests/` directory
- Tests follow TDD pattern with Arrange-Act-Assert structure
- Use `tempfile::TempDir` for filesystem test isolation
- See `test-driven-development` skill for full TDD workflow

### Spec-Driven Development
The project uses openspec for feature specifications:
- `openspec/changes/` - Active feature specifications
- `openspec/archive/` - Completed specs
- Each spec contains: proposal.md, design.md, specs/, tasks.md

### Subagent-Driven Development
When implementing plans from `docs/plans/` (Runtime Phase 1–4), use the `subagent-driven-development` skill to delegate tasks to subagents (explore/shell/generalPurpose) for parallel execution. See `.cursor/skills/subagent-driven-development/SKILL.md`.

### Writing Plans
When creating implementation plans from specs or approved designs, use the `writing-plans` skill. Plans go to `docs/plans/YYYY-MM-DD-<feature>.md` with bite-sized tasks, exact file paths, and complete code samples. See `.cursor/skills/writing-plans/SKILL.md`.

### Code Review
When the user requests a code review, use the `requesting-code-review` skill. Review against design.md, success criteria, and pmux conventions. See `.cursor/skills/requesting-code-review/SKILL.md`.

### UI Component Pattern
GPUI components implement the `Render` trait:
```rust
impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Return div() chain with styling and event handlers
    }
}
```

### Error Handling
- Custom error types using `thiserror` derive macro
- Git operations return `GitError` enum
- Tmux operations return `SessionError`, `PaneError`, `WindowError`
- Error messages should be user-friendly (some in Chinese for the target audience)

### Shell Integration Flow
1. Shell (zsh/bash/fish) emits OSC 133 sequences (A=PromptStart, B=PromptEnd, C=PreExec, D=PostExec)
2. ContentExtractor.feed() parses bytes via Osc133Parser, yields ShellPhaseInfo
3. StatusPublisher.check_status() uses ShellPhaseInfo when available (Running→Running, PostExec+exit≠0→Error)
4. Fallback: when OSC 133 unavailable, text-based detection (patterns like "thinking", "?") still works

See `docs/shell-integration.md` for user shell configuration.

### State Flow
1. AppRoot loads Config on startup
2. Valid workspace triggers runtime creation (local PTY or tmux via `create_runtime_from_env`)
3. Terminal pipeline: subscribe_output → tee_output → (RuntimeReader → gpui_terminal, ContentExtractor → StatusPublisher)
4. When agent status changes:
   - StatusPublisher publishes to EventBus
   - AppRoot's event subscription updates pane_statuses, StatusCounts
   - cx.notify() triggers UI redraw
   - Sidebar displays updated status icons
   - TopBar displays updated aggregate counts
5. Keyboard events: gpui_terminal (when focused) or AppRoot handle_key_down → runtime.send_input
6. Workspace switching stops current runtime and starts new one
7. All state changes trigger cx.notify() for UI redraw

## Important Notes

- The application requires tmux to be installed for terminal functionality
- Configuration is stored at ~/.config/pmux/config.json
- GPUI is pinned to a specific git commit (rev = "269b03f4") in Cargo.toml
- The UI uses a dark theme with rgb(0x1e1e1e) as the background color
- Git repository validation supports normal repos, bare repos, and worktrees
- Agent status detection uses event-driven StatusPublisher (triggered by ContentExtractor on terminal output) with debouncing
- Shell integration (OSC 133) improves status accuracy when enabled in user's shell; see docs/shell-integration.md
- Status updates propagate from StatusPublisher → EventBus → AppRoot → Sidebar/TopBar via cx.notify()
