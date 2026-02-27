## ADDED Requirements

### Requirement: Sidebar worktree selection activates corresponding TabBar tab

The system SHALL activate the TabBar tab that matches the selected worktree in the Sidebar when the user clicks on a worktree entry in the Sidebar.

#### Scenario: User clicks Sidebar worktree entry
- **WHEN** user clicks on a worktree entry in the Sidebar
- **THEN** the TabBar tab corresponding to that worktree SHALL become active
- **AND** the TabBar active tab SHALL be visually highlighted
- **AND** the selected Sidebar worktree SHALL remain selected

#### Scenario: User clicks Sidebar worktree when TabBar has matching tab
- **WHEN** user clicks on a worktree that has an open tab in the TabBar
- **THEN** the matching TabBar tab SHALL become active
- **AND** the previous TabBar active tab SHALL become inactive

### Requirement: TabBar tab activation updates Sidebar selection

The system SHALL select the corresponding worktree entry in the Sidebar when the user activates a TabBar tab.

#### Scenario: User clicks TabBar tab
- **WHEN** user clicks on a TabBar tab for a worktree
- **THEN** the Sidebar worktree entry matching that worktree SHALL become selected
- **AND** the Sidebar selected worktree SHALL be visually highlighted
- **AND** the TabBar tab SHALL remain active

#### Scenario: User closes TabBar tab clears Sidebar selection
- **WHEN** user closes a TabBar tab that corresponds to the currently selected Sidebar worktree
- **THEN** the Sidebar selection SHALL be cleared
- **AND** no Sidebar worktree SHALL be selected

### Requirement: Worktree selection state consistency

The system SHALL maintain a single source of truth for the active worktree state across both Sidebar and TabBar components.

#### Scenario: Single worktree active state
- **WHEN** a worktree is selected in either Sidebar or TabBar
- **THEN** only one worktree SHALL be active across both components
- **AND** the active worktree SHALL be consistently displayed in both Sidebar and TabBar

#### Scenario: No worktree selected state
- **WHEN** no worktree is currently selected
- **THEN** no Sidebar worktree SHALL be selected
- **AND** no TabBar tab SHALL be active