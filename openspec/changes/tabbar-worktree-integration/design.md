## Context

The pmux application currently has separate TabBar (`src/ui/tabbar.rs`) and Sidebar (`src/ui/sidebar.rs`) components that manage worktree state independently. The WorkspaceManager (`src/workspace_manager.rs`) already maintains a list of active worktree tabs with activation state. However, there's no coordination mechanism:

- Clicking a worktree in the Sidebar doesn't switch the active TabBar tab
- Closing a TabBar tab doesn't update Sidebar selection
- Deleting a worktree has no UI affordance or workflow

The application uses GPUI's event-driven model where state changes trigger `cx.notify()` for UI re-renders. Tmux sessions are managed per-worktree with background polling for terminal content updates.

**Constraints:**
- Must work within existing GPUI architecture (no framework changes)
- Git worktree operations should be validated and safe
- Tmux pane cleanup must handle sessions gracefully
- Error messages should be user-friendly (Chinese language target audience)

## Goals / Non-Goals

**Goals:**
- Establish bidirectional sync between Sidebar and TabBar so selecting a worktree in either updates the other
- Provide a safe worktree deletion workflow with user confirmation and proper cleanup
- Maintain existing tmux session behavior while adding deletion capability
- Keep UI responsive during async git operations

**Non-Goals:**
- Complex drag-and-drop tab reordering
- Multi-select worktree operations
- Worktree conflict resolution UI (will fail with error message)
- Worktree rename functionality
- Branch switching within worktrees (requires separate feature)

## Decisions

### 1. Synchronization Architecture

**Decision:** Use WorkspaceManager as the single source of truth with event callbacks

WorkspaceManager already tracks active worktree tabs. We'll add:
- `pub fn set_active_tab(&mut self, worktree_id: &str)` - external activation API
- `pub fn close_tab(&mut self, worktree_id: &str) -> Result<(), WorkspaceError>` - external close API
- Event emission through `cx.notify()` when active tab changes

Sidebar and TabBar will subscribe to WorkspaceManager state changes. When Sidebar selection changes:
```
sidebar.on_worktree_click(id) → WorkspaceManager.set_active_tab(id) → TabBar re-renders
```

When TabBar tab is activated:
```
tabbar.on_tab_click(id) → WorkspaceManager.set_active_tab(id) → Sidebar re-renders
```

**Rationale:** Centralizing state in WorkspaceManager prevents sync bugs and reduces complexity. Both components already depend on it, so this extends the existing pattern. Alternative approaches considered:
- Direct Sidebar↔TabBar communication (rejected: tight coupling, harder to test)
- Global event bus (rejected: overkill, adds complexity)

### 2. Deletion Dialog UI Pattern

**Decision:** Create new `DeleteWorktreeDialogUI` component as a modal overlay

The dialog will be:
- Rendered as a centered modal with backdrop
- Display worktree path and branch name
- Show warning about uncommitted changes (via git status check)
- Have "Cancel" and "Delete" buttons

Dialog state managed in AppRoot:
```rust
struct DeleteWorktreeDialogState {
    worktree: Option<WorktreeInfo>,
    show: bool,
}
```

**Rationale:** Modal pattern matches existing `NewBranchDialogUI`. Reuse the modal infrastructure for consistency. Alternative: inline delete with undo (rejected: more complex state management, git worktree removal is destructive).

### 3. Deletion Flow Sequence

**Decision:** Validate → Show Dialog → Cleanup → Refresh

```
1. Sidebar.delete_button_click(id)
2. AppRoot.show_delete_dialog(worktree)
3. User confirms
4. git worktree remove (with validation)
5. tmux kill-pane (find pane by worktree name)
6. WorkspaceManager.remove_tab(id)
7. Refresh worktree list
```

**Rationale:** Git cleanup before tmux to ensure filesystem changes commit first. Tmux pane may already be dead (session closed), so kill-pane should handle errors gracefully. Alternative: kill tmux first (rejected: could leave orphaned git worktrees).

### 4. Worktree Identification

**Decision:** Use worktree path as unique identifier

WorktreeInfo already has a `path` field that's guaranteed unique by git. TabBar and Sidebar will use this as the worktree_id.

**Rationale:** Git worktrees cannot have duplicate paths. Using path is more stable than branch names (branches can be deleted/renamed). Alternative: use git worktree ID (rejected: not easily accessible across operations).

## Risks / Trade-offs

**[Race condition during deletion]**
User could attempt to switch to a worktree while it's being deleted.
→ Mitigation: Disable Sidebar/TabBar interaction while deletion dialog is active. Use `Arc<Mutex<>>` for WorkspaceManager if needed (likely not due to single-threaded GPUI).

**[Tmux pane not found]**
Kill-pane may fail if the session was already closed externally.
→ Mitigation: Treat kill-pane as best-effort operation. Log warning but continue with git cleanup. Check if session exists first using `tmux has-session`.

**[Uncommitted changes]**
Deleting a worktree with uncommitted changes loses data.
→ Mitigation: Run `git status --short` before showing dialog. If changes exist, show warning message in dialog with bold text. Let user decide (common in git worktree workflows).

**[Git worktree busy]**
Worktree may be locked (e.g., another git process running).
→ Mitigation: `git worktree remove` will fail with clear error message. Display error in dialog and keep worktree in list. User can retry after closing other processes.

**[Performance with many worktrees]**
Sidebar list with 100+ worktrees could slow re-renders.
→ Mitigation: Use virtualization if needed. For now, keep simple since typical use case is <20 worktrees. Monitor performance after implementation.

## Migration Plan

This is a new feature with no breaking changes. Deployment steps:
1. Add new methods to WorkspaceManager (backward compatible)
2. Add DeleteWorktreeDialogUI component
3. Wire up Sidebar click handlers
4. Test with small worktree count (2-3 tabs)
5. Test with larger worktree count (10+ tabs)
6. Verify deletion workflow success and error paths

**Rollback:** If issues arise, can disable deletion by removing Sidebar delete button while keeping sync functionality. No data migration needed.

## Open Questions

- Should we prompt for confirmation if worktree has uncommitted changes? **Decision: Show warning but still allow deletion (git worktree remove handles this)**
- Should tmux kill-pane fail if session doesn't exist? **Decision: No, best-effort cleanup is sufficient**
- How to display deletion errors to user? **Decision: Show in dialog with "Retry" button, similar to NewBranchDialogUI error handling**