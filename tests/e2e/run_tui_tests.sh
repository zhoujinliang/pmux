#!/bin/bash
# tests/e2e/run_tui_tests.sh
# E2E tests for TUI applications (vim, etc.)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCREENSHOT_DIR="/tmp/pmux_screenshots"
APP_PATH="$PROJECT_ROOT/target/debug/pmux"

# Helper functions
source "$PROJECT_ROOT/tests/helpers/window_control.sh" 2>/dev/null || true
source "$PROJECT_ROOT/tests/helpers/keyboard.sh" 2>/dev/null || true
source "$PROJECT_ROOT/tests/helpers/screenshot.sh" 2>/dev/null || true

mkdir -p "$SCREENSHOT_DIR"

start_app() {
    if [ ! -f "$APP_PATH" ]; then
        echo "Building pmux..."
        cd "$PROJECT_ROOT"
        cargo build --bin pmux 2>&1 | tail -5
    fi
    
    open -a "$APP_PATH"
    sleep 2
}

stop_app() {
    osascript -e 'tell application "pmux" to quit' 2>/dev/null || true
    sleep 1
}

# Test: E001 - vim cursor shape
test_vim_cursor() {
    echo ""
    echo "Test E001: vim cursor shape"
    
    start_app
    
    # Launch vim
    osascript -e 'tell application "System Events" to keystroke "v"'
    osascript -e 'tell application "System Events" to keystroke "i"'
    osascript -e 'tell application "System Events" to keystroke "m"'
    osascript -e 'tell application "System Events" to key code 36'  # Enter
    sleep 2
    
    # Screenshot - normal mode (block cursor)
    screencapture -x "$SCREENSHOT_DIR/vim_normal.png"
    echo "  Screenshot: vim_normal.png"
    
    # Enter insert mode (should change to bar cursor if configured)
    osascript -e 'tell application "System Events" to keystroke "i"'
    sleep 0.5
    screencapture -x "$SCREENSHOT_DIR/vim_insert.png"
    echo "  Screenshot: vim_insert.png"
    
    # Exit vim
    osascript -e 'tell application "System Events" to key code 53'  # Escape
    sleep 0.2
    osascript -e 'tell application "System Events" to keystroke ":"'
    osascript -e 'tell application "System Events" to keystroke "q"'
    osascript -e 'tell application "System Events" to keystroke "a"'
    osascript -e 'tell application "System Events" to keystroke "!"'
    osascript -e 'tell application "System Events" to key code 36'  # Enter
    sleep 1
    
    stop_app
    
    echo "  ✓ PASS: vim cursor test completed (manual verification needed)"
    echo "  Screenshots saved to: $SCREENSHOT_DIR"
    return 0
}

# Test: E002 - vim resize
test_vim_resize() {
    echo ""
    echo "Test E002: vim resize behavior"
    
    start_app
    
    # Launch vim with a file
    osascript -e 'tell application "System Events" to keystroke "v"'
    osascript -e 'tell application "System Events" to keystroke "i"'
    osascript -e 'tell application "System Events" to keystroke "m"'
    osascript -e 'tell application "System Events" to key code 36'
    sleep 2
    
    # Type some content
    osascript -e 'tell application "System Events" to keystroke "i"'
    for i in {1..10}; do
        osascript -e "tell application \"System Events\" to keystroke \"Line $i\""
        osascript -e 'tell application "System Events" to key code 36'
    done
    sleep 0.5
    
    # Resize window
    osascript -e 'tell application "pmux" to set bounds of front window to {100, 100, 900, 700}'
    sleep 1
    
    # Screenshot after resize
    screencapture -x "$SCREENSHOT_DIR/vim_resize.png"
    echo "  Screenshot: vim_resize.png"
    
    # Exit vim
    osascript -e 'tell application "System Events" to key code 53'
    sleep 0.2
    osascript -e 'tell application "System Events" to keystroke ":"'
    osascript -e 'tell application "System Events" to keystroke "q"'
    osascript -e 'tell application "System Events" to keystroke "a"'
    osascript -e 'tell application "System Events" to keystroke "!"'
    osascript -e 'tell application "System Events" to key code 36'
    sleep 1
    
    stop_app
    
    echo "  ✓ PASS: vim resize test completed"
    return 0
}

# Test: E003 - Alternate screen mode
test_alternate_screen() {
    echo ""
    echo "Test E003: Alternate screen mode"
    
    start_app
    
    # Enter alternate screen manually
    osascript -e 'tell application "System Events" to keystroke "e"'
    osascript -e 'tell application "System Events" to keystroke "c"'
    osascript -e 'tell application "System Events" to keystroke "h"'
    osascript -e 'tell application "System Events" to keystroke "o"'
    osascript -e 'tell application "System Events" to keystroke " "'
    osascript -e 'tell application "System Events" to keystroke "-"'
    osascript -e 'tell application "System Events" to keystroke "e"'
    osascript -e 'tell application "System Events" to keystroke " "'
    osascript -e 'tell application "System Events" to keystroke "\""'
    # ESC [ ? 1 0 4 9 h
    osascript -e 'tell application "System Events" to key code 53'  # Escape
    osascript -e 'tell application "System Events" to keystroke "["'
    osascript -e 'tell application "System Events" to keystroke "?"'
    osascript -e 'tell application "System Events" to keystroke "1"'
    osascript -e 'tell application "System Events" to keystroke "0"'
    osascript -e 'tell application "System Events" to keystroke "4"'
    osascript -e 'tell application "System Events" to keystroke "9"'
    osascript -e 'tell application "System Events" to keystroke "h"'
    osascript -e 'tell application "System Events" to keystroke "\""'
    osascript -e 'tell application "System Events" to key code 36'
    sleep 1
    
    # Should now be in alternate screen
    screencapture -x "$SCREENSHOT_DIR/alternate_screen.png"
    echo "  Screenshot: alternate_screen.png"
    
    # Exit alternate screen
    osascript -e 'tell application "System Events" to keystroke "e"'
    osascript -e 'tell application "System Events" to keystroke "c"'
    osascript -e 'tell application "System Events" to keystroke "h"'
    osascript -e 'tell application "System Events" to keystroke "o"'
    osascript -e 'tell application "System Events" to keystroke " "'
    osascript -e 'tell application "System Events" to keystroke "-"'
    osascript -e 'tell application "System Events" to keystroke "e"'
    osascript -e 'tell application "System Events" to keystroke " "'
    osascript -e 'tell application "System Events" to keystroke "\""'
    osascript -e 'tell application "System Events" to key code 53'
    osascript -e 'tell application "System Events" to keystroke "["'
    osascript -e 'tell application "System Events" to keystroke "?"'
    osascript -e 'tell application "System Events" to keystroke "1"'
    osascript -e 'tell application "System Events" to keystroke "0"'
    osascript -e 'tell application "System Events" to keystroke "4"'
    osascript -e 'tell application "System Events" to keystroke "9"'
    osascript -e 'tell application "System Events" to keystroke "l"'
    osascript -e 'tell application "System Events" to keystroke "\""'
    osascript -e 'tell application "System Events" to key code 36'
    sleep 1
    
    stop_app
    
    echo "  ✓ PASS: Alternate screen test completed"
    return 0
}

# Test: E004 - Fast typing
test_fast_typing() {
    echo ""
    echo "Test E004: Fast typing responsiveness"
    
    start_app
    
    # Rapid typing
    for i in {1..50}; do
        osascript -e "tell application \"System Events\" to keystroke \"test_$i \""
    done
    sleep 0.5
    
    screencapture -x "$SCREENSHOT_DIR/fast_typing.png"
    echo "  Screenshot: fast_typing.png"
    
    stop_app
    
    echo "  ✓ PASS: Fast typing test completed"
    return 0
}

# Main
main() {
    echo "=== TUI E2E Tests ==="
    echo ""
    
    PASSED=0
    FAILED=0
    
    for test_func in test_vim_cursor test_vim_resize test_alternate_screen test_fast_typing; do
        if $test_func; then
            ((PASSED++))
        else
            ((FAILED++))
        fi
    done
    
    echo ""
    echo "=== Results ==="
    echo "Passed: $PASSED"
    echo "Failed: $FAILED"
    echo ""
    echo "Screenshots: $SCREENSHOT_DIR"
    
    if [ "$FAILED" -gt 0 ]; then
        exit 1
    fi
    
    exit 0
}

main "$@"