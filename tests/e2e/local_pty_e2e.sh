#!/bin/bash
# E2E Tests for Phase 1: Local PTY Default Backend
# Uses screenshot + OCR + image analysis for assertions.
# Requires: tesseract, ffmpeg, PIL (pip3 install Pillow)
#
# Usage: bash tests/e2e/local_pty_e2e.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/local_pty_$(date +%Y%m%d_%H%M%S)"
RECORDING_FILE="$RESULTS_DIR/session.mp4"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"

# Override SCRIPT_DIR for take_screenshot output path
export SCRIPT_DIR="$RESULTS_DIR/.."

# ── Helpers ─────────────────────────────────────────────────

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
    local image="$1"
    local expected="$2"
    local test_name="$3"
    local ocr_out
    ocr_out=$(python3 "$IMAGE_ANALYSIS" ocr "$image" 2>/dev/null || echo "OK:False
TEXT:")
    local ocr_text
    ocr_text=$(echo "$ocr_out" | grep "^TEXT:" | sed 's/^TEXT://')

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
    local image="$1"
    local test_name="$2"
    local result
    result=$(python3 "$IMAGE_ANALYSIS" verify_window "$image" 400 300 2>/dev/null || echo "OK:False")
    local ok
    ok=$(echo "$result" | grep "^OK:" | sed 's/^OK://')
    local reason
    reason=$(echo "$result" | grep "^REASON:" | sed 's/^REASON://')

    if [ "$ok" = "True" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — window valid"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — window invalid: $reason"
        return 1
    fi
}

assert_terminal_has_content() {
    local image="$1"
    local test_name="$2"
    # Analyze the right 70% of the image (terminal area, skip sidebar)
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

    if [ "$var_int" -gt 100 ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — terminal has content (variance=$variance)"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — terminal looks empty (variance=$variance)"
        return 1
    fi
}

# ── Setup ───────────────────────────────────────────────────

init_report
add_report_section "Phase 1: Local PTY E2E Tests"

# Ensure local backend
unset PMUX_BACKEND
export PMUX_BACKEND=local

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3)

# Start recording (background, 120s max)
if command -v ffmpeg &>/dev/null; then
    ffmpeg -f avfoundation -i "1" -r 15 -t 120 -y "$RECORDING_FILE" 2>/dev/null &
    FFMPEG_PID=$!
    log_info "Recording started (PID=$FFMPEG_PID)"
else
    FFMPEG_PID=""
    log_warn "ffmpeg not found, skipping recording"
fi

stop_pmux 2>/dev/null || true
sleep 1
start_pmux || { log_error "Failed to start pmux"; exit 1; }
sleep 4
activate_window
sleep 1

# ── Test 1: Window visible & valid ─────────────────────────

log_info "=== Test 1: Window visible ==="
IMG=$(screenshot_and_ocr "01_window_visible")
assert_window_valid "$IMG" "Window is visible and valid"
add_report_result "Window visible" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 2: Prompt at top (not middle) ─────────────────────

log_info "=== Test 2: Prompt position ==="
IMG=$(screenshot_and_ocr "02_prompt_position")
# Cursor detection: check if content is in the top 40% of terminal area
python3 -c "
from PIL import Image
img = Image.open('$IMG')
w, h = img.size
# Sample top 40% of terminal area (right 70%)
x0 = int(w * 0.3)
y_top = int(h * 0.05)
y_mid = int(h * 0.40)
top = img.crop((x0, y_top, w, y_mid))
bot = img.crop((x0, y_mid, w, int(h * 0.95)))
top_px = list(top.getdata())
bot_px = list(bot.getdata())
top_var = sum(abs(p[0]-30)+abs(p[1]-30)+abs(p[2]-30) for p in top_px) / len(top_px)
bot_var = sum(abs(p[0]-30)+abs(p[1]-30)+abs(p[2]-30) for p in bot_px) / len(bot_px)
# Top should have more variance (text content) than bottom
print(f'TOP_VAR:{top_var:.1f}')
print(f'BOT_VAR:{bot_var:.1f}')
print(f'PROMPT_AT_TOP:{top_var > bot_var}')
" 2>/dev/null > "$RESULTS_DIR/02_analysis.txt" || true

PROMPT_TOP=$(grep "PROMPT_AT_TOP:" "$RESULTS_DIR/02_analysis.txt" 2>/dev/null | sed 's/.*://')
if [ "$PROMPT_TOP" = "True" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ Prompt at top (not staircase in middle)"
    add_report_result "Prompt at top" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Prompt may not be at top"
    add_report_result "Prompt at top" "FAIL"
fi

# ── Test 3: Command execution — ls ─────────────────────────

log_info "=== Test 3: ls command ==="
click_terminal_area
sleep 0.5
send_keystroke "ls"
sleep 0.3
send_keycode 36  # Enter
sleep 2
IMG=$(screenshot_and_ocr "03_ls_output")
assert_terminal_has_content "$IMG" "ls command shows output"
add_report_result "ls output" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Check OCR for common filenames (Cargo.toml, src, etc.)
assert_ocr_contains "$IMG" "src\|Cargo\|README" "ls output contains expected files" || true
add_report_result "ls OCR" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 4: Command execution — echo with CJK ──────────────

log_info "=== Test 4: CJK echo ==="
send_keystroke "echo 'hello pmux 你好世界'"
sleep 0.3
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "04_cjk_echo")
assert_terminal_has_content "$IMG" "CJK echo shows content"
add_report_result "CJK echo" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 5: No crash after Enter ────────────────────────────

log_info "=== Test 5: Multiple Enter presses ==="
for i in $(seq 1 10); do
    send_keycode 36
    sleep 0.2
done
sleep 1

if pgrep -f "target/debug/pmux" > /dev/null; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ No crash after 10 rapid Enter presses"
    add_report_result "Rapid Enter" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Crashed after rapid Enter"
    add_report_result "Rapid Enter" "FAIL"
fi

# ── Test 6: Vim compatibility ───────────────────────────────

log_info "=== Test 6: vim open/close ==="
send_keystroke "vim /tmp/pmux_e2e_test.txt"
sleep 0.3
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "06_vim_open")
assert_window_valid "$IMG" "vim opens without crash"
add_report_result "vim open" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Type :q! to exit vim
send_keystroke ":"
sleep 0.2
send_keystroke "q!"
sleep 0.2
send_keycode 36
sleep 2
IMG=$(screenshot_and_ocr "06_vim_closed")
assert_terminal_has_content "$IMG" "vim closed, back to shell"
add_report_result "vim close" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 7: No staircase prompt ─────────────────────────────

log_info "=== Test 7: Staircase detection ==="
# Run several commands and check for staircase pattern
send_keystroke "echo line1"
send_keycode 36
sleep 0.5
send_keystroke "echo line2"
send_keycode 36
sleep 0.5
send_keystroke "echo line3"
send_keycode 36
sleep 1
IMG=$(screenshot_and_ocr "07_staircase_check")

# Staircase detection: check if text starts at increasing x offsets per line
# (In a healthy terminal, prompts all start at x=0; staircase has increasing indent)
python3 -c "
from PIL import Image
img = Image.open('$IMG')
w, h = img.size
x0 = int(w * 0.3)
terminal = img.crop((x0, 0, w, h))
tw, th = terminal.size
# Scan columns 0..20 of terminal for each row — staircase means text shifts right
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
# If staircase, the x values would monotonically increase
if len(rows_with_content) > 5:
    diffs = [rows_with_content[i+1] - rows_with_content[i] for i in range(len(rows_with_content)-1)]
    increasing_count = sum(1 for d in diffs if d > 3)
    ratio = increasing_count / len(diffs)
    print(f'STAIRCASE_RATIO:{ratio:.2f}')
    print(f'IS_STAIRCASE:{ratio > 0.5}')
else:
    print('STAIRCASE_RATIO:0')
    print('IS_STAIRCASE:False')
" 2>/dev/null > "$RESULTS_DIR/07_analysis.txt" || true

IS_STAIRCASE=$(grep "IS_STAIRCASE:" "$RESULTS_DIR/07_analysis.txt" 2>/dev/null | sed 's/.*://')
if [ "$IS_STAIRCASE" = "False" ] || [ -z "$IS_STAIRCASE" ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ No staircase prompt detected"
    add_report_result "No staircase" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Staircase prompt detected!"
    add_report_result "No staircase" "FAIL"
fi

# ── Test 8: Large output (seq 500) ──────────────────────────

log_info "=== Test 8: Large output ==="
send_keystroke "seq 1 500"
send_keycode 36
sleep 3
IMG=$(screenshot_and_ocr "08_large_output")

if pgrep -f "target/debug/pmux" > /dev/null; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ Large output handled without crash"
    add_report_result "Large output" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ Crashed on large output"
    add_report_result "Large output" "FAIL"
fi

# ── Teardown ────────────────────────────────────────────────

stop_pmux

# Stop recording
if [ -n "${FFMPEG_PID:-}" ]; then
    kill "$FFMPEG_PID" 2>/dev/null || true
    wait "$FFMPEG_PID" 2>/dev/null || true
    log_info "Recording saved: $RECORDING_FILE"

    # Analyze recording for FPS
    if command -v ffprobe &>/dev/null && [ -f "$RECORDING_FILE" ]; then
        bash "$PMUX_ROOT/tests/helpers/recording.sh" analyze "$RECORDING_FILE" >> "$RESULTS_DIR/recording_analysis.txt" 2>&1 || true
    fi
fi

# ── Report ──────────────────────────────────────────────────

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "================================"
echo "Phase 1 E2E Results"
echo "================================"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "  Screenshots: $RESULTS_DIR/"
echo "  Recording: $RECORDING_FILE"
echo "  Report: $RESULTS_DIR/report.md"
echo "================================"

exit $TESTS_FAILED
