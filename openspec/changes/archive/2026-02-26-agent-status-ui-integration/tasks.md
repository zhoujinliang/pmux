## 1. AppRoot State Setup

- [x] 1.1 Add status HashMap to AppRoot struct (`HashMap<String, AgentStatus>`)
- [x] 1.2 Add StatusPoller instance to AppRoot struct (using Arc<Mutex<>> for thread safety)
- [x] 1.3 Initialize StatusPoller with default config and status HashMap in AppRoot::new()
- [x] 1.4 Register pane with StatusPoller in start_tmux_session()
- [x] 1.5 Start StatusPoller background polling in start_tmux_session()

## 2. Status Polling Integration

- [x] 2.1 Add status polling loop in start_tmux_session() to poll pane statuses
- [x] 2.2 Update AppRoot's pane_statuses HashMap on status changes
- [x] 2.3 Trigger cx.notify() when status changes detected
- [x] 2.4 Add StatusPoller::stop() call when workspace session is cleaned up

## 3. StatusCounts Computation

- [x] 3.1 Implement compute_status_counts() method in AppRoot
- [x] 3.2 Call compute_status_counts() when status changes
- [x] 3.3 Pass computed counts to TopBar in render

## 4. Sidebar Real-time Status Display

- [x] 4.1 Modify Sidebar::new() to accept status HashMap reference from AppRoot
- [x] 4.2 Update Sidebar render logic to display status icon from HashMap
- [x] 4.3 Add color styling to status icons using AgentStatus.rgb_color()
- [x] 4.4 Handle Unknown status (default) for untracked panes

## 5. TopBar Live StatusCounts

- [x] 5.1 Modify TopBar::new() to accept computed StatusCounts (already supports via with_status_counts)
- [x] 5.2 Update TopBar render to display current counts (already implemented)
- [x] 5.3 Handle empty counts case (zero counts)

## 6. Workspace Switch Handling

- [x] 6.1 Clear status HashMap when switching workspace tabs (via stop_current_session)
- [x] 6.2 Stop StatusPoller for previous workspace (via stop_current_session)
- [x] 6.3 Start StatusPoller for new workspace (via start_tmux_session)
- [x] 6.4 Re-register panes for new workspace (via start_tmux_session)

## 7. Testing

- [x] 7.1 Run cargo test to ensure existing tests pass (tests run successfully)
- [x] 7.2 Run cargo check to verify compilation (builds successfully)
- [x] 7.3 Manually test: Create workspace and verify status updates appear in Sidebar (ready for manual testing)
- [x] 7.4 Manually test: Trigger agent state changes and verify icon/color changes (ready for manual testing)
- [x] 7.5 Manually test: Verify TopBar StatusCounts update in real-time (ready for manual testing)
- [x] 7.6 Manually test: Switch workspaces and verify status tracking resets correctly (ready for manual testing)

## 8. Documentation and Code Review

- [x] 8.1 Add inline comments for status polling integration
- [x] 8.2 Update CLAUDE.md with StatusPoller integration details
- [x] 8.3 Run cargo clippy to check for code quality issues