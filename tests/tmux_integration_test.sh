#!/bin/bash
# Tmux backend integration test - no GUI, uses tmux capture-pane to verify.
# Run: tmux kill-server; bash tests/tmux_integration_test.sh
set -e

cd "$(dirname "$0")/.."
PMUX_ROOT="$(pwd)"

# Kill tmux for clean state
tmux kill-server 2>/dev/null || true
sleep 0.5

# Build
RUSTUP_TOOLCHAIN=stable cargo build --quiet 2>/dev/null

# Create temp repo
TEST_DIR=$(mktemp -d /tmp/pmux-tmux-test-XXXXXX)
trap "rm -rf $TEST_DIR" EXIT
cd "$TEST_DIR"
git init -q
git config user.email "t@t.local"
git config user.name "T"
touch README
git add README
git commit -q -m "init"

SESSION="pmux-tmux-integration-test"
tmux kill-session -t "$SESSION" 2>/dev/null || true

# Run the Rust integration test (creates session, sends input, verifies output)
export PMUX_TMUX_TEST_DIR="$TEST_DIR"
export PMUX_TMUX_TEST_SESSION="$SESSION"
RUSTUP_TOOLCHAIN=stable cargo test tmux_integration -- --ignored --nocapture 2>&1 || {
    echo "FAIL: integration test failed"
    tmux kill-session -t "$SESSION" 2>/dev/null || true
    exit 1
}

tmux kill-session -t "$SESSION" 2>/dev/null || true
echo "PASS: tmux integration test"
