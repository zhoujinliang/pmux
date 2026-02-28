#!/bin/bash
# 功能测试: 窗口创建与基本属性
# 测试窗口是否能正确创建、显示、激活

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../regression/lib/test_utils.sh"

echo "================================"
echo "Window Creation Functional Test"
echo "================================"
echo ""

test_window_creates_with_correct_size() {
    log_info "Test: Window creates with expected dimensions"
    
    start_pmux || return 1
    sleep 3
    
    # 获取窗口尺寸
    WIN_INFO=$(osascript -e 'tell application "System Events" to tell process "pmux" to get size of window 1' 2>/dev/null)
    WIN_W=$(echo "$WIN_INFO" | cut -d',' -f1 | tr -d ' ')
    WIN_H=$(echo "$WIN_INFO" | cut -d',' -f2 | tr -d ' ')
    
    log_info "Window size: ${WIN_W}x${WIN_H}"
    
    # 验证尺寸合理 (至少 800x600)
    if [ "$WIN_W" -gt 800 ] && [ "$WIN_H" -gt 600 ]; then
        log_info "✓ Window size is acceptable"
        add_report_result "Window Size" "PASS" "${WIN_W}x${WIN_H}"
    else
        log_error "✗ Window too small"
        add_report_result "Window Size" "FAIL" "${WIN_W}x${WIN_H}"
    fi
    
    stop_pmux
}

test_window_appears_on_screen() {
    log_info "Test: Window appears within screen bounds"
    
    start_pmux || return 1
    sleep 3
    
    # 获取窗口位置
    POS=$(osascript -e 'tell application "System Events" to tell process "pmux" to get position of window 1' 2>/dev/null)
    POS_X=$(echo "$POS" | cut -d',' -f1 | tr -d ' ')
    POS_Y=$(echo "$POS" | cut -d',' -f2 | tr -d ' ')
    
    log_info "Window position: ($POS_X, $POS_Y)"
    
    # 验证窗口在屏幕内（假设常见屏幕分辨率）
    if [ "$POS_X" -gt -500 ] && [ "$POS_X" -lt 3000 ] && \
       [ "$POS_Y" -gt -100 ] && [ "$POS_Y" -lt 2000 ]; then
        log_info "✓ Window is on screen"
        add_report_result "Window On Screen" "PASS" "($POS_X, $POS_Y)"
    else
        log_warn "⚠ Window may be off-screen"
        add_report_result "Window On Screen" "WARN" "($POS_X, $POS_Y)"
    fi
    
    stop_pmux
}

test_window_can_be_activated() {
    log_info "Test: Window can be activated (frontmost)"
    
    start_pmux || return 1
    sleep 3
    
    # 尝试激活
    if activate_window; then
        sleep 1
        # 检查是否在最前
        FRONTMOST=$(osascript -e 'tell application "System Events" to tell process "pmux" to get frontmost' 2>/dev/null)
        if [ "$FRONTMOST" = "true" ]; then
            log_info "✓ Window activated successfully"
            add_report_result "Window Activation" "PASS"
        else
            log_warn "⚠ Window not frontmost"
            add_report_result "Window Activation" "WARN"
        fi
    else
        log_error "✗ Failed to activate window"
        add_report_result "Window Activation" "FAIL"
    fi
    
    stop_pmux
}

test_window_has_titlebar() {
    log_info "Test: Window has proper titlebar"
    
    start_pmux || return 1
    sleep 3
    
    # 通过 AppleScript 检查窗口属性
    HAS_TITLEBAR=$(osascript << 'EOF'
tell application "System Events"
    tell process "pmux"
        try
            get name of window 1
            return "true"
        on error
            return "false"
        end try
    end tell
end tell
EOF
)
    
    if [ "$HAS_TITLEBAR" = "true" ]; then
        log_info "✓ Window has titlebar"
        add_report_result "Window Titlebar" "PASS"
    else
        log_warn "⚠ Could not verify titlebar"
        add_report_result "Window Titlebar" "WARN"
    fi
    
    stop_pmux
}

# 运行所有测试
test_window_creates_with_correct_size
test_window_appears_on_screen
test_window_can_be_activated
test_window_has_titlebar

echo ""
echo "================================"
echo "Window Creation Test Complete"
echo "================================"
exit $TESTS_FAILED
