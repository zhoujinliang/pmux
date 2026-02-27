## ADDED Requirements

### Requirement: Sidebar displays repository and worktree information
The system SHALL display a sidebar showing the current repository name and all worktrees.

#### Scenario: Single main branch view
- **GIVEN** user has opened a git repository
- **WHEN** the workspace view loads
- **THEN** the sidebar displays the repository name at top
- **AND** shows the main branch worktree entry
- **AND** the main branch entry is selected by default

#### Scenario: Multiple worktrees
- **GIVEN** repository has multiple worktrees (main, feat-x, fix-bug)
- **WHEN** the sidebar renders
- **THEN** all worktrees are listed vertically
- **AND** each shows the branch name
- **AND** each shows ahead count if any (+2)

### Requirement: Sidebar supports selection and highlighting
The system SHALL allow users to select worktree entries with visual highlighting.

#### Scenario: Selecting a worktree
- **WHEN** user clicks on a worktree entry in sidebar
- **THEN** the entry becomes highlighted with blue background
- **AND** the corresponding pane/tab becomes active
- **AND** the terminal view switches to that worktree's pane

### Requirement: Sidebar can be toggled visible/hidden
The system SHALL allow users to collapse and expand the sidebar.

#### Scenario: Toggling sidebar with keyboard
- **WHEN** user presses ⌘B
- **THEN** the sidebar collapses if visible
- **AND** expands if hidden
- **AND** the main content area resizes accordingly

#### Scenario: Toggling sidebar with button
- **WHEN** user clicks the ≡ button in TopBar
- **THEN** the sidebar visibility toggles

### Requirement: Sidebar shows context menu for worktree actions
The system SHALL provide a context menu when right-clicking worktree entries.

#### Scenario: Right-clicking worktree entry
- **WHEN** user right-clicks on a worktree entry
- **THEN** a context menu appears with options:
  - View Diff
  - Remove Worktree

#### Scenario: Selecting View Diff from context menu
- **WHEN** user selects "View Diff" from context menu
- **THEN** the system opens the diff view for that branch

### Requirement: Sidebar provides New Branch button
The system SHALL provide a button to create new branches at the bottom of sidebar.

#### Scenario: Clicking New Branch button
- **WHEN** user clicks "+ New Branch" button
- **THEN** a dialog opens for entering branch name
- **AND** allows selecting base branch
