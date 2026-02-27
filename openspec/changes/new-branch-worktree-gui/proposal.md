## Why

Users need an intuitive GUI workflow to create new Git branches and corresponding worktrees. The current implementation renders the "+ New Branch" button in the Sidebar but lacks the click event handler. This change enables seamless branch/worktree creation through a simple GUI interaction, reducing reliance on command-line tools and improving productivity.

## What Changes

- Implement click event handler for the "+ New Branch" button in the Sidebar
- Add input dialog for branch name entry
- Execute `git worktree add` command to create new worktree
- Create new tmux pane for the worktree
- Refresh Sidebar to display the new workspace entry

## Capabilities

### New Capabilities

- `branch-worktree-creation`: GUI workflow for creating new branches and worktrees through a dialog-based interface

### Modified Capabilities

None - this introduces entirely new functionality

## Impact

- **Sidebar component**: Add event handler for "+ New Branch" button
- **Workspace management**: Integrate with existing `start_tmux_session` and workspace tracking
- **Git integration**: Execute `git worktree add` commands
- **Tmux integration**: Create panes for new worktrees
- **State management**: Update Sidebar to reflect newly created workspaces