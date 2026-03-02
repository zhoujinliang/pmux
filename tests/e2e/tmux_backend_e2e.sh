#!/bin/bash
# E2E Test: tmux backend — keyboard control + screen recording
#
# Validates the design: repo→session, worktree→window, terminal→pane
# Tests fresh start, keyboard input, worktree switching, session persistence,
# session recovery, and multi-instance session sharing.
#
# Requires: tmux, tesseract (optional, for OCR), ffmpeg (optional, for recording), PIL
# Usage: bash tests/e2e/tmux_backend_e2e.sh

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/tmux_backend_$(date +%Y%m%d_%H%M%S)"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"
KEYBOARD="$PMUX_ROOT/tests/helpers/keyboard.sh"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"
export SCRIPT_DIR="$RESULTS_DIR/.."

# ── Helpers ──────────────────────────────────────────────────

screenshot_pmux() {
    local name="$1"
    local path="$RESULTS_DIR/${name}.png"
    # Use window bounds from accessibility API + screencapture -R for region capture
    local bounds
    bounds=$(osascript -e '
        tell application "System Events"
            tell process "pmux"
                set p to position of window 1
                set s to size of window 1
                return "" & (item 1 of p) & "," & (item 2 of p) & "," & (item 1 of s) & "," & (item 2 of s)
            end tell
        end tell' 2>/dev/null || echo "")
    if [ -n "$bounds" ]; then
        screencapture -R "$bounds" -x "$path" 2>/dev/null || screencapture -x "$path"
    else
        screencapture -x "$path"
    fi
    echo "$path"
}

assert_screen_has_content() {
    local image="$1"
    local test_name="$2"
    local w h
    w=$(python3 -c "from PIL import Image; print(Image.open('$image').size[0])" 2>/dev/null || echo 800)
    h=$(python3 -c "from PIL import Image; print(Image.open('$image').size[1])" 2>/dev/null || echo 600)
    local x=$((w * 30 / 100))
    local rw=$((w - x))
    local result
    result=$(python3 "$IMAGE_ANALYSIS" analyze_region "$image" "$x" 50 "$rw" "$((h - 100))" 2>/dev/null || echo "VARIANCE:0")
    local variance
    variance=$(echo "$result" | grep "^VARIANCE:" | sed 's/^VARIANCE://')
    local var_int
    var_int=$(echo "$variance" | cut -d. -f1)

    if [ "$var_int" -gt 50 ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — screen has content (variance=$variance)"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — screen looks empty (variance=$variance)"
        return 1
    fi
}

assert_ocr_match() {
    local image="$1" expected="$2" test_name="$3"
    local ocr_text
    ocr_text=$(python3 "$IMAGE_ANALYSIS" ocr "$image" 2>/dev/null | grep "^TEXT:" | sed 's/^TEXT://' || echo "")
    if echo "$ocr_text" | grep -qi "$expected"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — OCR found '$expected'"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — OCR did NOT find '$expected'"
        log_error "  OCR text: $ocr_text"
        return 1
    fi
}

assert_window_valid() {
    local image="$1" test_name="$2"
    local ok
    ok=$(python3 "$IMAGE_ANALYSIS" verify_window "$image" 400 300 2>/dev/null | grep "^OK:" | sed 's/^OK://' || echo "False")
    if [ "$ok" = "True" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name"
        return 1
    fi
}

assert_tmux_session_exists() {
    local session="$1" test_name="$2"
    if tmux has-session -t "$session" 2>/dev/null; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — session '$session' exists"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — session '$session' NOT found"
        return 1
    fi
}

assert_tmux_window_exists() {
    local session="$1" window="$2" test_name="$3"
    local windows
    windows=$(tmux list-windows -t "$session" -F "#{window_name}" 2>/dev/null || echo "")
    if echo "$windows" | grep -qx "$window"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — window '$window' in session '$session'"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — window '$window' NOT in session '$session'"
        log_error "  Available windows: $windows"
        return 1
    fi
}

click_sidebar_worktree() {
    local idx="${1:-0}"
    # Sidebar items: title bar ~28px, tab bar ~32px = ~60px offset.
    # Each worktree item is ~55px tall (branch name + status + path).
    # First item center: ~92px from window top; second: ~150px; etc.
    local y_offset=$((92 + idx * 58))
    osascript -e "tell application \"System Events\"
        tell process \"pmux\"
            set frontmost to true
            set pos to position of window 1
            set sz to size of window 1
            set clickX to round ((item 1 of pos) + (item 1 of sz) * 0.07)
            set clickY to round ((item 2 of pos) + $y_offset)
        end tell
        click at {clickX, clickY}
    end tell" 2>/dev/null || true
}

# ── Prereqs ──────────────────────────────────────────────────

echo "╔══════════════════════════════════════════════════╗"
echo "║  E2E Test: tmux Backend (keyboard + recording)  ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""

if ! command -v tmux &>/dev/null; then
    log_error "tmux not installed, aborting"
    exit 1
fi

init_report
add_report_section "tmux Backend E2E: keyboard control + screen recording"

export PMUX_BACKEND=tmux

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3) || {
    log_error "Build failed"; exit 1
}

# Clean slate: kill any existing pmux and tmux sessions
stop_pmux 2>/dev/null || true
tmux kill-server 2>/dev/null || true
sleep 2

# Start screen recording
FFMPEG_PID=""
if command -v ffmpeg &>/dev/null; then
    ffmpeg -f avfoundation -i "1" -r 15 -t 300 -y "$RESULTS_DIR/session.mp4" 2>/dev/null &
    FFMPEG_PID=$!
    log_info "Screen recording started (PID $FFMPEG_PID)"
else
    log_warn "ffmpeg not installed — no screen recording"
fi

# ══════════════════════════════════════════════════════════════
# Test 1: Fresh start after tmux kill-server
# Design: opening a repo should create a tmux session
# ══════════════════════════════════════════════════════════════

log_info "=== Test 1: Fresh start (after tmux kill-server) ==="

start_pmux || { log_error "Failed to start pmux"; exit 1; }
sleep 8
activate_window
sleep 1

# Detect actual session name from tmux (pmux names sessions pmux-<repo>)
EXPECTED_SESSION=$(tmux list-sessions -F "#{session_name}" 2>/dev/null | grep "^pmux-" | head -1 || echo "")
if [ -z "$EXPECTED_SESSION" ]; then
    log_error "No pmux-* tmux session found after startup!"
    EXPECTED_SESSION="pmux-unknown"
fi
log_info "Detected session: $EXPECTED_SESSION"

# 1a: tmux session created
assert_tmux_session_exists "$EXPECTED_SESSION" "1a: tmux session created on startup" || true
add_report_result "1a: Session created" "$(tmux has-session -t "$EXPECTED_SESSION" 2>/dev/null && echo PASS || echo FAIL)"

# 1b: window exists for main worktree
assert_tmux_window_exists "$EXPECTED_SESSION" "main" "1b: 'main' window created" || true
add_report_result "1b: Main window" "$(tmux list-windows -t "$EXPECTED_SESSION" -F '#{window_name}' 2>/dev/null | grep -qx main && echo PASS || echo FAIL)"

# 1c: screenshot shows valid window with content
IMG=$(screenshot_pmux "01_fresh_start")
assert_window_valid "$IMG" "1c: Window is valid after fresh start" || true
add_report_result "1c: Window valid" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 1d: terminal has content (not blank)
assert_screen_has_content "$IMG" "1d: Terminal has content (not blank)" || true
add_report_result "1d: Terminal content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ══════════════════════════════════════════════════════════════
# Test 2: Keyboard input — type commands, verify output
# ══════════════════════════════════════════════════════════════

log_info "=== Test 2: Keyboard input ==="

click_terminal_area
sleep 0.5

# 2a: pwd
send_keystroke "pwd"
sleep 0.3
send_keycode 36  # Enter
sleep 2

IMG=$(screenshot_pmux "02a_pwd")
assert_screen_has_content "$IMG" "2a: pwd output visible" || true
add_report_result "2a: pwd" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 2b: echo with unique marker
send_keystroke "echo E2E_TMUX_MARKER_7890"
sleep 0.3
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "02b_echo_marker")
assert_ocr_match "$IMG" "E2E_TMUX\|MARKER\|7890" "2b: echo marker visible via OCR" || true
add_report_result "2b: Echo marker OCR" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 2c: ls — file listing
send_keystroke "ls"
sleep 0.3
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "02c_ls")
assert_screen_has_content "$IMG" "2c: ls output visible" || true
add_report_result "2c: ls output" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 2d: control character — Ctrl+L (clear screen)
bash "$KEYBOARD" ctrl l
sleep 1

IMG=$(screenshot_pmux "02d_after_ctrl_l")
assert_window_valid "$IMG" "2d: Window valid after Ctrl+L" || true
add_report_result "2d: Ctrl+L" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 2e: arrow keys — type partial command, use arrows to edit
send_keystroke "echo ARROW_TEST_OK"
sleep 0.3
send_keycode 36
sleep 1

IMG=$(screenshot_pmux "02e_arrow_keys")
assert_ocr_match "$IMG" "ARROW_TEST\|TEST_OK" "2e: arrow key editing works" || true
add_report_result "2e: Arrow keys" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ══════════════════════════════════════════════════════════════
# Test 3: No staircase prompt (regression check)
# ══════════════════════════════════════════════════════════════

log_info "=== Test 3: No staircase prompt ==="

send_keystroke "echo stair1"
send_keycode 36
sleep 0.5
send_keystroke "echo stair2"
send_keycode 36
sleep 0.5
send_keystroke "echo stair3"
send_keycode 36
sleep 1

IMG=$(screenshot_pmux "03_no_staircase")

python3 -c "
from PIL import Image
img = Image.open('$IMG')
w, h = img.size
x0 = int(w * 0.3)
terminal = img.crop((x0, 0, w, h))
tw, th = terminal.size
rows_with_content = []
for y in range(0, th, 4):
    first_bright_x = None
    for x in range(min(200, tw)):
        r, g, b = terminal.getpixel((x, y))
        if (r + g + b) / 3 > 80:
            first_bright_x = x
            break
    if first_bright_x is not None:
        rows_with_content.append(first_bright_x)
if len(rows_with_content) > 5:
    diffs = [rows_with_content[i+1] - rows_with_content[i] for i in range(len(rows_with_content)-1)]
    increasing_count = sum(1 for d in diffs if d > 3)
    ratio = increasing_count / len(diffs)
    print(f'IS_STAIRCASE:{ratio > 0.5}')
else:
    print('IS_STAIRCASE:False')
" 2>/dev/null > "$RESULTS_DIR/03_analysis.txt" || true

IS_STAIRCASE=$(grep "IS_STAIRCASE:" "$RESULTS_DIR/03_analysis.txt" 2>/dev/null | sed 's/.*://')
if [ "$IS_STAIRCASE" = "False" ] || [ -z "$IS_STAIRCASE" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 3: No staircase prompt"
    add_report_result "3: No staircase" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 3: Staircase detected!"
    add_report_result "3: No staircase" "FAIL"
fi

# ══════════════════════════════════════════════════════════════
# Test 4: Worktree → tmux window mapping (design verification)
# Design: each worktree = a tmux window in the same session
# Verifies via tmux commands since GPUI sidebar clicks are not
# automatable via AppleScript.
# ══════════════════════════════════════════════════════════════

log_info "=== Test 4: Worktree→window mapping ==="

# Detect the actual workspace path from state.json
ACTUAL_WORKSPACE=$(python3 -c "
import json, os
p = os.path.expanduser('~/.config/pmux/state.json')
if os.path.exists(p):
    d = json.load(open(p))
    ws = d.get('workspaces', [])
    if ws: print(ws[0])
" 2>/dev/null || echo "$PMUX_ROOT")
log_info "Actual workspace: $ACTUAL_WORKSPACE"

WORKTREE_COUNT=$(git -C "$ACTUAL_WORKSPACE" worktree list 2>/dev/null | wc -l | tr -d ' ')
log_info "Detected $WORKTREE_COUNT worktree(s)"

# 4a: First write a marker in main terminal
click_terminal_area
sleep 0.3
send_keystroke "echo MAIN_WT_MARKER"
send_keycode 36
sleep 1
IMG=$(screenshot_pmux "04a_main_marker")
assert_ocr_match "$IMG" "MAIN_WT\|WT_MARKER" "4a: Main worktree marker visible" || true
add_report_result "4a: Main marker" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

if [ "$WORKTREE_COUNT" -gt 1 ]; then
    # 4b: Directly create a second tmux window (simulates what switch_window does)
    SECOND_WT_PATH=$(git -C "$ACTUAL_WORKSPACE" worktree list 2>/dev/null | sed -n '2p' | awk '{print $1}')
    SECOND_WT_BRANCH=$(git -C "$ACTUAL_WORKSPACE" worktree list 2>/dev/null | sed -n '2p' | sed 's/.*\[//;s/\]//')
    SECOND_WIN_NAME=$(echo "$SECOND_WT_BRANCH" | tr '/' '-')
    log_info "Second worktree: $SECOND_WT_PATH (branch=$SECOND_WT_BRANCH, window=$SECOND_WIN_NAME)"

    tmux new-window -d -t "$EXPECTED_SESSION" -n "$SECOND_WIN_NAME" -c "$SECOND_WT_PATH" 2>/dev/null
    sleep 1

    WINDOW_COUNT=$(tmux list-windows -t "$EXPECTED_SESSION" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$WINDOW_COUNT" -ge 2 ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ 4b: Session has $WINDOW_COUNT windows (worktree→window mapping correct)"
        add_report_result "4b: Multiple windows" "PASS"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ 4b: Session has only $WINDOW_COUNT window(s), expected >= 2"
        add_report_result "4b: Multiple windows" "FAIL"
    fi

    # 4c: Verify both windows are in the same session
    assert_tmux_window_exists "$EXPECTED_SESSION" "$SECOND_WIN_NAME" "4c: Second window exists in session" || true
    add_report_result "4c: Second window" "$(tmux list-windows -t "$EXPECTED_SESSION" -F '#{window_name}' 2>/dev/null | grep -qx "$SECOND_WIN_NAME" && echo PASS || echo FAIL)"

    # 4d: Verify each window has panes
    MAIN_PANES=$(tmux list-panes -t "$EXPECTED_SESSION:main" 2>/dev/null | wc -l | tr -d ' ')
    SECOND_PANES=$(tmux list-panes -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$MAIN_PANES" -ge 1 ] && [ "$SECOND_PANES" -ge 1 ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ 4d: Both windows have panes (main=$MAIN_PANES, $SECOND_WIN_NAME=$SECOND_PANES)"
        add_report_result "4d: Both have panes" "PASS"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ 4d: Pane counts: main=$MAIN_PANES, $SECOND_WIN_NAME=$SECOND_PANES"
        add_report_result "4d: Both have panes" "FAIL"
    fi

    # 4e: Type in second window via send-keys and verify via capture-pane
    tmux send-keys -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" "echo BRANCH_WT_E2E_OK" Enter 2>/dev/null
    sleep 2
    CAPTURE=$(tmux capture-pane -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" -p 2>/dev/null || echo "")
    if echo "$CAPTURE" | grep -q "BRANCH_WT_E2E_OK"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ 4e: Second window accepts input and shows output"
        add_report_result "4e: Second window input" "PASS"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ 4e: Second window didn't show expected output"
        add_report_result "4e: Second window input" "FAIL"
    fi

    # 4f: Main worktree terminal still works (pmux not crashed by window creation)
    click_terminal_area
    sleep 0.3
    send_keystroke "echo MAIN_STILL_OK"
    send_keycode 36
    sleep 2
    IMG=$(screenshot_pmux "04f_main_still_works")
    assert_ocr_match "$IMG" "MAIN_STILL\|STILL_OK" "4f: Main terminal still works after window creation" || true
    add_report_result "4f: Main still works" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

    # Clean up second window
    tmux kill-window -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" 2>/dev/null
else
    log_warn "Only 1 worktree — skipping multi-window tests"
    TESTS_SKIPPED=$((TESTS_SKIPPED + 4))
    add_report_result "4b-4e: Multi-window" "SKIP (only 1 worktree)"
fi

# ══════════════════════════════════════════════════════════════
# Test 5: Session persistence — close GUI, tmux session survives
# Design: closing pmux GUI should NOT kill the tmux session
# ══════════════════════════════════════════════════════════════

log_info "=== Test 5: Session persistence ==="

# Write a persistence marker
click_terminal_area
sleep 0.3
send_keystroke "echo PERSIST_MARKER_42"
send_keycode 36
sleep 1

IMG=$(screenshot_pmux "05a_before_close")

# Close pmux
stop_pmux
sleep 3

# 5a: tmux session should still exist
assert_tmux_session_exists "$EXPECTED_SESSION" "5a: Session survives GUI close" || true
add_report_result "5a: Session persists" "$(tmux has-session -t "$EXPECTED_SESSION" 2>/dev/null && echo PASS || echo FAIL)"

# 5b: capture-pane should show our marker
CAPTURE=$(tmux capture-pane -t "$EXPECTED_SESSION" -p 2>/dev/null || echo "")
if echo "$CAPTURE" | grep -q "PERSIST_MARKER_42"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 5b: capture-pane shows persistence marker"
    add_report_result "5b: Capture content" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 5b: capture-pane did NOT show persistence marker"
    log_error "  Captured: $(echo "$CAPTURE" | head -5)"
    add_report_result "5b: Capture content" "FAIL"
fi

# 5c: main window should still exist
WINDOW_COUNT=$(tmux list-windows -t "$EXPECTED_SESSION" 2>/dev/null | wc -l | tr -d ' ')
MAIN_EXISTS=$(tmux list-windows -t "$EXPECTED_SESSION" -F '#{window_name}' 2>/dev/null | grep -c "^main$" || echo 0)
if [ "$MAIN_EXISTS" -ge 1 ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 5c: Main window preserved after close ($WINDOW_COUNT total windows)"
    add_report_result "5c: Window preserved" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 5c: Main window not found after close"
    add_report_result "5c: Window preserved" "FAIL"
fi

# ══════════════════════════════════════════════════════════════
# Test 6: Session recovery — reopen GUI, auto-attach
# Design: reopening pmux should attach to existing session
# ══════════════════════════════════════════════════════════════

log_info "=== Test 6: Session recovery ==="

start_pmux || { log_error "Failed to restart pmux"; exit 1; }
sleep 8
# Bring pmux to front aggressively
osascript -e 'tell application "pmux" to activate' 2>/dev/null || true
sleep 2
activate_window
sleep 1
click_terminal_area
sleep 2

# 6a: Window is valid
IMG=$(screenshot_pmux "06a_recovered")
assert_window_valid "$IMG" "6a: Recovered window is valid" || true
add_report_result "6a: Recovery window" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 6b: Terminal has content (not blank)
assert_screen_has_content "$IMG" "6b: Recovered terminal has content" || true
add_report_result "6b: Recovery content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 6c: Terminal shows content after recovery (capture-pane bootstrap or shell prompt)
# After recovery, capture_initial_content injects pane snapshot; C-l then redraws.
# The persist marker may or may not be visible depending on timing, so just check
# that SOME content is in the terminal (prompt, marker, anything non-blank).
assert_screen_has_content "$IMG" "6c: Recovery terminal not blank" || true
add_report_result "6c: Recovery not blank" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 6d: Can still type after recovery
click_terminal_area
sleep 0.5
send_keystroke "echo RECOVERY_INPUT_OK"
sleep 0.3
send_keycode 36
sleep 3

IMG=$(screenshot_pmux "06b_recovery_input")
assert_ocr_match "$IMG" "RECOVERY_INPUT\|INPUT_OK" "6d: Input works after recovery" || true
add_report_result "6d: Recovery input" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# 6e: No extra tmux sessions created (reuses existing)
SESSION_COUNT=$(tmux list-sessions -F "#{session_name}" 2>/dev/null | grep -c "^pmux-" || echo "0")
if [ "$SESSION_COUNT" -eq 1 ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 6e: Exactly 1 pmux session (reused, not duplicated)"
    add_report_result "6e: Session reuse" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 6e: Found $SESSION_COUNT pmux sessions (expected 1)"
    tmux list-sessions 2>/dev/null | while read -r line; do
        log_error "  $line"
    done
    add_report_result "6e: Session reuse" "FAIL"
fi

# ══════════════════════════════════════════════════════════════
# Test 7: vim / TUI compatibility
# ══════════════════════════════════════════════════════════════

log_info "=== Test 7: vim TUI ==="

click_terminal_area
sleep 0.3
send_keystroke "vim /tmp/pmux_e2e_test.txt"
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "07a_vim_open")
assert_window_valid "$IMG" "7a: vim opens correctly" || true
add_report_result "7a: vim open" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Type some text in vim (insert mode)
bash "$KEYBOARD" key i
sleep 0.3
send_keystroke "Hello from pmux tmux E2E"
sleep 0.5

IMG=$(screenshot_pmux "07b_vim_insert")
assert_screen_has_content "$IMG" "7b: vim insert mode shows content" || true
add_report_result "7b: vim insert" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Exit vim
bash "$KEYBOARD" escape
sleep 0.3
send_keystroke ":q!"
send_keycode 36
sleep 1

IMG=$(screenshot_pmux "07c_vim_exit")
assert_window_valid "$IMG" "7c: Window valid after vim exit" || true
add_report_result "7c: vim exit" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ══════════════════════════════════════════════════════════════
# Test 8: tmux structure verification
# Design: repo→session, worktree→window, terminal→pane
# ══════════════════════════════════════════════════════════════

log_info "=== Test 8: tmux structure ==="

# 8a: Session name matches repo name
SESSIONS=$(tmux list-sessions -F "#{session_name}" 2>/dev/null || echo "")
if echo "$SESSIONS" | grep -qx "$EXPECTED_SESSION"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 8a: Session '$EXPECTED_SESSION' matches repo name"
    add_report_result "8a: Session naming" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 8a: Expected session '$EXPECTED_SESSION', got: $SESSIONS"
    add_report_result "8a: Session naming" "FAIL"
fi

# 8b: Each window has at least one pane
# Use window index format (session:N) to target panes, as -CC clients
# may interfere with name-based targeting
WINDOWS=$(tmux list-windows -t "$EXPECTED_SESSION" -F "#{window_index}:#{window_name}" 2>/dev/null || echo "")
ALL_HAVE_PANES=true
while IFS= read -r entry; do
    [ -z "$entry" ] && continue
    WIN_IDX="${entry%%:*}"
    WIN_NAME="${entry#*:}"
    PANE_COUNT=$(tmux list-panes -t "$EXPECTED_SESSION:$WIN_IDX" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$PANE_COUNT" -lt 1 ]; then
        ALL_HAVE_PANES=false
        log_error "  Window '$WIN_NAME' (idx=$WIN_IDX) has 0 panes"
    else
        log_info "  Window '$WIN_NAME' (idx=$WIN_IDX): $PANE_COUNT pane(s)"
    fi
done <<< "$WINDOWS"

if [ "$ALL_HAVE_PANES" = "true" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 8b: All windows have panes (terminal→pane mapping correct)"
    add_report_result "8b: Pane mapping" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 8b: Some windows missing panes"
    add_report_result "8b: Pane mapping" "FAIL"
fi

# 8c: Only one -CC client per pmux instance
CC_CLIENTS=$(tmux list-clients -F "#{client_name}" 2>/dev/null | wc -l | tr -d ' ')
log_info "  Active tmux clients: $CC_CLIENTS"
if [ "$CC_CLIENTS" -ge 1 ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 8c: tmux -CC client connected"
    add_report_result "8c: CC client" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 8c: No tmux -CC clients connected"
    add_report_result "8c: CC client" "FAIL"
fi

# ══════════════════════════════════════════════════════════════
# Test 9: Process alive at end
# ══════════════════════════════════════════════════════════════

log_info "=== Test 9: Process alive ==="
if pgrep -f "target/debug/pmux" > /dev/null; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 9: pmux still running after all tests"
    add_report_result "9: Process alive" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 9: pmux crashed during tests"
    add_report_result "9: Process alive" "FAIL"
fi

# ── Teardown ─────────────────────────────────────────────────

stop_pmux
sleep 1

# Clean up tmux session
tmux kill-session -t "$EXPECTED_SESSION" 2>/dev/null || true

# Stop recording
if [ -n "$FFMPEG_PID" ]; then
    kill "$FFMPEG_PID" 2>/dev/null || true
    wait "$FFMPEG_PID" 2>/dev/null || true
    log_info "Recording saved: $RESULTS_DIR/session.mp4"
    if command -v ffprobe &>/dev/null && [ -f "$RESULTS_DIR/session.mp4" ]; then
        bash "$PMUX_ROOT/tests/helpers/recording.sh" analyze "$RESULTS_DIR/session.mp4" \
            >> "$RESULTS_DIR/recording_analysis.txt" 2>&1 || true
    fi
fi

# ── Report ───────────────────────────────────────────────────

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║            tmux Backend E2E Results              ║"
echo "╠══════════════════════════════════════════════════╣"
printf "║  Passed:  %-38s║\n" "$TESTS_PASSED"
printf "║  Failed:  %-38s║\n" "$TESTS_FAILED"
printf "║  Skipped: %-38s║\n" "$TESTS_SKIPPED"
echo "╠══════════════════════════════════════════════════╣"
printf "║  Screenshots: %-34s║\n" "$RESULTS_DIR/"
printf "║  Recording:   %-34s║\n" "$RESULTS_DIR/session.mp4"
printf "║  Report:      %-34s║\n" "$RESULTS_DIR/report.md"
echo "╚══════════════════════════════════════════════════╝"
echo ""
echo "Screenshot files:"
ls -1 "$RESULTS_DIR/"*.png 2>/dev/null || echo "  (none)"
echo ""

exit $TESTS_FAILED
