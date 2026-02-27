## Context

The pmux application currently renders a "+ New Branch" button in the Sidebar component but lacks the functionality to handle click events. The application uses GPUI for the UI framework and integrates with Git worktrees and tmux for workspace management. The `WorkspaceManager` component already handles multiple workspace tabs, and the `Sidebar` displays worktree items with status indicators.

The current flow requires users to manually execute `git worktree add` commands and create tmux sessions via the command line. This change will provide a GUI-based workflow that integrates seamlessly with the existing workspace management infrastructure.

## Goals / Non-Goals

**Goals:**

- Implement a complete GUI workflow for creating new branches and worktrees
- Integrate with existing `start_tmux_session` functionality
- Provide user feedback during the branch/worktree creation process
- Automatically refresh the Sidebar to display newly created workspaces
- Validate branch names and handle error conditions gracefully

**Non-Goals:**

- Branch switching (out of scope - existing Sidebar selection handles this)
- Branch deletion (will be covered in a separate specification)
- Worktree pruning or cleanup (future enhancement)
- Advanced Git operations (merge, rebase, cherry-pick - not part of this feature)

## Decisions

### Dialog Implementation

**Decision**: Use a modal input dialog for branch name entry within the GPUI framework.

**Rationale**: A modal dialog provides clear user focus and prevents interaction with other UI elements during branch creation. GPUI's `Input` and `Window` components can be used to implement this. This approach is consistent with modern GUI patterns and provides immediate validation feedback.

**Alternatives considered**:
- Sidebar text field: Would require persistent UI space and is less discoverable
- Command palette: Would require additional implementation overhead and may be less discoverable for this specific action

### Git Worktree Command Execution

**Decision**: Execute `git worktree add <path> -b <branch_name>` as a child process from Rust.

**Rationale**: This command creates both a new branch and the corresponding worktree directory in a single operation. Running as a child process from Rust provides control over the process lifecycle and enables proper error handling and status reporting.

**Alternatives considered**:
- libgit2 native bindings: Would add a dependency and increase complexity; command-line approach is simpler and provides clear error messages
- Two-step process (branch creation then worktree): Unnecessary complexity; single command achieves the goal

### Tmux Pane Creation

**Decision**: Reuse existing `start_tmux_session` functionality for new worktree pane creation.

**Rationale**: The application already has a mechanism for starting tmux sessions (`start_tmux_session`), so reusing this ensures consistency with existing workspace management behavior. This maintains a single source of truth for tmux integration.

**Alternatives considered**:
- Direct tmux command execution: Would duplicate functionality and diverge from existing patterns
- New pane creation method: Would introduce redundant code

### State Management

**Decision**: Update Sidebar by calling `set_worktrees` with the refreshed worktree list after successful creation.

**Rationale**: The `Sidebar` component already has a `set_worktrees` method that accepts a vector of `WorktreeInfo`. After successful worktree creation, we'll re-fetch the worktree list and pass it to the Sidebar. This ensures the UI reflects the current state.

**Alternatives considered**:
- Append to existing worktrees list: Risk of state desynchronization; re-fetching is more reliable
- Manual UI updates: More error-prone and harder to maintain

### Error Handling

**Decision**: Display error messages as toast notifications and keep the dialog open for user correction.

**Rationale**: Toast notifications provide non-blocking feedback while allowing the user to correct input errors. Keeping the dialog open enables quick retry without requiring multiple clicks.

**Alternatives considered**:
- Alert dialog: More disruptive for minor errors
- Silent failure: Poor user experience

## Risks / Trade-offs

### Risk: Concurrent worktree creation
[Risk] Multiple rapid clicks on "+ New Branch" could lead to duplicate branch creation attempts.
→ Mitigation: Disable the button during branch creation process and re-enable after completion or error.

### Risk: Invalid branch names
[Risk] Git branch names have constraints (no spaces, special characters, etc.).
→ Mitigation: Validate branch name input using regex before attempting creation, display specific validation errors.

### Risk: Git command failures
[Risk] The `git worktree add` command may fail due to network issues, merge conflicts, or repository state.
→ Mitigation: Capture and display the full error message from the git command, allowing users to resolve underlying issues.

### Trade-off: Synchronous vs asynchronous operation
[Trade-off] Synchronous operation blocks the UI during branch creation, while asynchronous adds complexity.
→ Decision: Use asynchronous execution with a loading indicator. This provides better user experience and prevents UI freezing, which is critical for a GUI application.

### Risk: Tmux session conflicts
[Risk] Tmux session name conflicts could occur if a session with the same name already exists.
→ Mitigation: Generate unique session names using the worktree path and a timestamp, or use tmux's automatic session naming.

## Open Questions

1. Should branch names be validated against existing branches to prevent duplicates?
   - Recommendation: Yes, validate against existing branches and offer to switch to existing branch if name matches

2. Should the user be able to specify a parent branch for the new branch?
   - Recommendation: Initially use the currently selected worktree's branch as parent. Future enhancement could add branch selection dialog.

3. Should the new worktree pane be automatically selected after creation?
   - Recommendation: Yes, automatically select the new workspace to provide immediate feedback and enable immediate work