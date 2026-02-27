## Context

The backend agent status detection system is complete with the following components:
- `AgentStatus` enum defines status states (Running, Waiting, Idle, Error, Unknown) with display properties
- `StatusDetector` analyzes pane content to determine status
- `StatusPoller` runs background polling thread (500ms interval) tracking all panes
- `PaneStatusTracker` maintains status history per pane

The UI layer has static status display in Sidebar and TopBar but lacks real-time updates. The current AppRoot already uses `cx.spawn` for terminal content polling with `Arc<Mutex<TerminalContent>>` for shared state and `cx.notify()` to trigger UI redraws.

## Goals / Non-Goals

**Goals:**
- Connect StatusPoller status changes to GPUI UI thread via channel-based communication
- Enable real-time status updates in Sidebar entries as agents change state
- Enable real-time StatusCounts updates in TopBar
- Maintain thread safety between background polling thread and UI thread
- Follow existing AppRoot patterns (cx.spawn, Arc<Mutex<>>)

**Non-Goals:**
- Modifying StatusPoller core polling logic (already works)
- Changing status detection algorithms (already implemented)
- Adding new status types (already defined)
- Notification system integration (separate feature)

## Decisions

### 1. Use GPUI channels for cross-thread communication

**Decision:** Use GPUI's channel API (Sender/Receiver) to send status updates from StatusPoller thread to AppRoot UI thread.

**Rationale:**
- GPUI channels are designed for this exact use case
- AppRoot already uses `cx.spawn` pattern for terminal content polling
- Follows GPUI best practices for thread-safe UI updates
- Cleaner than modifying StatusPoller callback system

**Alternatives considered:**
- **StatusPoller callback system**: Already exists but requires registering callbacks before thread starts. Less flexible for dynamic registration.
- **Shared Arc<Mutex<>> status map**: Would require polling from UI thread or complex notification. Channel is push-based, more efficient.

### 2. Integrate StatusPoller into AppRoot as a managed component

**Decision:** Create and manage StatusPoller instance within AppRoot, starting it when workspace loads.

**Rationale:**
- AppRoot already manages workspace lifecycle (tmux sessions, terminal polling)
- Single source of truth for which panes to track
- Easier to coordinate with cx.notify() for UI updates
- Follows existing pattern: `AppRoot.start_tmux_session()` creates session, starts terminal polling, now will also start status polling

### 3. Store real-time status in AppRoot state

**Decision:** Add `HashMap<String, AgentStatus>` to AppRoot to track current status per pane ID.

**Rationale:**
- Sidebar needs per-pane status for each entry
- TopBar needs aggregate counts (computed from HashMap)
- Centralized location simplifies state management
- Updated via channel receiver in AppRoot's event loop

### 4. Pane ID mapping: Use tmux pane target format

**Decision:** Use tmux pane target format (e.g., `sdlc-myproject:control-tower.0`) as pane IDs for status tracking.

**Rationale:**
- AppRoot already tracks `active_pane_target` in this format
- StatusPoller uses same format for `register_pane()`
- Consistent across the codebase
- No need for additional mapping layer

### 5. Trigger UI updates on status changes

**Decision:** Call `cx.notify()` when receiving status changes on the channel to trigger UI redraw.

**Rationale:**
- Follows existing pattern used for terminal content updates
- Minimal overhead (only redraws on actual changes)
- Simple and reliable

## Risks / Trade-offs

### Risk: Channel overflow from frequent status changes

**Risk:** With 500ms polling interval and multiple panes, could flood channel if agent status oscillates rapidly.

**Mitigation:** StatusPoller already has debounce threshold (default: 2). Channel send is non-blocking, old messages are dropped if UI thread is slow. Monitor channel size in testing.

### Risk: Thread safety with shared state

**Risk:** Multiple threads accessing shared status state could cause race conditions.

**Mitigation:** Use Arc<Mutex<>> for all shared state. GPUI ensures UI thread serializes access to cx.notify(). Channel provides lock-free message passing.

### Trade-off: Polling frequency vs CPU usage

**Trade-off:** 500ms polling balances responsiveness vs CPU overhead.

**Consideration:** If performance issues arise, can adjust via PollerConfig. Current default is reasonable for desktop app.

### Risk: Pane ID desync

**Risk:** Pane IDs might change (session restarts, window reorganizations), causing stale status data.

**Mitigation:** When switching workspace or session restarts, clear status HashMap and re-register panes. Workspace manager handles tab switching.

## Migration Plan

1. Add channel types and status HashMap to AppRoot struct
2. Create GPUI channel in AppRoot::new()
3. Modify StatusPoller to accept channel sender (or use callback wrapper that sends to channel)
4. Start StatusPoller in start_tmux_session() alongside terminal polling
5. Add channel receiver handler to AppRoot (subscribe or poll in event loop)
6. Update Sidebar to use real-time status from AppRoot state
7. Update TopBar to compute StatusCounts from AppRoot state
8. Test: Create workspace, observe status updates in real-time
9. Test: Switch workspaces, verify status state resets correctly

## Open Questions

- **Q:** How does GPUI channel API work exactly? Need to verify channel creation and receiver subscription syntax.
- **Q:** Should we use `cx.subscribe()` for channel receiver or manual polling? Depends on GPUI channel semantics.

*These will be resolved during implementation by checking GPUI documentation and existing patterns in the codebase.*