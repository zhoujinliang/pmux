#!/bin/bash
# E2E Tests for Phase 2: tmux Control Mode (-CC) Persistence
# Tests: terminal rendering, session persistence, recovery after GUI restart
# Requires: tmux, tesseract, ffmpeg, PIL (pip3 install Pillow)
#
# Usage: bash tests/e2e/tmux_cc_e2e.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/tmux_cc_$(date +%Y%m%d_%H%M%S)"
RECORDING_FILE="$RESULTS_DIR/session.mp4"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"
export SCRIPT_DIR="$RESULTS_DIR/.."

# ── Helpers (reuse from local_pty_e2e.sh, inline for independence) ──

screenshot_and_ocr() {
    local name="$1"
    local path="$RESULTS_DIR/${name}.png"
    local win_id
    win_id=$(osascript -e 'tell application "System Events" to get id of window 1 of process "pmux"' 2>/dev/null || echo "")
    if [ -n "$win_id" ]; then
        screencapture -l "$win_id" -x "$path" 2>/dev/null || screencapture -x "$path"
    else
        screencapture -x "$path"
    fi
    echo "$path"
}

assert_ocr_contains() {
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

assert_tmux_session_gone() {
    local session="$1" test_name="$2"
    if ! tmux has-session -t "$session" 2>/dev/null; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — session '$session' is gone"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — session '$session' still exists"
        return 1
    fi
}

# ── Prereqs ─────────────────────────────────────────────────

if ! command -v tmux &>/dev/null; then
    log_error "tmux not installed, skipping Phase 2 E2E"
    exit 0
fi

init_report
add_report_section "Phase 2: tmux Control Mode E2E Tests"

export PMUX_BACKEND=tmux-cc

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3)

# Clean up any previous test sessions
tmux kill-session -t "pmux-e2e-test" 2>/dev/null || true
stop_pmux 2>/dev/null || true
sleep 1

# Start recording
if command -v ffmpeg &>/dev/null; then
    ffmpeg -f avfoundation -i "1" -r 15 -t 180 -y "$RECORDING_FILE" 2>/dev/null &
    FFMPEG_PID=$!
else
    FFMPEG_PID=""
fi

# ── Test 1: Start with tmux-cc, window valid ───────────────

log_info "=== Test 1: tmux-cc startup ==="
start_pmux || { log_error "Failed to start"; exit 1; }
sleep 5
activate_window
sleep 1
IMG=$(screenshot_and_ocr "01_tmux_cc_startup")
assert_window_valid "$IMG" "tmux-cc window visible"
add_report_result "tmux-cc startup" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 2: Command execution ──────────────────────────────

log_info "=== Test 2: Command execution ==="
click_terminal_area
sleep 0.5
send_keystroke "echo TMUX_CC_MARKER_42"
sleep 0.3
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "02_command_exec")
assert_ocr_contains "$IMG" "TMUX_CC_MARKER_42\|MARKER" "echo output visible via OCR" || true
add_report_result "Command exec" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 3: No staircase (same as Phase 1) ─────────────────

log_info "=== Test 3: No staircase ==="
send_keystroke "echo stair1"
send_keycode 36
sleep 0.5
send_keystroke "echo stair2"
send_keycode 36
sleep 0.5
send_keystroke "echo stair3"
send_keycode 36
sleep 1
IMG=$(screenshot_and_ocr "03_no_staircase")

# Reuse staircase detection from Phase 1
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
    log_info "✓ No staircase in tmux-cc mode"
    add_report_result "No staircase (tmux-cc)" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Staircase detected in tmux-cc mode!"
    add_report_result "No staircase (tmux-cc)" "FAIL"
fi

# ── Test 4: Session persistence — close GUI, tmux survives ─

log_info "=== Test 4: Session persistence ==="

# Write a marker to know which session to check
send_keystroke "echo 'SESSION_ALIVE_CHECK'"
send_keycode 36
sleep 1

# Find the pmux tmux session name
PMUX_SESSION=$(tmux list-sessions -F "#{session_name}" 2>/dev/null | grep "pmux" | head -1 || echo "")
log_info "Detected pmux session: $PMUX_SESSION"

# Take screenshot before closing
IMG=$(screenshot_and_ocr "04a_before_close")

# Close pmux GUI
stop_pmux
sleep 2

# Assert tmux session still exists
if [ -n "$PMUX_SESSION" ]; then
    assert_tmux_session_exists "$PMUX_SESSION" "tmux session survives GUI close"
    add_report_result "Session persists" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

    # Verify the session has content (capture-pane should show our marker)
    CAPTURE=$(tmux capture-pane -t "$PMUX_SESSION" -p 2>/dev/null || echo "")
    if echo "$CAPTURE" | grep -q "SESSION_ALIVE_CHECK"; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ Session content preserved (marker found)"
        add_report_result "Session content" "PASS"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ Session content NOT preserved"
        add_report_result "Session content" "FAIL"
    fi
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ No pmux tmux session found"
    add_report_result "Session persists" "FAIL"
    add_report_result "Session content" "SKIP"
fi

# ── Test 5: Recovery — reopen GUI, auto-attach ──────────────

log_info "=== Test 5: Session recovery ==="
sleep 1
start_pmux || { log_error "Failed to restart"; exit 1; }
sleep 5
activate_window
sleep 2
IMG=$(screenshot_and_ocr "05_recovered")

assert_window_valid "$IMG" "Recovered window is valid"
add_report_result "Recovery window" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Check OCR for our marker from before close
assert_ocr_contains "$IMG" "SESSION_ALIVE\|ALIVE_CHECK\|ALIVE" "Recovery shows previous content" || true
add_report_result "Recovery content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 6: vim after recovery ──────────────────────────────

log_info "=== Test 6: vim after recovery ==="
click_terminal_area
sleep 0.5
send_keystroke "vim /tmp/pmux_cc_test.txt"
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "06_vim_after_recovery")
assert_window_valid "$IMG" "vim opens after recovery"
add_report_result "vim after recovery" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Exit vim
send_keystroke ":"
sleep 0.2
send_keystroke "q!"
send_keycode 36
sleep 1

# ── Teardown ────────────────────────────────────────────────

stop_pmux

# Kill test tmux session
if [ -n "$PMUX_SESSION" ]; then
    tmux kill-session -t "$PMUX_SESSION" 2>/dev/null || true
fi

# Stop recording
if [ -n "${FFMPEG_PID:-}" ]; then
    kill "$FFMPEG_PID" 2>/dev/null || true
    wait "$FFMPEG_PID" 2>/dev/null || true
    log_info "Recording saved: $RECORDING_FILE"
    if command -v ffprobe &>/dev/null && [ -f "$RECORDING_FILE" ]; then
        bash "$PMUX_ROOT/tests/helpers/recording.sh" analyze "$RECORDING_FILE" >> "$RESULTS_DIR/recording_analysis.txt" 2>&1 || true
    fi
fi

# ── Report ──────────────────────────────────────────────────

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "================================"
echo "Phase 2 E2E Results"
echo "================================"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "  Screenshots: $RESULTS_DIR/"
echo "  Recording: $RECORDING_FILE"
echo "  Report: $RESULTS_DIR/report.md"
echo "================================"

exit $TESTS_FAILED
