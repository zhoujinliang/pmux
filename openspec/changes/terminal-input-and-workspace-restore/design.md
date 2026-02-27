## Context

The pmux application currently displays terminal output from tmux sessions in a TerminalView component, but lacks the ability to forward keyboard input back to tmux. The `input_handler.rs` module provides a `key_to_tmux` function that converts GPUI key names to tmux-compatible strings, but there is no integration between GPUI keyboard events and tmux send-keys command execution.

Additionally, the application can save workspace paths to config but does not restore them on startup. The `App::new()` constructor loads the saved workspace from config, but the main application flow (presumably in `AppRoot`) does not call `start_tmux_session` when a saved workspace is present—only when the user manually selects a workspace.

**Current State:**
- Terminal output: Displayed via `TerminalView` with content updated from tmux `capture-pane` polling (every 200ms)
- Keyboard input: `key_to_tmux()` function exists but unused
- Workspace restoration: Config loads saved path, but `start_tmux_session` only called on manual selection

**Constraints:**
- Must use GPUI's keyboard event API for capturing user input
- Must use tmux `send-keys` command for forwarding input to the active pane
- Must maintain existing app shortcuts (Cmd+B, Cmd+N, Cmd+W)
- Must not break the 200ms polling loop for output updates

## Goals / Non-Goals

**Goals:**
1. Capture keyboard events from GPUI and forward them to the active tmux pane
2. Restore saved workspaces automatically on application startup
3. Preserve existing app-level keyboard shortcuts (Cmd+B, Cmd+N, Cmd+W)
4. Maintain the existing 200ms output polling mechanism

**Non-Goals:**
- Mouse input handling to tmux
- Custom terminal emulation (delegate to tmux)
- Multi-pane input routing (only send to currently active pane)
- Workspace state persistence beyond the path (e.g., pane history)

## Decisions

### 1. Input Handler Integration Architecture

**Decision:** Create an `InputHandler` struct in `input_handler.rs` that wraps the tmux send-keys logic and integrates with GPUI's event system.

**Rationale:**
- Separates concerns: key conversion logic stays in `input_handler.rs`, event routing handled by AppRoot/TerminalView
- Makes testing easier by isolating tmux interaction
- Allows future extension for multi-pane input routing

**Alternatives Considered:**
- Inline tmux send-keys in AppRoot: Would couple app logic with tmux details, harder to test
- Pass keyboard events directly from AppRoot to tmux: Violates single responsibility, AppRoot should coordinate, not execute

### 2. Keyboard Event Flow

**Decision:** Route keyboard events through AppRoot → InputHandler → tmux send-keys. TerminalView only handles rendering, not input.

**Rationale:**
- AppRoot is the natural coordinator for both TerminalView and tmux session management
- Centralizes input handling, making it easier to intercept app shortcuts before forwarding to tmux
- Consistent with existing architecture where AppRoot manages startup logic (`start_tmux_session`)

**Alternatives Considered:**
- TerminalView handles its own keyboard events: Would duplicate shortcut handling logic, create synchronization issues
- InputHandler directly subscribes to GPUI events: Breaks encapsulation, InputHandler shouldn't know about GPUI

### 3. Workspace Restoration Timing

**Decision:** Check for saved workspace in `App::new()` (already implemented), then call `start_tmux_session` in AppRoot's initialization if a workspace exists.

**Rationale:**
- Leverages existing config loading logic
- Restores workspace early in startup sequence
- Allows user to see output as soon as window opens

**Alternatives Considered:**
- Restore on first render: Delays session start, poor UX
- Show "Restoring workspace" loading state: Adds complexity without significant benefit

### 4. Tmux Target Specification

**Decision:** Use the active tmux pane as the target for send-keys. When creating new panes, track the pane ID in WorkspaceManager.

**Rationale:**
- Matches user expectations (typing goes to currently visible terminal)
- Minimal complexity for initial implementation
- Existing tmux session already tracks active pane

**Alternatives Considered:**
- Always send to the first pane: Would be confusing if user has multiple panes
- Track pane per TerminalView instance: Over-engineering for current single-pane use case

### 5. Error Handling for Send-Keys

**Decision:** Log errors but do not interrupt application flow. Failed send-keys should be handled gracefully.

**Rationale:**
- Network/process failures shouldn't crash the UI
- Terminal output will eventually reflect the failed input (user can retry)
- Non-blocking error handling maintains UI responsiveness

**Alternatives Considered:**
- Panic on send-keys failure: Too aggressive for transient failures
- Show error dialog to user: Excessive UI noise for common transient issues

## Risks / Trade-offs

**Risk:** Keyboard events might lag if tmux process is slow
**Mitigation:** Run send-keys as a fire-and-forget operation, don't block UI thread. Consider async execution in future if lag becomes noticeable.

**Risk:** Workspace restoration might fail if git repository no longer exists
**Mitigation:** Validate workspace path on startup (already done in `is_git_repository`). Fall back to welcome screen if validation fails.

**Trade-off:** Current design assumes single-pane usage. Multi-pane input routing will require adding pane tracking to WorkspaceManager.
**Acceptance:** Acceptable for initial implementation. The architecture can be extended without breaking changes.

**Risk:** Keyboard shortcuts might conflict with tmux bindings
**Mitigation:** Only intercept Cmd+key combinations (which are not tmux bindings). Regular key combinations are always forwarded to tmux.

## Migration Plan

1. Implement `InputHandler` struct with `send_key` method that executes tmux send-keys
2. Add keyboard event handling in AppRoot's `render` or event handler
3. Check for Cmd+key shortcuts before forwarding to InputHandler
4. Add startup logic in AppRoot to call `start_tmux_session` if workspace exists
5. Test workspace restoration by saving a workspace and restarting application

**Rollback Strategy:** If issues arise, keyboard handling can be disabled by removing the event subscription from AppRoot. Workspace restoration can be disabled by commenting out the startup check.

## Open Questions

- **What is the target tmux pane ID for send-keys?** Need to investigate if tmux provides a way to send keys to the active pane without specifying an ID, or if we need to track pane IDs when creating sessions.
- **Should we implement a rate limit for send-keys?** Rapid keyboard typing could flood tmux with commands. Current design assumes tmux handles this gracefully, but this may need testing.