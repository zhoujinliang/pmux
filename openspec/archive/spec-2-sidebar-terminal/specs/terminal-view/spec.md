# TerminalView 组件规格

## ADDED Requirements

### Requirement: Terminal Container

The application must display a terminal view on the right side.

#### Scenario: Layout

- **WHEN** the workspace view is displayed
- **THEN** a terminal view occupies the remaining space (right of sidebar)
- **AND** it has a dark background (#1e1e1e)
- **AND** it fills the available height

#### Scenario: Title Bar (Optional v2)

- **WHEN** viewing the terminal
- **THEN** a title bar shows the current pane name
- **AND** it includes window controls (minimize, maximize, close)

### Requirement: Terminal Content Rendering

The terminal must render tmux pane content accurately.

#### Scenario: Character Display

- **WHEN** content is captured from tmux
- **THEN** characters are rendered in a monospace font
- **AND** each character occupies a grid cell
- **AND** the font size is readable (14px default)

#### Scenario: Color Support

- **GIVEN** the terminal output contains ANSI colors
- **WHEN** rendering the content
- **THEN** foreground colors are displayed correctly
- **AND** background colors are displayed correctly
- **AND** bold/italic attributes are respected

#### Scenario: Scrolling

- **GIVEN** the content exceeds the visible area
- **WHEN** the user scrolls
- **THEN** the content scrolls smoothly
- **AND** scroll position is maintained during updates

### Requirement: Cursor Display

The terminal must show the cursor position.

#### Scenario: Cursor Visibility

- **WHEN** the terminal is active
- **THEN** a cursor is displayed at the current position
- **AND** it blinks (if configured)
- **AND** it uses a contrasting color

#### Scenario: Cursor Styles

- **GIVEN** different cursor modes (block, line, bar)
- **WHEN** rendering
- **THEN** the appropriate cursor style is used
- **AND** it matches the underlying terminal state

### Requirement: Input Handling

User input must be sent to the tmux pane.

#### Scenario: Character Input

- **WHEN** the user types a character
- **THEN** it is sent to the active tmux pane
- **AND** it appears in the terminal output
- **AND** the response is within 100ms

#### Scenario: Special Keys

- **WHEN** the user presses special keys (Enter, Tab, Backspace, Arrow keys)
- **THEN** the corresponding escape sequences are sent
- **AND** they function as expected in the terminal

#### Scenario: Application Shortcuts

- **GIVEN** certain key combinations are application shortcuts
- **WHEN** the user presses such a combination
- **THEN** the application handles it (does not send to terminal)
- **AND** examples: ⌘B (toggle sidebar), ⌘N (new branch)

#### Scenario: Focus Management

- **WHEN** the terminal view has focus
- **THEN** keyboard events go to the terminal
- **AND** the cursor is visible

- **WHEN** the terminal loses focus
- **THEN** keyboard events do not go to the terminal
- **AND** the cursor may be hidden or change style

### Requirement: Content Updates

The terminal must update when the tmux pane changes.

#### Scenario: Polling Mechanism

- **GIVEN** the terminal is displaying a pane
- **WHEN** time passes (50ms interval)
- **THEN** the pane content is checked for changes
- **AND** if changed, the display updates

#### Scenario: Performance Optimization

- **GIVEN** the content hasn't changed
- **WHEN** polling occurs
- **THEN** no re-rendering happens
- **AND** CPU usage remains low

#### Scenario: Active Pane Only

- **GIVEN** multiple panes exist
- **WHEN** polling occurs
- **THEN** only the active/visible pane is polled
- **AND** background panes are not polled

## Technical Specifications

### Dependencies

- `alacritty_terminal` - Terminal emulation and VT parsing
- Custom renderer for GPUI integration

### Rendering Pipeline

```
tmux capture-pane
  → raw text with ANSI codes
  → alacritty_terminal::Term::parse_bytes()
  → grid of cells (char + style)
  → GPUI render (text + rectangles)
```

### Performance Targets

- Poll interval: 50ms
- Render time: <16ms (60fps)
- Memory usage: <100MB for typical usage
- Startup time: <500ms after workspace selection

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Pane closed | Show placeholder, attempt reconnect |
| Tmux disconnected | Show error, offer retry |
| Capture failed | Retry with backoff, show loading state |
| Parse error | Skip invalid sequences, log warning |
