#!/bin/bash
# tests/visual/run_visual_tests.sh
# Visual regression test runner for TerminalElement

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SCREENSHOT_DIR="/tmp/pmux_screenshots"
BASELINE_DIR="$SCRIPT_DIR/baselines"
DIFF_DIR="/tmp/pmux_diffs"
APP_PATH="$PROJECT_ROOT/target/debug/pmux"

echo "=== Visual Regression Tests ==="
echo ""

# Check if app exists
if [ ! -f "$APP_PATH" ]; then
    echo "Building pmux..."
    cd "$PROJECT_ROOT"
    cargo build --bin pmux 2>&1 | tail -5
fi

# Create directories
mkdir -p "$SCREENSHOT_DIR" "$DIFF_DIR" "$BASELINE_DIR"

# Helper functions
take_screenshot() {
    local name=$1
    local delay=${2:-1}
    sleep "$delay"
    screencapture -x "$SCREENSHOT_DIR/${name}.png"
    echo "  Screenshot: $name"
}

compare_screenshot() {
    local name=$1
    local threshold=${2:-100}  # Allow some pixel difference
    
    if [ -f "$BASELINE_DIR/${name}.png" ]; then
        # Compare using ImageMagick or pixelmatch
        if command -v compare &> /dev/null; then
            DIFF_COUNT=$(compare -metric AE "$BASELINE_DIR/${name}.png" "$SCREENSHOT_DIR/${name}.png" "$DIFF_DIR/${name}_diff.png" 2>&1 || true)
            if [ "$DIFF_COUNT" -gt "$threshold" ]; then
                echo "  ❌ FAIL: $name (diff: $DIFF_COUNT pixels)"
                return 1
            else
                echo "  ✓ PASS: $name (diff: $DIFF_COUNT pixels)"
                return 0
            fi
        else
            echo "  ⚠️  SKIP: $name (ImageMagick not installed)"
            return 0
        fi
    else
        echo "  📷 BASELINE: $name (saved for future comparison)"
        cp "$SCREENSHOT_DIR/${name}.png" "$BASELINE_DIR/${name}.png"
        return 0
    fi
}

run_app() {
    open -a "$APP_PATH"
    sleep 2
}

close_app() {
    osascript -e 'tell application "pmux" to quit' 2>/dev/null || true
    sleep 1
}

type_text() {
    local text=$1
    osascript -e "tell application \"System Events\" to keystroke \"$text\""
}

press_enter() {
    osascript -e 'tell application "System Events" to key code 36'
}

# Test: V001 - Basic terminal render
test_basic_render() {
    echo ""
    echo "Test V001: Basic terminal render"
    run_app
    take_screenshot "v001_basic"
    compare_screenshot "v001_basic"
    local result=$?
    close_app
    return $result
}

# Test: V002 - ANSI colors
test_ansi_colors() {
    echo ""
    echo "Test V002: ANSI colors"
    run_app
    sleep 1
    type_text "echo -e \""
    osascript -e 'tell application "System Events" to keystroke "\\" using {control down}'
    type_text "e[31mred"
    osascript -e 'tell application "System Events" to keystroke "\\" using {control down}'
    type_text "e[0m "
    osascript -e 'tell application "System Events" to keystroke "\\" using {control down}'
    type_text "e[32mgreen"
    osascript -e 'tell application "System Events" to keystroke "\\" using {control down}'
    type_text "e[0m "
    osascript -e 'tell application "System Events" to keystroke "\\" using {control down}'
    type_text "e[34mblue"
    osascript -e 'tell application "System Events" to keystroke "\\" using {control down}'
    type_text "e[0m\""
    press_enter
    sleep 1
    take_screenshot "v002_colors"
    compare_screenshot "v002_colors" 200
    local result=$?
    close_app
    return $result
}

# Test: V004 - Cursor position
test_cursor_position() {
    echo ""
    echo "Test V004: Cursor position"
    run_app
    take_screenshot "v004_cursor"
    compare_screenshot "v004_cursor" 50
    local result=$?
    close_app
    return $result
}

# Test: V009 - Wide chars (emoji)
test_wide_chars() {
    echo ""
    echo "Test V009: Wide characters (emoji)"
    run_app
    sleep 1
    type_text "echo \""
    osascript -e 'tell application "System Events" to keystroke "🎉" using {option down}'
    type_text " hello "
    osascript -e 'tell application "System Events" to keystroke "👋" using {option down}'
    type_text "\""
    press_enter
    sleep 1
    take_screenshot "v009_wide"
    compare_screenshot "v009_wide" 200
    local result=$?
    close_app
    return $result
}

# Test: V008 - Resize terminal
test_resize() {
    echo ""
    echo "Test V008: Resize terminal"
    run_app
    
    # Type some content
    sleep 1
    type_text "echo 'This is a test of terminal resize behavior'"
    press_enter
    sleep 1
    
    # Resize window
    osascript -e 'tell application "pmux" to set bounds of front window to {100, 100, 800, 600}'
    sleep 1
    take_screenshot "v008_resize"
    compare_screenshot "v008_resize" 500
    local result=$?
    close_app
    return $result
}

# Run all tests
main() {
    PASSED=0
    FAILED=0
    SKIPPED=0
    
    echo "Running visual tests..."
    echo ""
    
    for test_func in test_basic_render test_ansi_colors test_cursor_position test_wide_chars test_resize; do
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
    echo "Skipped: $SKIPPED"
    echo ""
    
    if [ "$FAILED" -gt 0 ]; then
        echo "Diffs saved to: $DIFF_DIR"
        echo "Screenshots saved to: $SCREENSHOT_DIR"
        exit 1
    fi
    
    exit 0
}

main "$@"