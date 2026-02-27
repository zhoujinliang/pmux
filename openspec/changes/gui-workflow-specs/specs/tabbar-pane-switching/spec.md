## ADDED Requirements

### Requirement: TabBar displays pane tabs for current workspace
The system SHALL display a tab bar showing all panes in the current workspace.

#### Scenario: Single pane view
- **GIVEN** workspace has only main branch pane
- **WHEN** the workspace view loads
- **THEN** the tab bar shows one tab labeled "🖥 main"
- **AND** the tab is marked as active

#### Scenario: Multiple panes
- **GIVEN** workspace has multiple panes (main, feat-x, fix-bug)
- **WHEN** the tab bar renders
- **THEN** each pane has a corresponding tab
- **AND** tabs show 🖥 icon and branch name
- **AND** modified panes show ● indicator

### Requirement: User can switch between panes via tabs
The system SHALL allow users to click tabs to switch between panes.

#### Scenario: Clicking a tab
- **WHEN** user clicks on a tab
- **THEN** that tab becomes active (highlighted)
- **AND** the terminal view switches to that pane's content
- **AND** focus moves to that pane

#### Scenario: Keyboard shortcut switching
- **WHEN** user presses ⌘1
- **THEN** the first tab becomes active
- **AND** corresponding pane is shown

- **WHEN** user presses ⌘3
- **THEN** the third tab becomes active

### Requirement: User can close panes via tab close button
The system SHALL allow users to close panes by clicking the × button on tabs.

#### Scenario: Clicking close button
- **WHEN** user clicks the × on a tab
- **THEN** a confirmation dialog appears
- **AND** warns about killing the agent process

#### Scenario: Confirming pane closure
- **GIVEN** confirmation dialog is shown
- **WHEN** user confirms closing
- **THEN** the tmux pane is killed
- **AND** the tab is removed
- **AND** if it was the active tab, focus shifts to adjacent tab

### Requirement: TabBar provides New Tab button
The system SHALL provide a button to create new panes.

#### Scenario: Clicking New Tab button
- **WHEN** user clicks the + button on tab bar
- **THEN** a new pane is created in tmux
- **AND** a new tab appears
- **AND** focus moves to the new pane
