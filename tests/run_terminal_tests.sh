#!/bin/bash
# tests/run_terminal_tests.sh
# Main test runner for TerminalElement tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
SKIP_UNIT=${SKIP_UNIT:-false}
SKIP_INTEGRATION=${SKIP_INTEGRATION:-false}
SKIP_VISUAL=${SKIP_VISUAL:-false}
SKIP_PERFORMANCE=${SKIP_PERFORMANCE:-false}
SKIP_E2E=${SKIP_E2E:-false}

for arg in "$@"; do
    case $arg in
        --skip-unit) SKIP_UNIT=true ;;
        --skip-integration) SKIP_INTEGRATION=true ;;
        --skip-visual) SKIP_VISUAL=true ;;
        --skip-performance) SKIP_PERFORMANCE=true ;;
        --skip-e2e) SKIP_E2E=true ;;
        --quick) 
            SKIP_VISUAL=true
            SKIP_PERFORMANCE=true
            SKIP_E2E=true
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-unit          Skip unit tests"
            echo "  --skip-integration   Skip integration tests"
            echo "  --skip-visual        Skip visual regression tests"
            echo "  --skip-performance   Skip performance tests"
            echo "  --skip-e2e           Skip E2E tests"
            echo "  --quick              Run only unit and integration tests"
            echo "  --help               Show this help"
            exit 0
            ;;
    esac
done

echo -e "${GREEN}=== TerminalElement Test Suite ===${NC}"
echo ""

cd "$PROJECT_ROOT"

TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_SKIPPED=0

# Phase 1: Unit tests
if [ "$SKIP_UNIT" = false ]; then
    echo -e "${YELLOW}Phase 1: Unit Tests...${NC}"
    if cargo test --lib terminal_ -- --nocapture 2>&1; then
        echo -e "${GREEN}✓ Unit tests passed${NC}"
        ((TOTAL_PASSED++))
    else
        echo -e "${RED}✗ Unit tests failed${NC}"
        ((TOTAL_FAILED++))
    fi
    echo ""
else
    echo -e "${YELLOW}Skipping unit tests${NC}"
    ((TOTAL_SKIPPED++))
fi

# Phase 2: Integration tests
if [ "$SKIP_INTEGRATION" = false ]; then
    echo -e "${YELLOW}Phase 2: Integration Tests...${NC}"
    if cargo test --test terminal_ -- --nocapture 2>&1; then
        echo -e "${GREEN}✓ Integration tests passed${NC}"
        ((TOTAL_PASSED++))
    else
        echo -e "${RED}✗ Integration tests failed${NC}"
        ((TOTAL_FAILED++))
    fi
    echo ""
else
    echo -e "${YELLOW}Skipping integration tests${NC}"
    ((TOTAL_SKIPPED++))
fi

# Phase 3: Visual regression tests
if [ "$SKIP_VISUAL" = false ]; then
    echo -e "${YELLOW}Phase 3: Visual Regression Tests...${NC}"
    if [ -f "$SCRIPT_DIR/visual/run_visual_tests.sh" ]; then
        chmod +x "$SCRIPT_DIR/visual/run_visual_tests.sh"
        if "$SCRIPT_DIR/visual/run_visual_tests.sh"; then
            echo -e "${GREEN}✓ Visual tests passed${NC}"
            ((TOTAL_PASSED++))
        else
            echo -e "${RED}✗ Visual tests failed${NC}"
            ((TOTAL_FAILED++))
        fi
    else
        echo -e "${YELLOW}⚠ Visual test script not found${NC}"
        ((TOTAL_SKIPPED++))
    fi
    echo ""
else
    echo -e "${YELLOW}Skipping visual tests${NC}"
    ((TOTAL_SKIPPED++))
fi

# Phase 4: Performance tests
if [ "$SKIP_PERFORMANCE" = false ]; then
    echo -e "${YELLOW}Phase 4: Performance Tests...${NC}"
    if [ -f "$SCRIPT_DIR/performance/run_perf_tests.sh" ]; then
        chmod +x "$SCRIPT_DIR/performance/run_perf_tests.sh"
        if "$SCRIPT_DIR/performance/run_perf_tests.sh"; then
            echo -e "${GREEN}✓ Performance tests passed${NC}"
            ((TOTAL_PASSED++))
        else
            echo -e "${RED}✗ Performance tests failed${NC}"
            ((TOTAL_FAILED++))
        fi
    else
        echo -e "${YELLOW}⚠ Performance test script not found${NC}"
        ((TOTAL_SKIPPED++))
    fi
    echo ""
else
    echo -e "${YELLOW}Skipping performance tests${NC}"
    ((TOTAL_SKIPPED++))
fi

# Phase 5: E2E tests
if [ "$SKIP_E2E" = false ]; then
    echo -e "${YELLOW}Phase 5: E2E Tests...${NC}"
    if [ -f "$SCRIPT_DIR/e2e/run_tui_tests.sh" ]; then
        chmod +x "$SCRIPT_DIR/e2e/run_tui_tests.sh"
        if "$SCRIPT_DIR/e2e/run_tui_tests.sh"; then
            echo -e "${GREEN}✓ E2E tests passed${NC}"
            ((TOTAL_PASSED++))
        else
            echo -e "${RED}✗ E2E tests failed${NC}"
            ((TOTAL_FAILED++))
        fi
    else
        echo -e "${YELLOW}⚠ E2E test script not found${NC}"
        ((TOTAL_SKIPPED++))
    fi
    echo ""
else
    echo -e "${YELLOW}Skipping E2E tests${NC}"
    ((TOTAL_SKIPPED++))
fi

# Summary
echo -e "${GREEN}=== Summary ===${NC}"
echo -e "Passed:  ${GREEN}$TOTAL_PASSED${NC}"
echo -e "Failed:  ${RED}$TOTAL_FAILED${NC}"
echo -e "Skipped: ${YELLOW}$TOTAL_SKIPPED${NC}"
echo ""

if [ "$TOTAL_FAILED" -gt 0 ]; then
    echo -e "${RED}Some tests failed. Please check the output above.${NC}"
    exit 1
fi

echo -e "${GREEN}All tests passed!${NC}"
exit 0