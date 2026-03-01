# conftest.py - pytest fixtures for tmux tests
# Ref: pytest-tmux (isolated socket), tmux-test (Vagrant isolation), libtmux (tmpdir socket)

import os
import subprocess
import time

import pytest


def tmux_available() -> bool:
    try:
        r = subprocess.run(["tmux", "-V"], capture_output=True, timeout=5)
        return r.returncode == 0
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return False


@pytest.fixture(scope="session")
def require_tmux():
    if not tmux_available():
        pytest.skip("tmux not installed or not in PATH")


@pytest.fixture
def not_inside_tmux():
    if os.environ.get("TMUX"):
        pytest.skip("Do not run inside tmux session")


@pytest.fixture
def isolated_tmux_socket(tmp_path):
    """Unique socket name to avoid conflicts with system tmux. Returns -L flag value."""
    return f"pmux-pytest-{os.getpid()}"


def tmux_cmd(socket_name: str, *args: str, check: bool = True, timeout: int = 10) -> subprocess.CompletedProcess:
    """Run tmux with isolated socket."""
    cmd = ["tmux", "-L", socket_name] + list(args)
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, check=check)


def tmux_cmd_allow_fail(socket_name: str, *args: str, timeout: int = 10) -> subprocess.CompletedProcess:
    """Run tmux, return result without raising on non-zero exit."""
    cmd = ["tmux", "-L", socket_name] + list(args)
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
