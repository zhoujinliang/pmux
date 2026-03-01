# Tmux Backend Tests

Multiple test approaches to verify tmux integration (ref: pytest-tmux, tmux-test, libtmux).

## 1. Shell scripts (original)

```bash
# Direct tmux verification (isolated socket, no pmux)
bash tests/functional/tmux/tmux_direct_verify.sh

# Full pmux + tmux flow (starts GUI, sends keys via tmux)
bash tests/functional/tmux/tmux_ls_verification.sh

# Pipe-pane display check
bash tests/functional/tmux/tmux_pipe_pane_verify.sh
```

## 2. Python/pytest (alternative, isolated)

Uses subprocess + unique `-L` socket for reliability. No GUI focus dependency.

```bash
# Create venv and run (auto-creates .venv-tmux-test if missing)
bash tests/functional/tmux/run_pytest.sh

# Or manually:
python3 -m venv .venv-tmux-test
.venv-tmux-test/bin/pip install -r tests/functional/tmux/requirements.txt
unset TMUX
.venv-tmux-test/bin/python -m pytest tests/functional/tmux/test_tmux_pytest.py -v

# Run only direct tmux test (no pmux startup)
.venv-tmux-test/bin/python -m pytest tests/functional/tmux/test_tmux_pytest.py::TestTmuxDirect -v

# Run only pmux integration test
PMUX_ROOT=$(pwd) .venv-tmux-test/bin/python -m pytest tests/functional/tmux/test_tmux_pytest.py::TestPmuxTmuxBackend -v
```

## 3. Rust integration test

```bash
tmux kill-server
cargo test --test tmux_integration -- --ignored --nocapture
```

## Pre-requisites

- tmux installed
- Run outside tmux (`unset TMUX` if nested)
- For pmux tests: `cargo build` first
