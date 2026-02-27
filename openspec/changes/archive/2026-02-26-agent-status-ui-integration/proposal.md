## Why

The backend agent status detection system (AgentStatus, StatusDetector, StatusPoller, PaneStatusTracker) is complete and can detect agent states in real-time. However, the UI layer only displays static status information and lacks real-time updates. Without connecting the StatusPoller to the GPUI event system, users cannot see live agent state changes as they happen, reducing the effectiveness of the monitoring capability.

## What Changes

- **Connect StatusPoller to GPUI event system**: Create a GPUI channel that forwards status changes from the background polling thread to the UI thread
- **Integrate with AppRoot**: Add status change handling in AppRoot to receive channel messages and trigger UI updates via `cx.notify()`
- **Update Sidebar with real-time status**: Ensure Sidebar entries reflect current agent status from the channel
- **Update TopBar with live counts**: Ensure StatusCounts in TopBar update dynamically based on current status changes
- **Thread-safe state sharing**: Use Arc<Mutex<>> for state sharing between StatusPoller and UI components

## Capabilities

### New Capabilities
- `agent-status-realtime-updates`: Real-time status change notifications from background poller to GPUI UI layer via channel-based communication

### Modified Capabilities
- None (no existing specs in this project yet)

## Impact

- **Code**: `src/app_root.rs` - Add channel receiver and status change handler
- **Code**: `src/ui/sidebar.rs` - Update to receive real-time status from AppRoot
- **Code**: `src/ui/topbar.rs` - Update StatusCounts to reflect live data
- **Code**: `src/agent/status_poller.rs` - May need modifications to send channel messages on status changes
- **Concurrency**: Introduces cross-thread communication via GPUI channels
- **Performance**: Background polling (500ms) with UI update notifications should not impact responsiveness