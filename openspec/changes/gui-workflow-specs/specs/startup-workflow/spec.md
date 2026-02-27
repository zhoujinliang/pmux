## ADDED Requirements

### Requirement: Application startup flow
The system SHALL display appropriate initial screen based on saved workspace state.

#### Scenario: First launch with no saved workspace
- **WHEN** the application starts
- **AND** there is no recent_workspace in config
- **THEN** the system displays the centered "Welcome to pmux" startup page
- **AND** shows the "Open Workspace" CTA button

#### Scenario: Launch with saved workspace
- **WHEN** the application starts
- **AND** there is a valid recent_workspace in config
- **THEN** the system automatically loads that workspace
- **AND** transitions to the workspace view

### Requirement: Workspace selection
The system SHALL allow users to select a git repository as workspace.

#### Scenario: Click Open Workspace button
- **WHEN** user clicks the "Open Workspace" button on startup page
- **THEN** the system opens the system folder picker dialog
- **AND** allows user to select a directory

#### Scenario: Select valid git repository
- **WHEN** user selects a valid git repository
- **THEN** the system saves it as recent_workspace
- **AND** creates/attaches tmux session
- **AND** transitions to workspace view

#### Scenario: Select invalid directory
- **WHEN** user selects a non-git directory
- **THEN** the system displays error message "Not a git repository"
- **AND** remains on startup page

### Requirement: Recent workspaces list
The system SHALL display recently opened workspaces for quick access.

#### Scenario: Show recent workspaces
- **WHEN** there are saved recent workspaces
- **THEN** the startup page displays them as a list
- **AND** each item shows repo name and path

#### Scenario: Click recent workspace
- **WHEN** user clicks a recent workspace item
- **THEN** the system loads that workspace directly
- **AND** skips the folder picker

### Requirement: Keyboard shortcuts
The system SHALL support keyboard shortcuts for workspace operations.

#### Scenario: Press ⌘N on startup page
- **WHEN** user presses ⌘N
- **THEN** the system opens the folder picker

#### Scenario: Press ⌘N in workspace view
- **WHEN** user presses ⌘N in workspace view
- **THEN** the system opens additional workspace in new tab
