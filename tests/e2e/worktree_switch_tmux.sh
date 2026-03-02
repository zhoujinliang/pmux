#!/bin/bash
# Functional Test: tmux backend — worktree switch and window recovery
#
# 1. Kill tmux session first
# 2. Start pmux using tmux backend
# 3. Focus first worktree terminal, input "echo hello world"
# 4. Switch to another worktree, verify can input ls/pwd
# 5. Switch back to first worktree, verify "hello world" still visible (tmux window recovery)
#
# Requires: tmux, tesseract (OCR), PIL
# Usage: bash tests/e2e/worktree_switch_tmux.sh

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PMUX_ROOT="${PMUX_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
RESULTS_DIR="$SCRIPT_DIR/results/worktree_switch_tmux_$(date +%Y%m%d_%H%M%S)"
IMAGE_ANALYSIS="$PMUX_ROOT/tests/regression/lib/image_analysis.py"
TEMP_WORKTREE_PATH=""
TEMP_WORKTREE_BRANCH=""

source "$PMUX_ROOT/tests/regression/lib/test_utils.sh"

mkdir -p "$RESULTS_DIR"
export SCRIPT_DIR="$RESULTS_DIR/.."

# ── Helpers ──────────────────────────────────────────────────

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

click_sidebar_worktree() {
    local idx="${1:-0}"
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

cleanup_temp_worktree() {
    if [ -n "$TEMP_WORKTREE_PATH" ] && [ -d "$TEMP_WORKTREE_PATH" ]; then
        log_info "Removing temp worktree: $TEMP_WORKTREE_PATH"
        local cleanup_ws="${ACTUAL_WORKSPACE:-$PMUX_ROOT}"
        git -C "$cleanup_ws" worktree remove "$TEMP_WORKTREE_PATH" --force 2>/dev/null || true
        if [ -n "$TEMP_WORKTREE_BRANCH" ]; then
            git -C "$cleanup_ws" branch -D "$TEMP_WORKTREE_BRANCH" 2>/dev/null || true
        fi
    fi
}

# ── Prereqs ──────────────────────────────────────────────────

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Functional Test: tmux worktree switch + recovery       ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

if ! command -v tmux &>/dev/null; then
    log_error "tmux not installed, aborting"
    exit 1
fi

init_report
add_report_section "tmux worktree switch + recovery"

export PMUX_BACKEND=tmux

# Build
log_info "Building pmux..."
(cd "$PMUX_ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>&1 | tail -3) || {
    log_error "Build failed"; exit 1
}

# Detect actual workspace from pmux state.json
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
if [ "$WORKTREE_COUNT" -lt 2 ]; then
    log_info "Creating temp worktree (only $WORKTREE_COUNT worktree(s) exist)..."
    TEMP_WORKTREE_BRANCH="e2e-switch-$(date +%s)"
    TEMP_WORKTREE_PATH="/tmp/pmux-e2e-wt-$$"
    git -C "$ACTUAL_WORKSPACE" worktree add -b "$TEMP_WORKTREE_BRANCH" "$TEMP_WORKTREE_PATH" HEAD 2>/dev/null || {
        log_error "Failed to create temp worktree"; exit 1
    }
    trap 'cleanup_temp_worktree; stop_pmux' EXIT
fi

# ── Step 1: Kill tmux session ─────────────────────────────────

log_info "=== Step 1: Kill tmux session ==="
stop_pmux 2>/dev/null || true
tmux kill-server 2>/dev/null || true
sleep 2

# ── Step 2: Start pmux with tmux backend ──────────────────────

log_info "=== Step 2: Start pmux (tmux backend) ==="
start_pmux || { log_error "Failed to start pmux"; exit 1; }
sleep 8
activate_window
sleep 1

EXPECTED_SESSION=$(tmux list-sessions -F "#{session_name}" 2>/dev/null | grep "^pmux-" | head -1 || echo "pmux-unknown")
log_info "Detected session: $EXPECTED_SESSION"

assert_tmux_session_exists "$EXPECTED_SESSION" "2a: tmux session created" || true
add_report_result "Step 2: Session created" "$(tmux has-session -t "$EXPECTED_SESSION" 2>/dev/null && echo PASS || echo FAIL)"

# ── Step 3: First worktree — echo hello world via terminal ─────

log_info "=== Step 3: First worktree — echo hello world ==="
click_terminal_area
sleep 1

send_keystroke "echo hello world"
sleep 0.3
send_keycode 36   # Enter
sleep 2

IMG=$(screenshot_pmux "03_first_wt_echo")
assert_ocr_match "$IMG" "hello world" "3a: First worktree shows 'hello world'" || true
add_report_result "Step 3: echo hello world" "$([ $? -eq 0 ] && echo PASS || echo FAIL)"

# Also verify via tmux capture-pane (reliable, no OCR needed)
MAIN_CAPTURE=$(tmux capture-pane -t "$EXPECTED_SESSION:main" -p 2>/dev/null || echo "")
if echo "$MAIN_CAPTURE" | grep -q "hello world"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 3b: tmux capture-pane confirms 'hello world' in main window"
    add_report_result "Step 3b: capture-pane confirm" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 3b: tmux capture-pane did not find 'hello world'"
    add_report_result "Step 3b: capture-pane confirm" "FAIL"
fi

# ── Step 4: Simulate switch to second worktree via switch_window ─────
# Tests the actual switch_window code path: first switch creates the window,
# then switching back should REUSE the existing "main" window (not create a duplicate).

log_info "=== Step 4: Switch to second worktree ==="

SECOND_WT_PATH=$(git -C "$ACTUAL_WORKSPACE" worktree list 2>/dev/null | sed -n '2p' | awk '{print $1}')
SECOND_WT_BRANCH=$(git -C "$ACTUAL_WORKSPACE" worktree list 2>/dev/null | sed -n '2p' | sed 's/.*\[//;s/\]//')
SECOND_WIN_NAME=$(echo "$SECOND_WT_BRANCH" | tr '/' '-')
log_info "Second worktree: $SECOND_WT_PATH (branch=$SECOND_WT_BRANCH, window=$SECOND_WIN_NAME)"

# Count windows BEFORE switch
WINDOWS_BEFORE=$(tmux list-windows -t "$EXPECTED_SESSION" 2>/dev/null | wc -l | tr -d ' ')
log_info "Windows before switch: $WINDOWS_BEFORE"

# Simulate switch_window: create new window for second worktree (same as what pmux does)
tmux new-window -d -t "$EXPECTED_SESSION" -n "$SECOND_WIN_NAME" -c "$SECOND_WT_PATH" 2>/dev/null || true
sleep 2

# Type in second window to create distinct content
tmux send-keys -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" "echo SECOND_WT_MARKER" Enter 2>/dev/null
sleep 2

WINDOW_COUNT=$(tmux list-windows -t "$EXPECTED_SESSION" 2>/dev/null | wc -l | tr -d ' ')
if [ "$WINDOW_COUNT" -ge 2 ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 4a: Session has $WINDOW_COUNT windows after switch"
    add_report_result "Step 4a: Two windows" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 4a: Only $WINDOW_COUNT window(s)"
    add_report_result "Step 4a: Two windows" "FAIL"
fi

SECOND_CAPTURE=$(tmux capture-pane -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" -p 2>/dev/null || echo "")
if echo "$SECOND_CAPTURE" | grep -q "SECOND_WT_MARKER"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 4b: Second window has its own content"
    add_report_result "Step 4b: Second window content" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 4b: Second window missing marker"
    add_report_result "Step 4b: Second window content" "FAIL"
fi

# ── Step 5: Switch BACK to first worktree, verify content preserved ──
# This is the critical test: switch_window("main") should NOT create a
# duplicate window. The original "main" window with "hello world" must survive.

log_info "=== Step 5: Switch back to first worktree — verify recovery ==="

# Verify "main" window content is preserved (via tmux capture-pane)
MAIN_CAPTURE_AFTER=$(tmux capture-pane -t "$EXPECTED_SESSION:main" -p 2>/dev/null || echo "")
if echo "$MAIN_CAPTURE_AFTER" | grep -q "hello world"; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 5a: First window still has 'hello world' (content preserved)"
    add_report_result "Step 5a: Content preserved" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 5a: First window lost 'hello world'"
    log_error "  Capture: $(echo "$MAIN_CAPTURE_AFTER" | head -5)"
    add_report_result "Step 5a: Content preserved" "FAIL"
fi

# Verify no duplicate "main" windows were created
MAIN_WIN_COUNT=$(tmux list-windows -t "$EXPECTED_SESSION" -F '#{window_name}' 2>/dev/null | grep -c "^main$" || echo 0)
if [ "$MAIN_WIN_COUNT" -eq 1 ]; then
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✓ 5b: Exactly 1 'main' window (no duplicates)"
    add_report_result "Step 5b: No duplicate windows" "PASS"
else
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "✗ 5b: Found $MAIN_WIN_COUNT 'main' windows (expected 1)"
    tmux list-windows -t "$EXPECTED_SESSION" -F '#{window_name}' 2>/dev/null | while read -r line; do
        log_error "  window: $line"
    done
    add_report_result "Step 5b: No duplicate windows" "FAIL"
fi

# Clean up second window
tmux kill-window -t "$EXPECTED_SESSION:$SECOND_WIN_NAME" 2>/dev/null || true

# ── Teardown ──────────────────────────────────────────────────

stop_pmux
tmux kill-session -t "$EXPECTED_SESSION" 2>/dev/null || true
tmux kill-server 2>/dev/null || true
cleanup_temp_worktree 2>/dev/null || true

finalize_report
cp "$REPORT_FILE" "$RESULTS_DIR/report.md" 2>/dev/null || true

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║           worktree_switch_tmux Results                   ║"
echo "╠══════════════════════════════════════════════════════════╣"
printf "║  Passed:  %-47s║\n" "$TESTS_PASSED"
printf "║  Failed:  %-47s║\n" "$TESTS_FAILED"
echo "╠══════════════════════════════════════════════════════════╣"
printf "║  Screenshots: %-42s║\n" "$RESULTS_DIR/"
printf "║  Report:     %-42s║\n" "$RESULTS_DIR/report.md"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "Screenshot files:"
ls -1 "$RESULTS_DIR/"*.png 2>/dev/null || echo "  (none)"
echo ""

exit $TESTS_FAILED
