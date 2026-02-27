## 1. WorkspaceManager API Extensions

- [x] 1.1 Add `pub fn set_active_tab(&mut self, worktree_id: &str)` method to WorkspaceManager
- [x] 1.2 Add `pub fn close_tab(&mut self, worktree_id: &str) -> Result<(), WorkspaceError>` method to WorkspaceManager
- [x] 1.3 Add `pub fn remove_tab(&mut self, worktree_id: &str)` method to WorkspaceManager for deletion
- [ ] 1.4 Ensure tab state changes trigger `cx.notify()` for UI re-renders
- [x] 1.5 Add unit tests for set_active_tab, close_tab, and remove_tab methods

## 2. TabBar External Activation Support

- [ ] 2.1 Update TabBar to subscribe to WorkspaceManager state changes
- [ ] 2.2 Ensure TabBar renders active tab state correctly based on WorkspaceManager
- [ ] 2.3 Add test verifying TabBar activates when WorkspaceManager changes
- [ ] 2.4 Update TabBar close handler to trigger WorkspaceManager.close_tab()

## 3. Sidebar Selection Handlers

- [ ] 3.1 Add click event handler for worktree entries in Sidebar
- [ ] 3.2 Call WorkspaceManager.set_active_tab() on worktree entry click
- [ ] 3.3 Add visual highlighting for selected worktree in Sidebar
- [ ] 3.4 Add delete button to each Sidebar worktree entry (× icon)
- [ ] 3.5 Wire delete button click to show deletion dialog in AppRoot
- [ ] 3.6 Update Sidebar to subscribe to WorkspaceManager state changes for selection sync
- [ ] 3.7 Add tests for Sidebar click and delete button handlers

## 4. DeleteWorktreeDialogUI Component

- [ ] 4.1 Create new `DeleteWorktreeDialogUI` struct in `src/ui/new_branch_dialog_ui.rs` or separate file
- [ ] 4.2 Implement modal dialog with backdrop overlay
- [ ] 4.3 Render worktree path and branch name in dialog
- [ ] 4.4 Add "Cancel" button to close dialog without deletion
- [ ] 4.5 Add "Delete" button to confirm deletion
- [ ] 4.6 Display warning message for uncommitted changes (if any)
- [ ] 4.7 Display error message with "Retry" button if deletion fails
- [ ] 4.8 Ensure dialog blocks other UI interaction when open
- [ ] 4.9 Add tests for dialog rendering and button interactions

## 5. AppRoot Dialog State Management

- [ ] 5.1 Add `DeleteWorktreeDialogState` field to AppRoot with worktree and show fields
- [ ] 5.2 Implement `show_delete_dialog(&mut self, worktree: WorktreeInfo, cx: &mut Context<Self>)` method
- [ ] 5.3 Implement `close_delete_dialog(&mut self, cx: &mut Context<Self>)` method
- [ ] 5.4 Render DeleteWorktreeDialogUI conditionally when show is true
- [ ] 5.5 Wire "Delete" button to trigger deletion workflow
- [ ] 5.6 Wire "Cancel" and "Retry" buttons to close dialog

## 6. Git Worktree Removal Integration

- [ ] 6.1 Add `remove_worktree(&self, path: &Path) -> Result<(), WorktreeError>` method to WorktreeManager
- [ ] 6.2 Implement git worktree validation before removal
- [ ] 6.3 Execute `git worktree remove` command in remove_worktree method
- [ ] 6.4 Handle git worktree errors (lock, busy, etc.) with user-friendly messages
- [ ] 6.5 Add unit tests for remove_worktree with success and error cases
- [ ] 6.6 Integration test for WorktreeManager.remove_worktree

## 7. Tmux Pane Cleanup

- [ ] 7.1 Add `kill_pane(&self, pane_id: &str) -> Result<(), PaneError>` method to tmux session/pane module
- [ ] 7.2 Check if tmux session exists before attempting kill-pane
- [ ] 7.3 Implement graceful error handling for missing sessions (best-effort cleanup)
- [ ] 7.4 Log failures without blocking git worktree removal
- [ ] 7.5 Add unit tests for kill_pane success and failure scenarios
- [ ] 7.6 Integration test for tmux cleanup during worktree deletion

## 8. Deletion Workflow Orchestration

- [ ] 8.1 Implement `confirm_delete_worktree(&mut self, worktree: WorktreeInfo, cx: &mut Context<Self>)` in AppRoot
- [ ] 8.2 Check for uncommitted changes with `git status --short` before deletion
- [ ] 8.3 Execute tmux kill-pane first (best-effort)
- [ ] 8.4 Execute git worktree remove second
- [ ] 8.5 Call WorkspaceManager.remove_tab() on successful git removal
- [ ] 8.6 Refresh worktree list after successful deletion
- [ ] 8.7 Display error in dialog if git removal fails
- [ ] 8.8 Close dialog and update UI on successful deletion
- [ ] 8.9 Add integration test for complete deletion workflow

## 9. UI State Synchronization

- [ ] 9.1 Update AppRoot to pass WorkspaceManager state changes to Sidebar
- [ ] 9.2 Update AppRoot to pass WorkspaceManager state changes to TabBar
- [ ] 9.3 Ensure Sidebar selection clears when corresponding TabBar tab is closed
- [ ] 9.4 Test bidirectional sync: Sidebar → TabBar and TabBar → Sidebar
- [ ] 9.5 Add integration test for state consistency across components

## 10. Error Handling and User Feedback

- [ ] 10.1 Add Chinese error messages for git worktree removal failures
- [ ] 10.2 Add Chinese error messages for tmux operation failures
- [ ] 10.3 Ensure error messages display in deletion dialog
- [ ] 10.4 Add toast or notification for successful worktree deletion
- [ ] 10.5 Test error scenarios: locked worktree, busy tmux, permission errors

## 11. Testing and Verification

- [ ] 11.1 Run all unit tests and ensure they pass
- [ ] 11.2 Run all integration tests and ensure they pass
- [ ] 11.3 Manual test: Click Sidebar worktree, verify TabBar activates
- [ ] 11.4 Manual test: Click TabBar tab, verify Sidebar selects
- [ ] 11.5 Manual test: Delete worktree with no uncommitted changes
- [ ] 11.6 Manual test: Delete worktree with uncommitted changes (verify warning)
- [ ] 11.7 Manual test: Attempt to delete locked worktree (verify error handling)
- [ ] 11.8 Manual test: Delete worktree with no tmux session (verify best-effort cleanup)
- [ ] 11.9 Manual test: Delete the only worktree (verify empty state handling)
- [ ] 11.10 Manual test: Close TabBar tab, verify Sidebar selection clears