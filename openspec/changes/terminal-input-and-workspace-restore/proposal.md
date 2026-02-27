## Why

The pmux terminal application currently displays terminal output from tmux sessions but lacks interactive keyboard input handling. Users cannot type commands or interact with terminals rendered in TerminalView. Additionally, the application does not restore saved workspaces on startup, requiring users to manually select their workspace every time.

This change addresses two critical gaps:
1. Enable keyboard input passthrough so users can interact with terminals
2. Implement workspace restoration from saved config on application startup

## What Changes

- Implement keyboard event handling in `input_handler.rs` to capture GPUI keyboard events and forward them to tmux via `tmux send-keys`
- Integrate input_handler with AppRoot and TerminalView to create the input event flow
- Modify startup logic in AppRoot to check for saved workspace in config and automatically restore it if present
- Ensure `start_tmux_session` is called on both manual workspace selection and config-based restoration

## Capabilities

### New Capabilities

- `terminal-input-handling`: Capture GPUI keyboard events and forward them to tmux sessions for user interaction
- `workspace-restoration`: Automatically restore saved workspaces from config on application startup

### Modified Capabilities

None

## Impact

**Affected Code:**
- `input_handler.rs`: Implement keyboard event capture and tmux send-keys forwarding
- `AppRoot`: Integrate input handler into render pipeline and add startup workspace restoration logic
- `TerminalView`: Connect to input handler for keyboard event reception
- `Config`: Support reading saved workspace path for restoration

**Dependencies:**
- GPUI keyboard event API
- tmux `send-keys` command execution
- Existing config persistence layer

**Systems:**
- Terminal interaction flow
- Application startup sequence