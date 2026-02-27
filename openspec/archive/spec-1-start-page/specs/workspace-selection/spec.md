## ADDED Requirements

### Requirement: Startup Page Display

When pmux launches without a saved workspace, it shall display a startup page that guides the user to select a Git repository.

#### Scenario: First Launch

- **GIVEN** pmux is launched for the first time (no config file exists)
- **WHEN** the application initializes
- **THEN** a centered startup page is displayed
- **AND** the page contains a welcome message "Welcome to pmux"
- **AND** the page contains descriptive text "Select a Git repository to manage your AI agents"
- **AND** the page contains a primary CTA button labeled "📁 Select workspace"

#### Scenario: No Saved Workspace

- **GIVEN** pmux has been launched before but no workspace was saved
- **WHEN** the application initializes
- **THEN** the startup page is displayed (same as first launch)

#### Scenario: Has Saved Workspace

- **GIVEN** pmux has a valid saved workspace in config
- **WHEN** the application initializes
- **THEN** the startup page is NOT displayed
- **AND** the application proceeds to load the saved workspace

### Requirement: Workspace Selection

The user shall be able to select a directory through a system file picker dialog.

#### Scenario: Open File Picker

- **GIVEN** the startup page is displayed
- **WHEN** the user clicks the "Select workspace" button
- **THEN** a system-native folder picker dialog opens
- **AND** the dialog allows selecting only directories (not files)

#### Scenario: Cancel Selection

- **GIVEN** the file picker dialog is open
- **WHEN** the user cancels the dialog (presses Escape or clicks Cancel)
- **THEN** the dialog closes
- **AND** the startup page remains visible
- **AND** no error is shown

#### Scenario: Valid Git Repository Selected

- **GIVEN** the file picker dialog is open
- **WHEN** the user selects a directory containing a `.git` subdirectory
- **AND** the user confirms the selection
- **THEN** the selected path is validated as a Git repository
- **AND** the path is saved to the configuration file
- **AND** the application transitions to the workspace view

#### Scenario: Invalid Directory Selected

- **GIVEN** the file picker dialog is open
- **WHEN** the user selects a directory without a `.git` subdirectory
- **AND** the user confirms the selection
- **THEN** an error message is displayed: "The selected directory is not a Git repository. Please select a directory containing a .git folder."
- **AND** the startup page remains visible
- **AND** the user can try again

### Requirement: Configuration Persistence

The selected workspace path shall be persisted across application restarts.

#### Scenario: Save Workspace Path

- **GIVEN** a valid Git repository has been selected
- **WHEN** the path is validated
- **THEN** the path is written to `~/.config/pmux/config.json`
- **AND** the JSON structure includes: `{ "recent_workspace": "/path/to/repo" }`

#### Scenario: Load Workspace Path

- **GIVEN** a previous session saved a workspace path
- **WHEN** pmux starts
- **THEN** the configuration file is read
- **AND** if `recent_workspace` exists and points to a valid directory, it is loaded automatically
- **AND** the startup page is skipped

#### Scenario: Invalid Saved Path

- **GIVEN** a workspace path was previously saved
- **AND** the directory no longer exists or is no longer accessible
- **WHEN** pmux starts
- **THEN** the invalid path is detected
- **AND** the startup page is displayed instead
- **AND** the invalid entry is cleared from the config

### Requirement: Error Handling

The application shall handle errors gracefully with user-friendly messages.

#### Scenario: Config Read Error

- **GIVEN** the config file exists but is corrupted or unreadable
- **WHEN** pmux starts
- **THEN** the error is logged
- **AND** the startup page is displayed (as if no config existed)
- **AND** a new config will be created on successful workspace selection

#### Scenario: Config Write Error

- **GIVEN** a valid workspace has been selected
- **WHEN** attempting to save the config fails (e.g., permission denied)
- **THEN** an error message is displayed: "Failed to save workspace preference. You may need to select the workspace again on next launch."
- **AND** the application still proceeds to load the workspace for the current session

#### Scenario: File Picker Unavailable

- **GIVEN** the system file picker cannot be opened (rare edge case)
- **WHEN** the user clicks "Select workspace"
- **THEN** an error message is displayed: "Could not open file picker. Please check your system permissions."
- **AND** the startup page remains visible
