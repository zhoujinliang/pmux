## ADDED Requirements

### Requirement: OSC 133 sequence parsing
The system SHALL parse OSC 133 escape sequences from terminal output and identify shell lifecycle markers.

#### Scenario: Parse PromptStart marker
- **GIVEN** terminal output contains `ESC ] 133 ; A ESC \` (OSC 133;A)
- **WHEN** the byte stream is processed
- **THEN** a PromptStart marker is recorded at the current cursor position

#### Scenario: Parse command lifecycle
- **GIVEN** a complete command sequence: OSC 133;A → OSC 133;B → OSC 133;C → output → OSC 133;D
- **WHEN** each marker is parsed
- **THEN** the shell phase transitions: Unknown → Prompt → Input → Running → Output

#### Scenario: Optional exit code
- **GIVEN** an OSC 133;D sequence with exit code parameter (e.g., `OSC 133;D;0` or `OSC 133;D;1`)
- **WHEN** the sequence is parsed
- **THEN** the exit code is extracted and stored with the PostExec marker

### Requirement: Shell phase state tracking
The TerminalEngine SHALL maintain the current shell phase based on parsed OSC 133 markers.

#### Scenario: Phase transitions
- **GIVEN** the shell is in Prompt phase
- **WHEN** OSC 133;B is received
- **THEN** the phase transitions to Input

#### Scenario: Phase query API
- **GIVEN** markers have been parsed
- **WHEN** `shell_phase()` is called
- **THEN** it returns the current ShellPhase variant

#### Scenario: Prompt position tracking
- **GIVEN** OSC 133;A was parsed at grid line 100
- **WHEN** `prompt_line()` is queried
- **THEN** it returns Some(100)

### Requirement: Marker coordinate tracking
The system SHALL track marker positions in grid coordinates, adjusting for scrollback and resize.

#### Scenario: Scrollback adjustment
- **GIVEN** a marker at line 1000 in scrollback
- **WHEN** 500 lines scroll out of buffer
- **THEN** the marker line is adjusted to 500 (or removed if off-buffer)

#### Scenario: Resize handling
- **GIVEN** a terminal with markers stored at various lines
- **WHEN** the terminal is resized
- **THEN** marker line numbers are updated to reflect new grid positions

### Requirement: Status detector integration
The status detector SHALL use shell phase information when available, falling back to text parsing.

#### Scenario: OSC 133 based running detection
- **GIVEN** the shell phase is Running (OSC 133;C received, no OSC 133;D yet)
- **WHEN** status is detected
- **THEN** AgentStatus is Running regardless of screen text content

#### Scenario: Error detection via exit code
- **GIVEN** OSC 133;D with exit code 1 was received
- **WHEN** status is detected
- **THEN** AgentStatus is Error

#### Scenario: Fallback to text detection
- **GIVEN** no OSC 133 markers have been parsed
- **WHEN** status is detected
- **THEN** the detector falls back to text-based pattern matching

### Requirement: Marker expiration
The system SHALL expire old markers to prevent unbounded memory growth.

#### Scenario: Marker retention limit
- **GIVEN** a configured marker retention of 100
- **WHEN** more than 100 markers are recorded
- **THEN** oldest markers are removed (FIFO)

#### Scenario: Scrollback boundary
- **GIVEN** a marker at a line that scrolls off the scrollback buffer
- **WHEN** the line is no longer in the grid
- **THEN** the marker is removed
