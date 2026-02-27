## ADDED Requirements

### Requirement: User can open diff view for any worktree
The system SHALL allow users to open a diff view comparing any branch to main.

#### Scenario: Opening diff via keyboard shortcut
- **GIVEN** user is viewing a worktree
- **WHEN** user presses ⌘⇧R
- **THEN** a diff view opens for current branch

#### Scenario: Opening diff via context menu
- **GIVEN** user right-clicks on a worktree entry
- **WHEN** user selects "View Diff"
- **THEN** a diff view opens for that branch

### Requirement: Diff view uses nvim diffview in tmux
The system SHALL create a tmux window running nvim with diffview plugin.

#### Scenario: Creating diff view
- **GIVEN** user requests diff for branch "feat-x"
- **WHEN** diff view is opened
- **THEN** system executes:
  ```
  tmux new-window -t sdlc-repo -n "review-feat-x"
  tmux send-keys "nvim -c 'DiffviewOpen main...HEAD'" C-m
  ```
- **AND** a new "review" tab appears in tab bar

#### Scenario: Diff view rendering
- **GIVEN** diff tmux window is created
- **WHEN** TerminalView renders the review tab
- **THEN** nvim diffview interface is displayed
- **AND** shows changed files list and diff content

### Requirement: Diff view appears as special review tab
The system SHALL display diff views as specially marked tabs.

#### Scenario: Review tab appearance
- **GIVEN** diff view is open for "feat-x"
- **WHEN** tab bar renders
- **THEN** tab shows "📝 review-feat-x"
- **AND** tab has different styling from regular pane tabs

### Requirement: User can close diff view
The system SHALL allow users to close diff views and return to agent tabs.

#### Scenario: Closing via keyboard
- **GIVEN** user is viewing diff tab
- **WHEN** user presses ⌘W
- **THEN** the review tab closes
- **AND** tmux window is killed
- **AND** focus returns to previous agent tab

#### Scenario: Closing via nvim
- **GIVEN** user is in nvim diffview
- **WHEN** user types :q and exits nvim
- **THEN** the review tab automatically closes
- **AND** focus returns to agent tab

### Requirement: Only one diff view per branch
The system SHALL reuse existing diff views rather than creating duplicates.

#### Scenario: Opening diff for branch with existing review
- **GIVEN** review tab already exists for "feat-x"
- **WHEN** user requests diff for "feat-x" again
- **THEN** system switches to existing review tab
- **AND** does not create new tmux window

### Requirement: Diff view updates on file changes
The system SHALL reflect file changes in the diff view.

#### Scenario: File modified while diff open
- **GIVEN** diff view is open showing some changes
- **WHEN** agent modifies more files in the worktree
- **THEN** nvim diffview updates to show new changes
- **AND** user can refresh with standard nvim commands
