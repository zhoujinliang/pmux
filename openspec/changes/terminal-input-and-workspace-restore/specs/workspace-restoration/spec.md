## ADDED Requirements

### Requirement: Automatic Workspace Restoration on Startup
The system SHALL automatically restore the last saved workspace from config when the application starts, if a valid workspace path exists in config.

#### Scenario: Saved Workspace Restored
- **WHEN** application starts with a valid workspace path saved in config
- **THEN** system automatically loads the workspace
- **AND** system calls start_tmux_session for the workspace
- **AND** terminal output is displayed

#### Scenario: No Saved Workspace
- **WHEN** application starts with no workspace path in config
- **THEN** system displays the welcome screen
- **AND** system does NOT call start_tmux_session

#### Scenario: Invalid Workspace Path
- **WHEN** application starts with a workspace path that no longer exists
- **THEN** system validates the path
- **AND** system falls back to the welcome screen
- **AND** system does NOT call start_tmux_session

### Requirement: Workspace Path Validation
The system SHALL validate that the restored workspace path is a valid git repository before starting the tmux session.

#### Scenario: Valid Git Repository
- **WHEN** restored workspace path is a valid git repository
- **THEN** system proceeds with tmux session startup

#### Scenario: Non-Git Directory
- **WHEN** restored workspace path exists but is not a git repository
- **THEN** system displays error message
- **AND** system falls back to welcome screen

#### Scenario: Missing Directory
- **WHEN** restored workspace path does not exist
- **THEN** system clears the invalid path from config
- **AND** system falls back to welcome screen

### Requirement: Workspace Restoration Timing
The system SHALL restore the workspace early in the startup sequence to minimize delay before the user sees terminal output.

#### Scenario: Early Session Start
- **WHEN** application starts with a saved workspace
- **THEN** system starts tmux session during initial render
- **AND** terminal content is polled immediately after

#### Scenario: User Can Still Select Workspace
- **WHEN** application starts with a saved workspace but user wants a different workspace
- **THEN** system provides option to change workspace
- **AND** user can manually select a new workspace