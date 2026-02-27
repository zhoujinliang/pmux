## 1. Input Handler Implementation

- [x] 1.1 Create `InputHandler` struct in `src/input_handler.rs` with session name field
- [x] 1.2 Implement `InputHandler::send_key` method that executes `tmux send-keys` command
- [x] 1.3 Add error logging to `send_key` method to handle failed commands gracefully
- [x] 1.4 Add unit tests for `InputHandler::send_key` method
- [x] 1.5 Ensure `key_to_tmux` function returns correct tmux key strings

## 2. Keyboard Event Integration in AppRoot

- [x] 2.1 Add `InputHandler` field to `AppRoot` struct
- [x] 2.2 Initialize `InputHandler` in `AppRoot::new` when tmux session starts
- [x] 2.3 Subscribe to keyboard events in AppRoot's render method
- [x] 2.4 Implement keyboard event handler that checks for Cmd+key shortcuts
- [x] 2.5 Forward non-shortcut keys to `InputHandler::send_key`
- [x] 2.6 Test that Cmd+B toggles sidebar without sending to tmux
- [x] 2.7 Test that regular characters are forwarded to tmux

## 3. Workspace Restoration Logic

- [x] 3.1 Add workspace validation check in AppRoot initialization
- [x] 3.2 Call `start_tmux_session` when saved workspace is valid
- [x] 3.3 Handle invalid workspace path by falling back to welcome screen
- [x] 3.4 Clear invalid workspace path from config when restoration fails
- [x] 3.5 Test workspace restoration with valid path
- [x] 3.6 Test fallback to welcome screen with invalid path
- [x] 3.7 Test fallback to welcome screen with missing directory

## 4. Integration Testing

- [x] 4.1 Test keyboard input passthrough with tmux session running
- [x] 4.2 Verify Enter key executes commands in terminal
- [x] 4.3 Verify arrow keys navigate command history
- [x] 4.4 Verify rapid typing does not cause UI lag
- [x] 4.5 Test workspace restoration after saving and restarting application
- [x] 4.6 Test manual workspace selection still works when workspace is restored
- [x] 4.7 Test that app shortcuts (Cmd+B, Cmd+N, Cmd+W) still work correctly