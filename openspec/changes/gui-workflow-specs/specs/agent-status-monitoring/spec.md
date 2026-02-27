## ADDED Requirements

### Requirement: System polls pane output for status detection
The system SHALL periodically capture pane output to detect agent status.

#### Scenario: Status polling cycle
- **GIVEN** status poller is running
- **WHEN** 500ms interval elapses
- **THEN** system executes `tmux capture-pane` for each registered pane
- **AND** analyzes output using detect_status()

### Requirement: Status detection categorizes pane states
The system SHALL categorize each pane into one of five states based on output analysis.

#### Scenario: Detecting Running state
- **GIVEN** pane output contains "thinking" or "executing tool"
- **WHEN** status detection runs
- **THEN** state is set to Running
- **AND** sidebar shows green ● icon

#### Scenario: Detecting Waiting state
- **GIVEN** pane output contains "> " or "? " prompt
- **WHEN** status detection runs
- **THEN** state is set to Waiting
- **AND** sidebar shows yellow ◐ icon

#### Scenario: Detecting Error state
- **GIVEN** pane output contains "error" or "failed"
- **WHEN** status detection runs
- **THEN** state is set to Error
- **AND** sidebar shows red ✕ icon
- **AND** notification may be triggered

#### Scenario: Detecting Idle state
- **GIVEN** pane has content but no activity indicators
- **WHEN** status detection runs
- **THEN** state is set to Idle
- **AND** sidebar shows gray ○ icon

#### Scenario: Detecting Unknown state
- **GIVEN** pane has no content or cannot be captured
- **WHEN** status detection runs
- **THEN** state is set to Unknown
- **AND** sidebar shows purple ? icon

### Requirement: Sidebar displays status for each worktree
The system SHALL display the current status icon and text for each worktree in sidebar.

#### Scenario: Status display format
- **GIVEN** worktree "feat-x" is in Running state with 2 commits ahead
- **WHEN** sidebar renders
- **THEN** entry shows "● feat-x"
- **AND** second line shows "Running · +2"

### Requirement: TopBar displays overall status summary
The system SHALL display aggregate status counts in the TopBar.

#### Scenario: Status summary with errors
- **GIVEN** 2 panes in Error state, 1 in Waiting
- **WHEN** TopBar renders
- **THEN** notification bell shows badge with "3"
- **AND** bell icon is highlighted in red

#### Scenario: Status summary all good
- **GIVEN** all panes in Running or Idle state
- **WHEN** TopBar renders
- **THEN** notification bell shows no badge
- **AND** displays "3 running" or similar summary

### Requirement: State changes trigger UI updates
The system SHALL update the UI when pane status changes.

#### Scenario: Status transition
- **GIVEN** pane was in Running state
- **WHEN** agent finishes and shows prompt
- **THEN** status changes to Waiting
- **AND** sidebar icon updates from ● to ◐
- **AND** notification is triggered

### Requirement: Status detection uses debouncing
The system SHALL use debouncing to prevent rapid status flickering.

#### Scenario: Rapid output changes
- **GIVEN** pane output changes rapidly between states
- **WHEN** status detection runs
- **THEN** state only updates after consistent reading for 1 second
- **AND** except for Error state which updates immediately
