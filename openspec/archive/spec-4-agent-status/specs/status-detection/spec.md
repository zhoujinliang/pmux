# Agent 状态检测规格

## ADDED Requirements

### Requirement: Status Detection from Terminal Output

The system must detect agent status by analyzing tmux pane output.

#### Scenario: Running State Detection

- **GIVEN** the terminal output contains running indicators
- **WHEN** status detection runs
- **THEN** status is set to Running
- **AND** indicators include: "thinking", "writing", "running tool", "executing", "in progress", "loading"

#### Scenario: Waiting State Detection

- **GIVEN** the terminal output shows a prompt waiting for input
- **WHEN** status detection runs
- **THEN** status is set to Waiting
- **AND** prompts include: "? ", "> ", "Human:", "User:", "Awaiting input"

#### Scenario: Error State Detection

- **GIVEN** the terminal output contains error keywords
- **WHEN** status detection runs
- **THEN** status is set to Error
- **AND** error keywords include: "error", "failed", "panic", "traceback", "Exception"

#### Scenario: Idle State Detection

- **GIVEN** no running, waiting, or error indicators are found
- **AND** the terminal has content
- **WHEN** status detection runs
- **THEN** status is set to Idle

#### Scenario: Unknown State

- **GIVEN** the terminal is empty or cannot be read
- **WHEN** status detection runs
- **THEN** status is set to Unknown

### Requirement: Status Priority and Debounce

Status changes must be stable and not flicker.

#### Scenario: Status Stability

- **GIVEN** a new status is detected
- **WHEN** it differs from current status
- **THEN** it must be confirmed for 2 consecutive polls (1 second)
- **AND** only then the status changes
- **EXCEPT** for Error state which changes immediately

#### Scenario: Priority Order

- **GIVEN** multiple indicators could apply
- **WHEN** determining status
- **THEN** priority order is: Error > Waiting > Running > Idle > Unknown

### Requirement: Content Preprocessing

Terminal content must be preprocessed before analysis.

#### Scenario: ANSI Code Stripping

- **GIVEN** terminal content contains ANSI escape codes
- **WHEN** preprocessing runs
- **THEN** all ANSI codes are removed
- **AND** only plain text remains

#### Scenario: Recent Content Focus

- **GIVEN** terminal has long history
- **WHEN** analyzing
- **THEN** only the last 200 lines are considered
- **AND** older content is ignored

## Technical Specifications

### Detection Algorithm

```rust
fn detect_status(content: &str) -> AgentStatus {
    let clean = strip_ansi_codes(content);
    let recent = get_last_n_lines(&clean, 200);
    
    // Check in priority order
    if contains_error_keywords(&recent) {
        return AgentStatus::Error;
    }
    if contains_waiting_prompts(&recent) {
        return AgentStatus::Waiting;
    }
    if contains_running_indicators(&recent) {
        return AgentStatus::Running;
    }
    if !recent.trim().is_empty() {
        return AgentStatus::Idle;
    }
    AgentStatus::Unknown
}
```

### Keywords Lists

**Running Indicators:**
- "thinking"
- "writing"
- "running tool"
- "executing"
- "in progress"
- "loading"
- "downloading"
- "installing"
- "esc to interrupt"

**Waiting Prompts:**
- "? "
- "> "
- "Human:"
- "User:"
- "Awaiting input"
- "Press enter to continue"
- "human turn"

**Error Keywords:**
- "error"
- "failed"
- "panic"
- "traceback"
- "Exception"
- "stack trace"

### Performance Targets

- Detection time: <10ms per pane
- Memory usage: <1MB for 200 lines
- Accuracy: >95% for common AI agents
