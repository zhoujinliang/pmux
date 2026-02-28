## 1. Core Data Structures

- 1.1 Define `MarkerKind` enum (PromptStart, PromptEnd, PreExec, PostExec)
- 1.2 Define `ShellMarker` struct with line, column, timestamp
- 1.3 Define `ShellPhase` enum (Unknown, Prompt, Input, Running, Output)
- 1.4 Add `shell_state: ShellState` field to `TerminalEngine`

## 2. OSC 133 Parser

- 2.1 Create `Osc133Parser` to identify OSC 133 sequences in byte stream
- 2.2 Extract marker kind from sequence (A/B/C/D)
- 2.3 Parse optional exit code from PostExec sequences
- 2.4 Add unit tests for parser with sample sequences

## 3. TerminalEngine Integration

- 3.1 Add `advance_with_osc133()` method to process bytes and update markers
- 3.2 Update marker line numbers on grid resize/scroll
- 3.3 Expire old markers (keep only last N or within scrollback)
- 3.4 Add `shell_phase()`, `prompt_line()`, `visible_markers()` methods

## 4. Status Detector Enhancement

- 4.1 Add shell phase check to status detection logic
- 4.2 If `PreExec` phase → status Running
- 4.3 If `PostExec` with error code → status Error
- 4.4 Fallback to text detection when OSC 133 unavailable
- 4.5 Add tests for OSC 133 based detection

## 5. Cursor Positioning API (Foundation)

- 5.1 Add `prompt_line()` method returning Option
- 5.2 Add `click_to_prompt(col: usize)` helper for future UI use
- 5.3 Document API for future cursor click feature

## 6. Testing

- 6.1 Test with zsh + shell-integration plugin ✅
- 6.2 Test with bash + manual OSC 133 config ✅
- 6.3 Test with fish (native support) ✅
- 6.4 Test fallback behavior when shell lacks OSC 133 ✅
- 6.5 Test marker tracking during scrollback ✅

## 7. Documentation

- 7.1 Document shell configuration for zsh ✅
- 7.2 Document shell configuration for bash ✅
- 7.3 Document shell configuration for fish ✅
- 7.4 Add troubleshooting guide ✅
- 7.5 Update architecture docs with shell integration layer ✅

