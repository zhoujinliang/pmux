# Change: Add Shell Integration via OSC 133

## Why

Currently pmux cannot reliably detect shell prompt boundaries, which limits:
- Precise cursor positioning (click-to-move to prompt)
- Agent state detection accuracy
- Terminal scroll behavior (scroll-to-bottom on new output)

OSC 133 is the standard terminal escape sequence for shell integration, supported by zsh (with plugin), bash, fish, and modern terminals (iTerm2, VS Code, Ghostty).

## What Changes

- **OSC 133 parser**: Detect `OSC 133;A/B/C/D` sequences marking prompt/command/output boundaries
- **ShellMarker tracking**: Store prompt positions and command lifecycle state
- **Integration APIs**: Expose prompt position for cursor operations and status detection
- **Documentation**: Guide users to enable shell integration in their shells

## Impact

- Affected specs: shell-integration (new)
- Affected code:
  - `src/terminal/` - OSC sequence parsing
  - `src/terminal/engine.rs` - marker state storage
  - `src/status_detector.rs` - can use prompt markers for better accuracy
- No breaking changes - pure addition
- Works best when user's shell has OSC 133 enabled (optional enhancement)
