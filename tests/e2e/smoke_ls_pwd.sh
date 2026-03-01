#!/bin/bash
# Smoke test: ls + pwd with screenshot proof
# Verifies the terminal actually displays command output on screen.
# Assertions use pixel variance (content present) + OCR (text match).
#
# Usage: bash tests/e2e/smoke_ls_pwd.sh

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/smoke_ls_pwd_$(date +%Y%m%d_%H%M%S)"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"
export SCRIPT_DIR="$RESULTS_DIR/.."

screenshot_pmux() {
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

    if [ "$var_int" -gt 100 ]; then
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

# ── Setup ──────────────────────────────────────────────────

echo "========================================"
echo "  Smoke Test: ls + pwd (screenshot)"
echo "========================================"
echo ""

init_report
add_report_section "Smoke Test: ls + pwd with Screenshot Proof"

export PMUX_BACKEND=local
stop_pmux 2>/dev/null || true
sleep 1

start_pmux || { log_error "Failed to start pmux"; exit 1; }
sleep 5
activate_window
sleep 1
click_terminal_area
sleep 1

# ── Test 1: pwd — print working directory ──────────────────

log_info "=== Test 1: pwd ==="
send_keystroke "pwd"
sleep 0.3
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "01_pwd_output")
log_info "Screenshot: $IMG"
assert_screen_has_content "$IMG" "pwd shows directory path on screen"
add_report_result "pwd - screen has content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# OCR should find a path (/ or Users or home)
assert_ocr_match "$IMG" "/\|Users\|home\|workspace" "pwd OCR shows a directory path" || true
add_report_result "pwd - OCR path" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 2: ls — list files ────────────────────────────────

log_info "=== Test 2: ls ==="
send_keystroke "ls"
sleep 0.3
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "02_ls_output")
log_info "Screenshot: $IMG"
assert_screen_has_content "$IMG" "ls shows file listing on screen"
add_report_result "ls - screen has content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# OCR should find common project files
assert_ocr_match "$IMG" "src\|Cargo\|tests\|README\|design" "ls OCR shows project files" || true
add_report_result "ls - OCR files" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 3: ls -la — detailed listing ──────────────────────

log_info "=== Test 3: ls -la ==="
send_keystroke "ls -la"
sleep 0.3
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "03_ls_la_output")
log_info "Screenshot: $IMG"
assert_screen_has_content "$IMG" "ls -la shows detailed listing on screen"
add_report_result "ls -la - screen has content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 4: echo with unique marker ───────────────────────

log_info "=== Test 4: echo marker ==="
send_keystroke "echo PMUX_SMOKE_TEST_OK"
sleep 0.3
send_keycode 36
sleep 2

IMG=$(screenshot_pmux "04_echo_marker")
log_info "Screenshot: $IMG"
assert_screen_has_content "$IMG" "echo marker visible on screen"
add_report_result "echo - screen has content" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

assert_ocr_match "$IMG" "PMUX_SMOKE\|SMOKE_TEST\|TEST_OK" "echo OCR finds marker" || true
add_report_result "echo - OCR marker" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# ── Test 5: Verify pmux still alive ───────────────────────

log_info "=== Test 5: Process alive ==="
if pgrep -f "target/debug/pmux" > /dev/null; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ pmux still running after all commands"
    add_report_result "pmux alive" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ pmux crashed"
    add_report_result "pmux alive" "FAIL"
fi

# ── Teardown ──────────────────────────────────────────────

stop_pmux

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "========================================"
echo "  Smoke Test Results"
echo "========================================"
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
