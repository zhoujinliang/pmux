#!/bin/bash
# Paint 性能基准 - 使用 PMUX_PERF_LOG 打点，生成可被 analyze_performance.py 解析的日志

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$SCRIPT_DIR/../regression/lib/test_utils.sh"

PERF_LOG="${PERF_LOG:-/tmp/pmux_paint_bench_$$.log}"
export PMUX_PERF_LOG_FILE="$PERF_LOG"

echo "================================"
echo "Paint Performance Benchmark"
echo "================================"
echo ""
echo "Uses PMUX_PERF_LOG=1 to emit 'paint took Xms' to stderr."
echo "Log file: $PERF_LOG"
echo ""

# 准备 state
mkdir -p "$PMUX_CONFIG_DIR"
ROOT="${PMUX_ROOT:-$PROJECT_ROOT}"
cat > "$PMUX_CONFIG_DIR/state.json" << EOF
{
  "workspaces": ["$ROOT"],
  "active_workspace_index": 0
}
EOF

stop_pmux 2>/dev/null || true
sleep 1

log_info "Step 1: Starting pmux with PMUX_PERF_LOG..."
start_pmux || exit 1
sleep 4
activate_window
sleep 1

log_info "Step 2: Generating load (scroll + typing) for 15 seconds..."
# 触发大量输出
send_keystroke "seq 1 500"
send_keycode 36
sleep 5

# 持续按键以产生渲染
for i in {1..30}; do
    send_keystroke "x"
    sleep 0.2
done

log_info "Step 3: Stopping pmux..."
stop_pmux

if [ ! -s "$PERF_LOG" ]; then
    log_error "No paint log generated. Check PMUX_PERF_LOG_FILE."
    exit 1
fi

PAINT_COUNT=$(grep -c "paint took" "$PERF_LOG" 2>/dev/null || echo 0)
log_info "Captured $PAINT_COUNT paint samples"

echo ""
log_info "Step 4: Running analyze_performance.py..."
echo ""
if python3 "$PROJECT_ROOT/tests/helpers/analyze_performance.py" "$PERF_LOG"; then
    log_info "Paint benchmark PASSED"
    exit 0
else
    log_error "Paint benchmark FAILED"
    exit 1
fi
