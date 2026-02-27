## Why

The TabBar and sidebar currently operate independently without synchronization. Users can open multiple worktree tabs, but selecting a worktree in the sidebar doesn't activate the corresponding tab, and there's no way to delete worktrees from the UI. This disconnect creates confusion and prevents seamless worktree management.

## What Changes

- Implement bidirectional synchronization between Sidebar worktree list and TabBar worktree tabs
- Add a worktree deletion confirmation dialog with git worktree removal and tmux pane cleanup
- Enable switching between worktrees via both Sidebar and TabBar with consistent state
- Add delete button/accessibility to worktree entries

## Capabilities

### New Capabilities

- `tabbar-sidebar-sync`: Bidirectional state synchronization between sidebar worktree list and TabBar tabs, ensuring consistent active worktree across both UI components
- `worktree-deletion`: Worktree removal flow with confirmation dialog, git worktree cleanup, and tmux pane termination

### Modified Capabilities

None

## Impact

- `src/ui/sidebar.rs`: Add click handlers for worktree selection and delete actions
- `src/ui/tabbar.rs`: Expose API for external tab activation and close coordination
- `src/workspace_manager.rs`: Add deletion method and sync callbacks
- `src/tmux/session.rs`: Add kill-pane method for worktree-specific pane cleanup
- `src/worktree_manager.rs`: Add remove_worktree method with validation
- UI state flow: AppRoot will coordinate sidebar↔tabbar synchronization events