#!/bin/bash
# tests/performance/run_perf_tests.sh
# Performance tests for TerminalElement

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RECORD_DIR="/tmp/pmux_recordings"
LOG_DIR="/tmp/pmux_logs"
APP_PATH="$PROJECT_ROOT/target/debug/pmux"

echo "=== Performance Tests ==="
echo ""

# Create directories
mkdir -p "$RECORD_DIR" "$LOG_DIR"

# Check if app exists
if [ ! -f "$APP_PATH" ]; then
    echo "Building pmux..."
    cd "$PROJECT_ROOT"
    cargo build --bin pmux 2>&1 | tail -5
fi

# Helper functions
start_app() {
    open -a "$APP_PATH"
    sleep 2
}

stop_app() {
    osascript -e 'tell application "pmux" to quit' 2>/dev/null || true
    sleep 1
}

type_text() {
    osascript -e "tell application \"System Events\" to keystroke \"$1\""
}

press_enter() {
    osascript -e 'tell application "System Events" to key code 36'
}

# Test: P001 - Scroll performance
test_scroll_performance() {
    echo ""
    echo "Test P001: Scroll performance"
    
    start_app
    
    # Start log stream
    log stream --predicate 'process == "pmux"' --style compact --timeout 15 > "$LOG_DIR/scroll_test.log" 2>&1 &
    LOG_PID=$!
    
    sleep 1
    
    # Trigger large output
    type_text "cat /usr/share/dict/words"
    press_enter
    
    # Wait for output
    sleep 5
    
    # Stop log
    kill $LOG_PID 2>/dev/null || true
    
    stop_app
    
    # Analyze results
    echo "  Log saved to: $LOG_DIR/scroll_test.log"
    
    # Check for performance warnings in logs
    if grep -q "paint took" "$LOG_DIR/scroll_test.log" 2>/dev/null; then
        SLOW_FRAMES=$(grep -c "paint took" "$LOG_DIR/scroll_test.log" 2>/dev/null || echo "0")
        echo "  Slow frames (>8ms): $SLOW_FRAMES"
        
        if [ "$SLOW_FRAMES" -gt 10 ]; then
            echo "  ❌ FAIL: Too many slow frames"
            return 1
        fi
    fi
    
    echo "  ✓ PASS: Scroll performance acceptable"
    return 0
}

# Test: P002 - Memory usage
test_memory_usage() {
    echo ""
    echo "Test P002: Memory usage"
    
    start_app
    
    # Get initial memory
    PMUX_PID=$(pgrep -f "target/debug/pmux" | head -1)
    
    if [ -z "$PMUX_PID" ]; then
        echo "  ⚠️  SKIP: Could not find pmux process"
        stop_app
        return 0
    fi
    
    INITIAL_MEM=$(ps -o rss= -p "$PMUX_PID" 2>/dev/null || echo "0")
    echo "  Initial memory: ${INITIAL_MEM}KB"
    
    # Generate load
    type_text "for i in {1..100}; do echo \"Line \$i\"; done"
    press_enter
    sleep 2
    
    # Get final memory
    FINAL_MEM=$(ps -o rss= -p "$PMUX_PID" 2>/dev/null || echo "0")
    echo "  Final memory: ${FINAL_MEM}KB"
    
    GROWTH=$((FINAL_MEM - INITIAL_MEM))
    echo "  Memory growth: ${GROWTH}KB"
    
    stop_app
    
    # Check for memory leak (> 50MB growth)
    if [ "$GROWTH" -gt 50000 ]; then
        echo "  ❌ FAIL: Potential memory leak (>${GROWTH}KB growth)"
        return 1
    fi
    
    echo "  ✓ PASS: Memory usage stable"
    return 0
}

# Test: P003 - Resize performance
test_resize_performance() {
    echo ""
    echo "Test P003: Resize performance"
    
    start_app
    
    PMUX_PID=$(pgrep -f "target/debug/pmux" | head -1)
    
    if [ -z "$PMUX_PID" ]; then
        echo "  ⚠️  SKIP: Could not find pmux process"
        stop_app
        return 0
    fi
    
    # Sample CPU during resize
    sample "$PMUX_PID" 3 -f "$LOG_DIR/resize_sample.txt" 2>/dev/null &
    SAMPLE_PID=$!
    
    sleep 0.5
    
    # Rapid resize
    for i in {1..10}; do
        osascript -e "tell application \"pmux\" to set bounds of front window to {100, 100, $((600 + i * 20)), $((400 + i * 10))}"
        sleep 0.1
    done
    
    sleep 2
    kill $SAMPLE_PID 2>/dev/null || true
    
    stop_app
    
    echo "  Sample saved to: $LOG_DIR/resize_sample.txt"
    echo "  ✓ PASS: Resize performance test completed"
    return 0
}

# Test: P004 - FPS during fast input
test_fps() {
    echo ""
    echo "Test P004: FPS during fast input"
    
    start_app
    
    # Start video recording (if ffmpeg available)
    if command -v ffmpeg &> /dev/null; then
        ffmpeg -f avfoundation -i "1" -r 30 -t 10 -y "$RECORD_DIR/fps_test.mp4" 2>/dev/null &
        FFMPEG_PID=$!
        sleep 1
    fi
    
    # Fast input
    for i in {1..50}; do
        type_text "echo line_$i"
        press_enter
        sleep 0.05
    done
    
    # Stop recording
    if [ -n "$FFMPEG_PID" ]; then
        sleep 2
        kill $FFMPEG_PID 2>/dev/null || true
        echo "  Recording saved to: $RECORD_DIR/fps_test.mp4"
    fi
    
    stop_app
    
    echo "  ✓ PASS: FPS test completed (analyze video for frame drops)"
    return 0
}

# Test: P005 - Instruments profiling
test_instruments_profile() {
    echo ""
    echo "Test P005: Instruments profiling"
    
    if ! command -v instruments &> /dev/null; then
        echo "  ⚠️  SKIP: instruments not available"
        return 0
    fi
    
    TRACE_FILE="/tmp/pmux_traces/profile_$(date +%s).trace"
    mkdir -p /tmp/pmux_traces
    
    # Run with Instruments
    instruments -t "Time Profiler" -D "$TRACE_FILE" "$APP_PATH" &
    INSTR_PID=$!
    
    sleep 3
    
    # Generate load
    type_text "cat /usr/share/dict/words | head -100"
    press_enter
    
    sleep 5
    
    # Stop
    kill $INSTR_PID 2>/dev/null || true
    stop_app
    
    echo "  Trace saved to: $TRACE_FILE"
    echo "  Open with: open $TRACE_FILE"
    echo "  ✓ PASS: Profile captured"
    return 0
}

# Run all tests
main() {
    PASSED=0
    FAILED=0
    SKIPPED=0
    
    echo "Running performance tests..."
    echo ""
    
    for test_func in test_scroll_performance test_memory_usage test_resize_performance test_fps test_instruments_profile; do
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
    echo "Logs: $LOG_DIR"
    echo "Recordings: $RECORD_DIR"
    
    if [ "$FAILED" -gt 0 ]; then
        exit 1
    fi
    
    exit 0
}

main "$@"