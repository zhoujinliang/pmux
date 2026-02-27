## ADDED Requirements

### Requirement: StatusPoller sends status changes to GPUI channel
The StatusPoller SHALL send status change events to a GPUI channel when an agent's status changes.

#### Scenario: Status change triggers channel message
- **WHEN** StatusPoller detects a pane's status has changed
- **THEN** system sends a message containing pane ID and new AgentStatus to the GPUI channel

#### Scenario: No change, no message
- **WHEN** StatusPoller polls a pane and status remains unchanged
- **THEN** system does NOT send a message to the GPUI channel

### Requirement: AppRoot receives status updates via channel
The AppRoot SHALL receive status change messages from the GPUI channel and update its internal state.

#### Scenario: Channel message updates state
- **WHEN** AppRoot receives a status change message on the GPUI channel
- **THEN** AppRoot updates the AgentStatus for the specified pane ID in its status HashMap

#### Scenario: UI redraw triggered
- **WHEN** AppRoot receives and processes a status change message
- **THEN** AppRoot calls cx.notify() to trigger a UI redraw

#### Scenario: Multiple panes update independently
- **WHEN** multiple panes have status changes
- **THEN** each pane's status is updated independently in AppRoot's status HashMap

### Requirement: Sidebar displays real-time status
The Sidebar SHALL display the current status for each entry based on AppRoot's status HashMap.

#### Scenario: Sidebar shows status icon
- **WHEN** Sidebar renders a pane entry
- **THEN** Sidebar displays the status icon (● ◐ ○ ✕ ?) corresponding to the AgentStatus from AppRoot

#### Scenario: Status color matches AgentStatus
- **WHEN** Sidebar renders a status icon
- **THEN** Sidebar uses the RGB color from AgentStatus.rgb_color() for the icon

#### Scenario: Status updates reflect in UI
- **WHEN** AppRoot updates a pane's status in its HashMap
- **THEN** Sidebar displays the new status icon on next render

#### Scenario: Unknown status for untracked panes
- **WHEN** Sidebar renders a pane not tracked in AppRoot's status HashMap
- **THEN** Sidebar displays the Unknown status (purple ?)

### Requirement: TopBar computes live StatusCounts
The TopBar SHALL compute StatusCounts from AppRoot's status HashMap and display aggregate counts.

#### Scenario: StatusCounts computed from HashMap
- **WHEN** TopBar renders StatusCounts
- **THEN** TopBar computes counts by iterating through AppRoot's status HashMap values

#### Scenario: Error count displayed
- **WHEN** multiple panes have Error status
- **THEN** TopBar displays the total count of Error statuses

#### Scenario: Waiting count displayed
- **WHEN** multiple panes have Waiting status
- **THEN** TopBar displays the total count of Waiting statuses

#### Scenario: Empty HashMap shows zero counts
- **WHEN** AppRoot's status HashMap is empty
- **THEN** TopBar displays zero for all status counts

### Requirement: StatusPoller tracks panes by tmux pane ID
The StatusPoller SHALL register and track panes using tmux pane target format (e.g., "sdlc-myproject:control-tower.0").

#### Scenario: Pane registration uses tmux format
- **WHEN** AppRoot starts a tmux session for a workspace
- **THEN** AppRoot registers the pane with StatusPoller using tmux pane target format

#### Scenario: Pane status stored with tmux format ID
- **WHEN** AppRoot receives a status change message
- **THEN** AppRoot stores the status using the tmux pane target ID as the HashMap key

#### Scenario: Pane ID consistent across components
- **WHEN** StatusPoller, AppRoot, Sidebar, and TopBar reference the same pane
- **THEN** all components use the same tmux pane target ID format

### Requirement: Workspace switches reset status tracking
The AppRoot SHALL reset status tracking when switching to a new workspace.

#### Scenario: Workspace switch clears HashMap
- **WHEN** user switches to a different workspace tab
- **THEN** AppRoot clears the status HashMap for the previous workspace

#### Scenario: New workspace starts fresh tracking
- **WHEN** user switches to a new workspace
- **THEN** AppRoot registers the new workspace's panes with StatusPoller

### Requirement: Thread-safe status state sharing
The system SHALL use Arc<Mutex<>> for thread-safe sharing of status state between threads.

#### Scenario: Channel send is lock-free
- **WHEN** StatusPoller sends a status change to the channel
- **THEN** channel send operation is lock-free and non-blocking

#### Scenario: AppRoot state access is serialized
- **WHEN** multiple status change messages arrive
- **THEN** AppRoot's status HashMap access is serialized through Mutex lock

#### Scenario: UI thread serializes notify calls
- **WHEN** multiple cx.notify() calls are triggered
- **THEN** GPUI ensures UI thread serializes render operations