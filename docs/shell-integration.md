# Shell Integration (OSC 133)

pmux uses OSC 133 escape sequences to detect shell prompt boundaries and command lifecycle. This improves agent status detection and enables future features like click-to-prompt.

## Overview

OSC 133 is a de-facto standard for shell-to-terminal communication:

| Sequence   | Marker     | Meaning                          |
|------------|------------|----------------------------------|
| `OSC 133;A`| PromptStart| Just before the shell prompt     |
| `OSC 133;B`| PromptEnd  | After prompt, before user input  |
| `OSC 133;C`| PreExec    | Command about to run             |
| `OSC 133;D`| PostExec   | Command finished (optional exit) |

Supported terminals: iTerm2, VS Code, Ghostty, WezTerm, kitty.

---

## Shell Configuration

### zsh (Task 7.1)

**oh-my-zsh shell-integration plugin**

Add the plugin to your `~/.zshrc`:

```zsh
plugins=(... shell-integration)
```

Then reload:

```bash
source ~/.zshrc
```

**Powerlevel10k**

If using Powerlevel10k, enable shell integration:

```zsh
POWERLEVEL9K_TERM_SHELL_INTEGRATION=true
```

---

### bash (Task 7.2)

**Manual PS1 modifications**

Add to `~/.bashrc`:

```bash
# OSC 133: A before prompt, B after prompt
PS1='\[\e]133;A\e\\]'"$PS1"'\[\e]133;B\e\\]'

# DEBUG trap emits PreExec before each command
trap 'printf "\e]133;C\e\\"' DEBUG

# PROMPT_COMMAND emits PostExec after each command (with exit code)
PROMPT_COMMAND='printf "\e]133;D;%s\e\\" "$?"'
```

**Note:** If you already use `PROMPT_COMMAND`, append the PostExec printf to your existing command.

---

### fish (Task 7.3)

**Native support (fish 3.4+)**

Fish has built-in OSC 133. Enable if needed:

```fish
# In ~/.config/fish/config.fish
set -g fish_handle_osc133 1
```

On many installations, OSC 133 is enabled by default. Check with:

```fish
fish --version
```

---

## Troubleshooting (Task 7.4)

### How to verify OSC 133 is working

1. **Enable shell integration** in your shell (see above).
2. **Run pmux** and open a terminal pane.
3. **Type a command** (e.g. `echo test`) and press Enter.
4. **Check agent status** in the sidebar: it should show Running briefly, then Idle when the command finishes.

### Debug commands

**Inspect raw terminal output for OSC 133 sequences:**

```bash
# Run a command and capture output (OSC sequences are invisible but present)
script -q /tmp/term.log
echo test
exit
xxd /tmp/term.log | grep -A1 "1b5d313333"
```

Look for `1b 5d 31 33 33 3b` (ESC ] 133 ;) in the hex dump.

**Test with a minimal prompt:**

```bash
# bash: minimal PS1 with OSC 133
PS1='\[\e]133;A\e\\]$ \[\e]133;B\e\\]'
```

**fish:**

```fish
# Ensure OSC 133 is on
set -g fish_handle_osc133 1
```

### Common issues

| Issue | Cause | Fix |
|------|-------|-----|
| Status always Unknown | Shell not emitting OSC 133 | Enable shell integration (see config above) |
| Status flickers | Debouncing in progress | Normal; status stabilizes after ~1s |
| Works in iTerm2, not pmux | tmux passthrough | Ensure tmux forwards OSC sequences (tmux 3.3+) |

### Fallback behavior

If your shell does not emit OSC 133, pmux falls back to **text-based detection**: it analyzes terminal content for patterns like "thinking", "?", "error", etc. Status detection still works, but may be less accurate than with OSC 133.

---

## Architecture (Task 7.5)

Data flow:

```
Shell (zsh/bash/fish)
    → emits OSC 133 sequences (A/B/C/D)
    → TerminalEngine.advance_with_osc133()
    → Osc133Parser parses sequences
    → ShellState stores markers + phase
    → StatusDetector.detect_with_shell_phase()
    → AgentStatus (Running/Idle/Error/etc.)
```

See [CLAUDE.md](../CLAUDE.md) for the full architecture including the shell integration layer.
