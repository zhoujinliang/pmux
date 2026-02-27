## ADDED Requirements

### Requirement: User can open new branch creation dialog
The system SHALL provide a "+ New Branch" button in the Sidebar that opens a modal dialog for entering a new branch name when clicked.

#### Scenario: Opening branch creation dialog
- **WHEN** user clicks the "+ New Branch" button in the Sidebar
- **THEN** system displays a modal dialog with an input field for branch name
- **AND** the input field is focused and ready for text entry
- **AND** the "+ New Branch" button is disabled to prevent concurrent creation attempts

#### Scenario: Dialog displays validation hints
- **WHEN** the new branch dialog is displayed
- **THEN** the system shows validation criteria (no spaces, valid Git branch name format)
- **AND** placeholder text indicates expected branch name format

### Requirement: System validates branch name before creation
The system SHALL validate the entered branch name against Git branch naming rules before attempting creation.

#### Scenario: Valid branch name
- **WHEN** user enters a valid Git branch name (e.g., "feature/new-functionality", "fix-bug-123")
- **THEN** the validation passes without displaying errors
- **AND** the "Create" button remains enabled

#### Scenario: Invalid branch name with spaces
- **WHEN** user enters a branch name containing spaces
- **THEN** system displays an error message indicating spaces are not allowed
- **AND** the "Create" button is disabled
- **AND** the dialog remains open for correction

#### Scenario: Invalid branch name with special characters
- **WHEN** user enters a branch name with invalid Git characters (e.g., "~", "^", ":", "..")
- **THEN** system displays an error message with specific invalid characters
- **AND** the "Create" button is disabled
- **AND** the dialog remains open for correction

#### Scenario: Duplicate branch name
- **WHEN** user enters a branch name that already exists in the repository
- **THEN** system displays an error message indicating the branch already exists
- **AND** offers an option to switch to the existing branch
- **AND** the dialog remains open for correction

### Requirement: System creates new Git branch and worktree
The system SHALL execute `git worktree add` command to create both a new branch and corresponding worktree directory when validation passes.

#### Scenario: Successful branch and worktree creation
- **WHEN** user clicks "Create" button with a valid, unique branch name
- **THEN** system executes `git worktree add <path> -b <branch_name>` asynchronously
- **AND** displays a loading indicator during command execution
- **AND** creates the new worktree directory at the appropriate path
- **AND** creates the new branch in the Git repository

#### Scenario: Git command failure
- **WHEN** the `git worktree add` command fails (network error, merge conflict, repository state)
- **THEN** system displays a toast notification with the full error message
- **AND** the dialog remains open for user correction
- **AND** the "+ New Branch" button is re-enabled

#### Scenario: Concurrent creation attempt
- **WHEN** user clicks "+ New Branch" while a creation is in progress
- **THEN** the button does not trigger another dialog
- **AND** the existing dialog continues to process the current creation

### Requirement: System creates tmux pane for new worktree
The system SHALL create a new tmux pane for the newly created worktree using the existing `start_tmux_session` functionality.

#### Scenario: Successful tmux pane creation
- **WHEN** git worktree creation completes successfully
- **THEN** system calls `start_tmux_session` with the new worktree path
- **AND** generates a unique tmux session name to avoid conflicts
- **AND** creates a new tmux pane for the worktree

#### Scenario: Tmux pane creation failure
- **WHEN** tmux pane creation fails after successful worktree creation
- **THEN** system displays an error notification indicating tmux creation failed
- **AND** informs the user that the worktree exists but tmux session was not created
- **AND** provides instructions for manual tmux session creation

### Requirement: System refreshes Sidebar to display new workspace
The system SHALL refresh the Sidebar to display the newly created workspace entry after successful branch and worktree creation.

#### Scenario: Sidebar refresh after creation
- **WHEN** both git worktree and tmux pane creation complete successfully
- **THEN** system re-fetches the worktree list
- **AND** calls `set_worktrees` on the Sidebar with the updated list
- **AND** the new workspace entry is displayed in the Sidebar
- **AND** the new workspace is automatically selected

#### Scenario: New workspace displays correct status
- **WHEN** the new workspace is displayed in the Sidebar
- **THEN** the workspace shows the branch name
- **AND** the status indicator is set to "Idle" (○)
- **AND** the workspace can be selected and activated

### Requirement: System provides user feedback during creation process
The system SHALL provide clear visual feedback throughout the branch/worktree creation process.

#### Scenario: Loading indicator during creation
- **WHEN** the git worktree command is executing
- **THEN** the dialog displays a loading indicator
- **AND** the input field is disabled
- **AND** the "Create" button displays "Creating..."

#### Scenario: Success notification
- **WHEN** branch, worktree, and tmux pane creation complete successfully
- **THEN** the dialog closes automatically
- **AND** a success toast notification is displayed
- **AND** the Sidebar refreshes to show the new workspace
- **AND** the "+ New Branch" button is re-enabled

#### Scenario: Error notification
- **WHEN** any step of the creation process fails
- **THEN** a toast notification displays the error message
- **AND** the dialog remains open for correction
- **AND** the "+ New Branch" button is re-enabled

### Requirement: User can cancel branch creation
The system SHALL allow users to cancel the branch creation process by closing the dialog.

#### Scenario: Canceling dialog before creation
- **WHEN** user clicks "Cancel" button or closes the dialog before clicking "Create"
- **THEN** the dialog closes without executing any git commands
- **AND** the "+ New Branch" button is re-enabled
- **AND** no worktree or branch is created

#### Scenario: Canceling during creation
- **WHEN** user attempts to close the dialog while creation is in progress
- **THEN** the dialog cannot be closed until creation completes or fails
- **AND** a message indicates that creation is in progress