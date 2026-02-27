## ADDED Requirements

### Requirement: Worktree deletion requires user confirmation

The system SHALL display a confirmation dialog when the user initiates worktree deletion.

#### Scenario: User clicks delete button on Sidebar worktree
- **WHEN** user clicks the delete button on a Sidebar worktree entry
- **THEN** a confirmation dialog SHALL appear
- **AND** the dialog SHALL display the worktree path and branch name
- **AND** the dialog SHALL include "Cancel" and "Delete" buttons
- **AND** no other UI components SHALL be interactive while the dialog is open

#### Scenario: User cancels worktree deletion
- **WHEN** user clicks "Cancel" in the deletion confirmation dialog
- **THEN** the dialog SHALL close
- **AND** the worktree SHALL remain in the list
- **AND** no changes SHALL be made to the git worktree

### Requirement: Worktree deletion cleans up git worktree

The system SHALL remove the git worktree from the repository when the user confirms deletion.

#### Scenario: Successful git worktree removal
- **WHEN** user confirms deletion of a worktree in the confirmation dialog
- **THEN** the system SHALL execute `git worktree remove` command
- **AND** the worktree directory SHALL be removed from the filesystem
- **AND** the worktree SHALL be removed from the git repository's worktree list
- **AND** the worktree entry SHALL be removed from the Sidebar list
- **AND** the corresponding TabBar tab SHALL be removed if it exists

#### Scenario: Worktree removal fails due to uncommitted changes
- **WHEN** user confirms deletion of a worktree that has uncommitted changes
- **THEN** the deletion SHALL succeed (git worktree remove handles uncommitted changes)
- **AND** the uncommitted changes SHALL be discarded

#### Scenario: Worktree removal fails due to lock
- **WHEN** git worktree remove command fails due to a lock or busy state
- **THEN** the deletion dialog SHALL display an error message
- **AND** the worktree SHALL remain in the list
- **AND** the dialog SHALL remain open with a "Retry" button

### Requirement: Worktree deletion terminates tmux session pane

The system SHALL terminate the tmux pane associated with the worktree when deletion is confirmed.

#### Scenario: Tmux pane exists and is terminated
- **WHEN** user confirms deletion of a worktree with an active tmux session
- **THEN** the system SHALL execute `tmux kill-pane` for the worktree's pane
- **AND** the tmux pane SHALL be terminated
- **AND** the git worktree removal SHALL proceed

#### Scenario: Tmux pane does not exist
- **WHEN** user confirms deletion of a worktree that does not have an active tmux session
- **THEN** the system SHALL skip tmux pane termination
- **AND** the git worktree removal SHALL proceed successfully
- **AND** no error SHALL be displayed for missing tmux pane

#### Scenario: Tmux kill-pane fails
- **WHEN** tmux kill-pane command fails for any reason
- **THEN** the system SHALL log the failure
- **AND** the git worktree removal SHALL continue
- **AND** the worktree deletion SHALL complete successfully

### Requirement: Worktree deletion warns about uncommitted changes

The system SHALL warn the user when attempting to delete a worktree that has uncommitted changes.

#### Scenario: Worktree has uncommitted changes
- **WHEN** user initiates deletion of a worktree that has uncommitted changes
- **THEN** the deletion confirmation dialog SHALL display a warning message
- **AND** the warning SHALL indicate that uncommitted changes will be discarded
- **AND** the warning SHALL be displayed in bold text
- **AND** the deletion SHALL proceed when user confirms

#### Scenario: Worktree has no uncommitted changes
- **WHEN** user initiates deletion of a worktree with no uncommitted changes
- **THEN** the deletion confirmation dialog SHALL NOT display any warning about changes
- **AND** the dialog SHALL display only the worktree path and branch name

### Requirement: Workspace manager updates after worktree deletion

The system SHALL update the workspace manager state to reflect the removed worktree.

#### Scenario: Workspace manager removes deleted worktree
- **WHEN** a worktree is successfully deleted
- **THEN** the WorkspaceManager SHALL remove the worktree from its tab list
- **AND** if the deleted worktree was the active tab, another tab SHALL become active if available
- **AND** if no tabs remain, no tab SHALL be active
- **AND** the UI SHALL refresh to show the updated state

#### Scenario: Deleting the only worktree
- **WHEN** user deletes the only worktree in the workspace
- **THEN** the WorkspaceManager tab list SHALL become empty
- **AND** no TabBar tab SHALL be visible
- **AND** the Sidebar worktree list SHALL be empty or show only non-worktree entries