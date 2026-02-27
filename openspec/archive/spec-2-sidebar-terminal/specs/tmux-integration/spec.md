# tmux 集成规格

## ADDED Requirements

### Requirement: Session Management

The application must manage tmux sessions for each workspace.

#### Scenario: Create New Session

- **WHEN** the user selects a workspace and no tmux session exists
- **THEN** a new tmux session is created with name `sdlc-<repo-name>`
- **AND** a default window named "control-tower" is created
- **AND** the session is detached (running in background)

#### Scenario: Attach Existing Session

- **WHEN** the user selects a workspace and a tmux session already exists
- **THEN** the application attaches to the existing session
- **AND** all existing panes are discovered and displayed

#### Scenario: Session Persistence

- **WHEN** the application closes
- **THEN** the tmux session continues running
- **AND** when the application reopens, it can re-attach to the same session

### Requirement: Pane Management

The application must create and manage tmux panes for each worktree.

#### Scenario: Create Pane for Worktree

- **WHEN** a worktree is discovered without a corresponding pane
- **THEN** a new pane is created in the tmux session
- **AND** the pane's working directory is set to the worktree path
- **AND** the pane ID is stored for future reference

#### Scenario: Capture Pane Content

- **GIVEN** a tmux pane exists
- **WHEN** the application polls for content (every 50ms)
- **THEN** the current pane content is captured using `tmux capture-pane`
- **AND** the content is parsed and displayed in TerminalView

#### Scenario: Send Input to Pane

- **GIVEN** a tmux pane is active
- **WHEN** the user types on the keyboard
- **AND** the key is not an application shortcut
- **THEN** the key is sent to the pane using `tmux send-keys`
