#!/bin/bash
# Regression: After pmux restart (tmux/local), input "echo hello world" and verify terminal prints correctly.
#
# Usage:
#   bash tests/e2e/restart_echo_hello.sh           # default: local backend
#   PMUX_BACKEND=tmux bash tests/e2e/restart_echo_hello.sh
#   PMUX_BACKEND=local bash tests/e2e/restart_echo_hello.sh

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
BACKEND="${PMUX_BACKEND:-local}"
RESULTS_DIR="$SCRIPT_DIR/results/restart_echo_hello_${BACKEND}_$(date +%Y%m%d_%H%M%S)"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"
export SCRIPT_DIR="$RESULTS_DIR/.."

screenshot_pmux() {
    local name="$1"
    local path="$RESULTS_DIR/${name}.png"
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
        log_info "  OCR text: $ocr_text"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — OCR did NOT find '$expected'"
        log_error "  OCR text: $ocr_text"
        return 1
    fi
}

# Assert pattern must NOT appear in output. Catches bugs where the command
# leaks into output (e.g. input "echo hello" incorrectly outputs "echohello").
assert_ocr_excludes() {
    local image="$1" forbidden="$2" test_name="$3"
    local ocr_text
    ocr_text=$(python3 "$IMAGE_ANALYSIS" ocr "$image" 2>/dev/null | grep "^TEXT:" | sed 's/^TEXT://' || echo "")
    if echo "$ocr_text" | grep -qi "$forbidden"; then
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — output must NOT contain '$forbidden' (command leaked into output?)"
        log_error "  OCR text: $ocr_text"
        return 1
    else
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — output correctly excludes '$forbidden'"
        return 0
    fi
}

# Detect duplicate output: assert pattern appears exactly N times (default 1).
assert_ocr_count_exactly() {
    local image="$1" pattern="$2" expected_count="${3:-1}" test_name="$4"
    local ocr_text
    ocr_text=$(python3 "$IMAGE_ANALYSIS" ocr "$image" 2>/dev/null | grep "^TEXT:" | sed 's/^TEXT://' || echo "")
    local count
    count=$(echo "$ocr_text" | grep -oi "$pattern" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$count" -eq "$expected_count" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log_info "✓ $test_name — '$pattern' appears exactly $expected_count time(s)"
        return 0
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log_error "✗ $test_name — '$pattern' appears $count time(s), expected $expected_count (duplicate output?)"
        log_error "  OCR text: $ocr_text"
        return 1
    fi
}

# ── Setup ──────────────────────────────────────────────────

echo "========================================"
echo "  Regression: Restart + echo hello world"
echo "  Backend: $BACKEND"
echo "========================================"
echo ""

init_report
add_report_section "Restart + echo hello world (backend=$BACKEND)"

export PMUX_BACKEND="$BACKEND"

if [ "$BACKEND" = "tmux" ] && ! command -v tmux &>/dev/null; then
    log_error "tmux not installed, aborting"
    exit 1
fi

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3) || {
    log_error "Build failed"; exit 1
}

# Clean slate
stop_pmux 2>/dev/null || true
if [ "$BACKEND" = "tmux" ]; then
    tmux kill-server 2>/dev/null || true
fi
sleep 2

# ── Test: Restart pmux, then echo hello world ───────────────

log_info "=== Test: Restart + echo hello world ==="

# Start pmux (first run)
start_pmux || { log_error "Failed to start pmux"; exit 1; }
sleep 5
activate_window
sleep 1
click_terminal_area
sleep 1

# Simulate restart: stop pmux
log_info "Stopping pmux (simulate restart)..."
stop_pmux
sleep 2

# For tmux: session persists; for local: fresh start
if [ "$BACKEND" = "tmux" ]; then
    log_info "tmux session should persist; restarting pmux to reattach..."
fi

# Restart pmux
start_pmux || { log_error "Failed to restart pmux"; exit 1; }
sleep 6
activate_window
sleep 1
click_terminal_area
sleep 1

# Clear screen first so OCR sees only fresh output
log_info "Clearing screen before typing..."
send_keystroke "clear"
sleep 0.3
send_keycode 36   # Enter
sleep 1

# Type "echo hello world" and Enter
log_info "Typing 'echo hello world'..."
send_keystroke "echo hello world"
sleep 0.3
send_keycode 36   # Enter
sleep 2

# Capture screen and verify
IMG=$(screenshot_pmux "restart_echo_hello_output")
log_info "Screenshot: $IMG"

assert_screen_has_content "$IMG" "Terminal has content after restart" || true
add_report_result "Screen has content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

assert_ocr_match "$IMG" "hello world" "Terminal prints 'hello world' correctly" || true
add_report_result "OCR: hello world" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

assert_ocr_excludes "$IMG" "echohello" "Output is just 'hello world', not 'echohello' (command must not leak)" || true
add_report_result "No command leak" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# OCR may count "hello world" in both the command and the output line;
# accept 1 or 2 occurrences (command + output) as correct.
assert_ocr_count_exactly "$IMG" "hello world" 2 "Command + output both contain 'hello world'" || true
add_report_result "Output count" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Teardown ──────────────────────────────────────────────

stop_pmux
if [ "$BACKEND" = "tmux" ]; then
    tmux kill-server 2>/dev/null || true
fi

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "========================================"
echo "  Restart + echo hello world Results"
echo "========================================"
echo "  Backend: $BACKEND"
echo "  Passed: $TESTS_PASSED"
echo "  Failed: $TESTS_FAILED"
echo "  Screenshots: $RESULTS_DIR/"
echo "  Report: $RESULTS_DIR/report.md"
echo "========================================"
echo ""
echo "Screenshot files:"
ls -1 "$RESULTS_DIR/"*.png 2>/dev/null || echo "  (none)"
echo ""

exit $TESTS_FAILED
