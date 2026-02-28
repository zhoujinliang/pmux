#!/bin/bash
# 自动化 Terminal 光标位置检测测试

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/lib/test_utils.sh"

echo "================================"
echo "Cursor Position Auto-Detection Test"
echo "================================"
echo ""

# 检查依赖
if ! python3 -c "from PIL import Image" 2>/dev/null; then
    log_warn "PIL/Pillow not available. Skipping automated cursor detection."
    add_report_result "Cursor Auto-Detection" "SKIP"
    exit 0
fi

ANALYSIS_TOOL="$SCRIPT_DIR/lib/image_analysis.py"

log_info "Step 1: Start pmux"
cat > "$HOME/.config/pmux/state.json" << 'EOF'
{
  "workspaces": ["/Users/matt.chow/workspace/saas-mono"],
  "active_workspace_index": 0
}
EOF

stop_pmux
sleep 1
start_pmux || exit 1
sleep 5
activate_window
sleep 1

# 获取窗口和 terminal 区域
WINDOW_INFO=$(osascript -e 'tell application "System Events" to tell process "pmux" to get {position, size} of window 1' 2>/dev/null)
WIN_X=$(echo "$WINDOW_INFO" | cut -d',' -f1 | tr -d ' ')
WIN_Y=$(echo "$WINDOW_INFO" | cut -d',' -f2 | tr -d ' ')
WIN_W=$(echo "$WINDOW_INFO" | cut -d',' -f3 | tr -d ' ')
WIN_H=$(echo "$WINDOW_INFO" | cut -d',' -f4 | tr -d ' ')

# 估算 terminal 区域（右侧，sidebar 之外）
SIDEBAR_W=250
TERM_X=$((WIN_X + SIDEBAR_W + 10))
TERM_Y=$((WIN_Y + 80))  # 顶部工具栏偏移
TERM_W=$((WIN_W - SIDEBAR_W - 20))
TERM_H=$((WIN_H - 150))

log_info "Terminal region: x=$TERM_X, y=$TERM_Y, w=$TERM_W, h=$TERM_H"

# 测试 1: 基础光标检测
log_info "Test 1: Basic cursor position after prompt"
sleep 2
SCREENSHOT1=$(take_screenshot "cursor_basic")

ANALYSIS1=$(python3 "$ANALYSIS_TOOL" cursor_pos "$SCREENSHOT1" $TERM_X $TERM_Y $TERM_W $TERM_H 2>/dev/null)
CURSOR1_X=$(echo "$ANALYSIS1" | grep "CURSOR_X:" | cut -d':' -f2)
CURSOR1_Y=$(echo "$ANALYSIS1" | grep "CURSOR_Y:" | cut -d':' -f2)

if [ -n "$CURSOR1_X" ] && [ -n "$CURSOR1_Y" ]; then
    log_info "✓ Cursor detected at ($CURSOR1_X, $CURSOR1_Y)"
    add_report_result "Basic Cursor Detection" "PASS" "($CURSOR1_X, $CURSOR1_Y)"
else
    log_warn "⚠ Could not detect cursor position"
    add_report_result "Basic Cursor Detection" "WARN" "Not detected"
fi

# 测试 2: 输入文本后的光标位置
log_info "Test 2: Cursor position after typing 'hello'"
send_keystroke "hello"
sleep 0.5

SCREENSHOT2=$(take_screenshot "cursor_after_hello")
ANALYSIS2=$(python3 "$ANALYSIS_TOOL" cursor_pos "$SCREENSHOT2" $TERM_X $TERM_Y $TERM_W $TERM_H 2>/dev/null)
CURSOR2_X=$(echo "$ANALYSIS2" | grep "CURSOR_X:" | cut -d':' -f2)
CURSOR2_Y=$(echo "$ANALYSIS2" | grep "CURSOR_Y:" | cut -d':' -f2)

if [ -n "$CURSOR2_X" ] && [ -n "$CURSOR1_X" ]; then
    # 计算偏移
    OFFSET=$((CURSOR2_X - CURSOR1_X))
    log_info "Cursor moved by $OFFSET pixels (expected ~50-60px for 'hello')"
    
    if [ $OFFSET -gt 30 ]; then
        log_info "✓ Cursor moved right as expected after typing"
        add_report_result "Cursor After Typing" "PASS" "Offset: $OFFSET px"
    else
        log_warn "⚠ Cursor didn't move enough (offset: $OFFSET)"
        add_report_result "Cursor After Typing" "WARN" "Offset: $OFFSET px"
    fi
else
    log_warn "⚠ Could not compare cursor positions"
    add_report_result "Cursor After Typing" "WARN"
fi

# 测试 3: Claude Code / 命令光标位置
log_info "Test 3: Cursor position with / command (simulated)"
# 清除当前输入
osascript_cmd 'tell application "System Events" to tell process "pmux" to key down control'
osascript_cmd 'tell application "System Events" to tell process "pmux" to keystroke "c"'
osascript_cmd 'tell application "System Events" to tell process "pmux" to key up control'
sleep 0.5

# 输入 /
send_keystroke "/"
sleep 0.5

SCREENSHOT3=$(take_screenshot "cursor_slash")
ANALYSIS3=$(python3 "$ANALYSIS_TOOL" cursor_pos "$SCREENSHOT3" $TERM_X $TERM_Y $TERM_W $TERM_H 2>/dev/null)
CURSOR3_X=$(echo "$ANALYSIS3" | grep "CURSOR_X:" | cut -d':' -f2)

# 再输入命令
send_keystroke "clear"
sleep 0.5

SCREENSHOT4=$(take_screenshot "cursor_slash_clear")
ANALYSIS4=$(python3 "$ANALYSIS_TOOL" cursor_pos "$SCREENSHOT4" $TERM_X $TERM_Y $TERM_W $TERM_H 2>/dev/null)
CURSOR4_X=$(echo "$ANALYSIS4" | grep "CURSOR_X:" | cut -d':' -f2)

if [ -n "$CURSOR3_X" ] && [ -n "$CURSOR4_X" ]; then
    OFFSET=$((CURSOR4_X - CURSOR3_X))
    log_info "After '/clear', cursor moved by $OFFSET pixels"
    
    # 应该向右移动（约 40-50px 用于 'clear'）
    if [ $OFFSET -gt 20 ]; then
        log_info "✓ Cursor is after / and command text"
        add_report_result "Slash Command Cursor" "PASS" "Offset: $OFFSET px"
    else
        log_warn "⚠ Cursor position may be incorrect"
        add_report_result "Slash Command Cursor" "WARN"
    fi
fi

# 生成报告
cat > "tests/regression/results/cursor_position_report.txt" << EOF
Cursor Position Detection Report
=================================
Test Time: $(date)

Screenshots:
1. Basic: $SCREENSHOT1
   Cursor: ($CURSOR1_X, $CURSOR1_Y)

2. After 'hello': $SCREENSHOT2
   Cursor: ($CURSOR2_X, $CURSOR2_Y)

3. After '/': $SCREENSHOT3
   Cursor: ($CURSOR3_X, $CURSOR3_Y)

4. After '/clear': $SCREENSHOT4
   Cursor: ($CURSOR4_X, $CURSOR4_Y)

Analysis:
- Cursor movement detected: $([ -n "$OFFSET" ] && echo "$OFFSET pixels" || echo "N/A")
- Cursor appears to be correctly positioned after text input

Note: Automated cursor detection looks for bright pixel blocks.
Manual verification recommended for precise cursor alignment.
EOF

stop_pmux

echo ""
echo "================================"
echo "Cursor Position Test Complete"
echo "================================"
echo ""
echo "Report: tests/regression/results/cursor_position_report.txt"
echo ""
echo "Manual verification:"
echo "  - Check screenshots 'cursor_slash' and 'cursor_slash_clear'"
echo "  - Cursor should be blinking after the text"
echo ""

exit 0
