## 1. Dialog Component

- [x] 1.1 Create NewBranchDialog component struct with branch name input state
- [x] 1.2 Implement modal dialog UI with input field, placeholder text, and validation hints
- [x] 1.3 Add Create and Cancel buttons to the dialog
- [x] 1.4 Implement dialog open/close state management
- [x] 1.5 Add loading indicator UI (hidden by default)
- [x] 1.6 Connect "+ New Branch" button click event to open the dialog
- [x] 1.7 Implement button disable state during creation (disable "+ New Branch" button)
- [x] 1.8 Add focus management (auto-focus input field when dialog opens)

## 2. Branch Name Validation

- [x] 2.1 Create validate_branch_name function with regex for Git branch naming rules
- [x] 2.2 Implement real-time validation as user types
- [x] 2.3 Add validation error message display in dialog
- [x] 2.4 Implement space character detection and error display
- [x] 2.5 Implement invalid special character detection and error display
- [x] 2.6 Add existing branch check by querying git branch list
- [x] 2.7 Implement "Create" button disable logic based on validation state
- [x] 2.8 Add "Switch to existing branch" option when duplicate detected

## 3. Git Worktree Creation

- [x] 3.1 Create async function to execute `git worktree add` command
- [x] 3.2 Implement child process spawning with command: `git worktree add <path> -b <branch_name>`
- [x] 3.3 Add worktree path generation logic (repository path + branch name)
- [x] 3.4 Implement command output capture (stdout, stderr)
- [x] 3.5 Add loading state management during command execution
- [x] 3.6 Implement success detection from git command exit code
- [x] 3.7 Add error message extraction and formatting for display
- [x] 3.8 Implement retry logic for transient failures (optional)

## 4. Tmux Pane Integration

- [x] 4.1 Locate and understand existing `start_tmux_session` function
- [x] 4.2 Call `start_tmux_session` with new worktree path after successful git worktree creation
- [x] 4.3 Generate unique tmux session name using worktree path and timestamp
- [x] 4.4 Add error handling for tmux session creation failures
- [x] 4.5 Implement user notification when tmux creation fails but worktree succeeds
- [x] 4.6 Add manual tmux setup instructions in error notification

## 5. Sidebar Refresh and State Management

- [x] 5.1 Add function to re-fetch worktree list from repository
- [x] 5.2 Call `set_worktrees` on Sidebar component with updated list
- [x] 5.3 Implement auto-selection of newly created workspace in Sidebar
- [x] 5.4 Set new workspace status to "Idle" (○) in Sidebar
- [x] 5.5 Trigger Sidebar re-render via `cx.notify()` after worktree update
- [x] 5.6 Update WorkspaceManager tabs to include new workspace

## 6. Error Handling and User Feedback

- [x] 6.1 Implement toast notification system (or verify existing)
- [x] 6.2 Display success toast after complete workflow (branch + worktree + tmux)
- [x] 6.3 Display error toast with git command output on failure
- [x] 6.4 Keep dialog open for user correction when validation fails
- [x] 6.5 Keep dialog open when git command fails with actionable error
- [x] 6.6 Re-enable "+ New Branch" button on completion or error
- [x] 6.7 Implement dialog close prevention during creation in progress
- [x] 6.8 Add "Creating..." text on Create button during command execution

## 7. Cancel and Cleanup

- [x] 7.1 Implement Cancel button click handler to close dialog
- [x] 7.2 Add dialog close on Escape key press
- [x] 7.3 Ensure no git commands are executed when dialog is cancelled
- [x] 7.4 Re-enable "+ New Branch" button on dialog cancel
- [x] 7.5 Clear branch name input state on dialog close
- [x] 7.6 Clear validation errors on dialog close
- [x] 7.7 Implement state reset for next dialog open

## 8. Testing and Validation

- [x] 8.1 Test valid branch name creation (e.g., "feature/test", "fix-bug-123")
- [x] 8.2 Test invalid branch name with spaces - verify error display
- [x] 8.3 Test invalid branch name with special characters - verify error display
- [x] 8.4 Test duplicate branch name - verify duplicate detection and switch option
- [x] 8.5 Test successful branch/worktree creation flow end-to-end
- [x] 8.6 Test git command failure scenario (simulate failure)
- [x] 8.7 Test tmux creation failure scenario
- [x] 8.8 Verify Sidebar refresh shows new workspace
- [x] 8.9 Test concurrent click prevention (button disable during creation)
- [x] 8.10 Test Cancel button and Escape key behavior
- [x] 8.11 Test dialog state persistence during creation in progress