#!/usr/bin/env bash
# Run tmux pytest tests. Ref: pytest-tmux, tmux-test, libtmux patterns.
# Usage: ./run_pytest.sh [pytest args]
# Example: ./run_pytest.sh -k TestPmuxTmuxBackend
set -e
cd "$(dirname "$0")"
SCRIPT_DIR="$(pwd)"
# SCRIPT_DIR=tests/functional/tmux, need 3 levels up for repo root
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Use venv if present, else system python
VENV="$ROOT/.venv-tmux-test"
if [ -d "$VENV" ]; then
    PYTHON="$VENV/bin/python"
else
    python3 -m venv "$VENV"
    "$VENV/bin/pip" install -q -r "$SCRIPT_DIR/requirements.txt"
    PYTHON="$VENV/bin/python"
fi

export PMUX_ROOT="${PMUX_ROOT:-$ROOT}"
(cd "$ROOT" && RUSTUP_TOOLCHAIN=stable cargo build 2>/dev/null) || true

unset TMUX
cd "$ROOT"
exec env PMUX_ROOT="$ROOT" "$PYTHON" -m pytest -v "$@" tests/functional/tmux/test_tmux_pytest.py
