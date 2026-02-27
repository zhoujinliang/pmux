## ADDED Requirements

### Requirement: Dialog for creating new branch and worktree
The system SHALL provide a dialog for users to create new branches and associated worktrees.

#### Scenario: Opening new branch dialog
- **WHEN** user clicks "+ New Branch" button or presses ⌘⇧N
- **THEN** a modal dialog opens
- **AND** dialog contains:
  - Branch name input field
  - Base branch dropdown (default: main)
  - Create and Cancel buttons

#### Scenario: Creating new branch with valid name
- **GIVEN** user enters "feat/auth" as branch name
- **AND** selects "main" as base branch
- **WHEN** user clicks Create button
- **THEN** the system executes:
  ```
  git branch feat/auth
  git worktree add ../repo-feat-auth feat/auth
  ```
- **AND** creates new tmux window/pane for the worktree
- **AND** adds new entry to sidebar
- **AND** adds new tab to tab bar
- **AND** automatically switches to new pane

#### Scenario: Creating branch with invalid name
- **GIVEN** user enters a branch name that already exists
- **WHEN** user clicks Create
- **THEN** the system shows error "Branch already exists"
- **AND** remains in dialog

#### Scenario: Canceling branch creation
- **WHEN** user clicks Cancel button
- **THEN** the dialog closes
- **AND** no changes are made

### Requirement: Sidebar refreshes after branch creation
The system SHALL automatically refresh the sidebar to show newly created branches.

#### Scenario: After successful branch creation
- **GIVEN** new branch "feat/auth" was just created
- **WHEN** the operation completes
- **THEN** sidebar updates to show "feat/auth" entry
- **AND** the entry shows "Unknown" status initially
- **AND** status updates once agent starts running

### Requirement: New branch pane starts in correct directory
The system SHALL ensure new panes start in the correct worktree directory.

#### Scenario: New pane directory
- **GIVEN** new worktree created at ../repo-feat-auth
- **WHEN** tmux pane is created
- **THEN** pane starts in ../repo-feat-auth directory
- **AND** shell prompt reflects the path
