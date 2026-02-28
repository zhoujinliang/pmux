# Design: Shell Integration via OSC 133

## Context

OSC 133 is a de-facto standard for shell-to-terminal communication:
- `OSC 133;A` - Prompt start
- `OSC 133;B` - Prompt end / input start
- `OSC 133;C` - Pre-execution (command about to run)
- `OSC 133;D` - Post-execution (command finished, output complete)

Used by: iTerm2, VS Code terminal, Ghostty, WezTerm, kitty

## Goals

1. Parse OSC 133 sequences from terminal output stream
2. Track shell state machine (prompt → input → running → output)
3. Expose prompt line position for cursor operations
4. Enhance status detection with command lifecycle awareness

## Non-Goals

- Automatic shell configuration (document only, don't auto-modify user shells)
- Non-OSC 133 integration methods (e.g., PS1 parsing)
- Shell history recording or persistence

## Decisions

### Decision 1: State machine in TerminalEngine

**What**: Add shell state tracking to `TerminalEngine`, updated by OSC 133 parsing.

**State machine**:
```
Unknown → PromptStart → InputStart → PreExec → PostExec → PromptStart (loop)
   ↑_________________________________________________________|
```

**Why**: Centralizes shell-aware logic in the terminal layer. Runtime/UI query state via API.

**Storage**:
```rust
struct ShellState {
    phase: ShellPhase,
    prompt_line: Option<usize>,  // grid row of current prompt
    last_command_start: Option<Instant>,
}
```

### Decision 2: OSC parsing via vte dispatcher

**What**: Hook into existing alacritty_terminal VTE processing.

**Why**: alacritty_terminal already parses OSC sequences. We can:
- Use its dispatcher to intercept OSC 133
- Or post-process the grid for OSC markers (simpler, less invasive)

**Selected approach**: Post-process on render loop. Add method to scan recent lines for OSC 133 markers.

**Rationale**: Avoids modifying/alacritty_terminal internals. Markers are rare (only at prompt boundaries), so scanning is cheap.

### Decision 3: Marker storage format

**What**: Store detected markers with grid coordinates.

```rust
struct ShellMarker {
    kind: MarkerKind,  // PromptStart, PromptEnd, PreExec, PostExec
    line: usize,       // grid row (scrollback-aware)
    column: usize,
    timestamp: Instant,
}

enum MarkerKind {
    PromptStart,  // OSC 133;A
    PromptEnd,    // OSC 133;B
    PreExec,      // OSC 133;C
    PostExec,     // OSC 133;D (with optional exit code)
}
```

### Decision 4: API design

**What**: Simple query methods on `TerminalEngine`:

```rust
impl TerminalEngine {
    /// Current shell phase
    pub fn shell_phase(&self) -> ShellPhase;

    /// Line number of current prompt (if known)
    pub fn prompt_line(&self) -> Option<usize>;

    /// All markers in visible range
    pub fn visible_markers(&self) -> Vec<ShellMarker>;
}
```

### Decision 5: Status detector integration

**What**: Enhance `status_detector.rs` to use prompt markers when available.

**Current**: Parse screen text for patterns.
**Enhanced**: If `shell_phase() == PreExec`, command is running → status is Running.

**Fallback**: Text-based detection still works without OSC 133.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Shell not configured for OSC 133 | Graceful fallback to text detection |
| Marker line numbers drift on resize | Update storage on grid resize events |
| Scrollback clears markers | Acceptable - markers are ephemeral session state |
| Multiple prompts on screen | Track most recent, allow querying all |

## User Shell Configuration

Document how users enable OSC 133 in their shells:

**zsh** (with oh-my-zsh):
```zsh
plugins=(... shell-integration)
```

**bash** (manual):
```bash
# Add to .bashrc
PS1='\[\e]133;A\e\\\]'$PS1'\[\e]133;B\e\\\]'
trap 'printf "\e]133;C\e\\"' DEBUG
```

**fish**:
```fish
# Built-in in fish 3.4+
set -g fish_handle_osc133 1
```

## Open Questions

1. Should we support OSC 133's exit code parameter (`OSC 133;D;<exit_code>`)?
2. How to handle subshells / nested prompts?
3. Should markers persist across workspace switches?
