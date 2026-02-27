## ADDED Requirements

### Requirement: Keyboard Input Passthrough to Tmux
The system SHALL capture keyboard events from GPUI and forward them to the active tmux pane via the tmux send-keys command.

#### Scenario: Character Input
- **WHEN** user types a printable character key (e.g., 'a', '1', '!')
- **THEN** system forwards the character to tmux via `tmux send-keys`
- **AND** the character appears in the terminal output

#### Scenario: Special Key Input
- **WHEN** user presses Enter key
- **THEN** system forwards "Enter" to tmux via `tmux send-keys`
- **AND** terminal command is executed

#### Scenario: Navigation Key Input
- **WHEN** user presses Up arrow key
- **THEN** system forwards "Up" to tmux via `tmux send-keys`
- **AND** command history is navigated

#### Scenario: Application Shortcut Intercepted
- **WHEN** user presses Cmd+B key combination
- **THEN** system does NOT forward the key to tmux
- **AND** system toggles sidebar visibility

### Requirement: Key Name Mapping
The system SHALL map GPUI key names to tmux-compatible key strings using the key_to_tmux function.

#### Scenario: Enter Key Mapping
- **WHEN** GPUI provides key name "enter"
- **THEN** system maps it to "Enter" for tmux send-keys

#### Scenario: Backspace Key Mapping
- **WHEN** GPUI provides key name "backspace"
- **THEN** system maps it to "BSpace" for tmux send-keys

#### Scenario: Page Navigation Key Mapping
- **WHEN** GPUI provides key name "pageup"
- **THEN** system maps it to "PPage" for tmux send-keys

#### Scenario: Regular Character Passthrough
- **WHEN** GPUI provides any other key name (e.g., "x", "5")
- **THEN** system passes it through unchanged to tmux send-keys

### Requirement: Non-Blocking Input Handling
The system SHALL execute tmux send-keys commands without blocking the UI thread.

#### Scenario: Rapid Keyboard Input
- **WHEN** user types multiple keys in quick succession
- **THEN** system queues and executes send-keys commands without UI lag
- **AND** all characters eventually appear in terminal output

#### Scenario: Tmux Process Slow
- **WHEN** tmux process is slow to respond
- **THEN** system remains responsive to other UI interactions
- **AND** input is still forwarded to tmux

### Requirement: Input Error Handling
The system SHALL log errors from failed tmux send-keys commands without crashing or interrupting the application.

#### Scenario: Tmux Not Running
- **WHEN** tmux is not running and user types a key
- **THEN** system logs the error
- **AND** application continues running normally
- **AND** user can still interact with UI elements

#### Scenario: Invalid Pane Target
- **WHEN** tmux send-keys fails due to invalid pane ID
- **THEN** system logs the error
- **AND** application continues running normally