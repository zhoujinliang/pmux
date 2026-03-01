"""
Tmux backend tests - Python/pytest alternative to shell scripts.

Ref: pytest-tmux (screen assertions + retry), tmux-test (isolated socket),
     libtmux (TestServer unique socket). Uses subprocess + isolated -L socket
     for reliability without extra deps.
"""
import os
import shutil
import subprocess
import time

import pytest


def _tmux_in_path() -> bool:
    return shutil.which("tmux") is not None


def _tmux(sock: str, *args: str, check: bool = False) -> subprocess.CompletedProcess:
    cmd = ["tmux", "-L", sock] + list(args)
    return subprocess.run(cmd, capture_output=True, text=True, timeout=10, check=check)


def _tmux_ok(sock: str, *args: str) -> bool:
    """Run tmux command, return True if exit 0."""
    r = _tmux(sock, *args)
    return r.returncode == 0


def _capture_pane(sock: str, target: str) -> str:
    r = _tmux(sock, "capture-pane", "-t", target, "-p", check=True)
    return r.stdout


def _send_keys(sock: str, target: str, keys: str, enter: bool = True) -> None:
    args = ["send-keys", "-t", target, keys]
    if enter:
        args.append("Enter")
    _tmux(sock, *args, check=True)


@pytest.mark.skipif(not _tmux_in_path(), reason="tmux not in PATH")
class TestTmuxDirect:
    """Direct tmux verification: send-keys + capture-pane (no pmux)."""

    def test_send_keys_capture_output(self, tmp_path, isolated_tmux_socket):
        """Verify tmux send-keys and capture-pane work (isolated socket)."""
        if os.environ.get("TMUX"):
            pytest.skip("Run outside tmux")
        sock = isolated_tmux_socket
        # Kill any existing server on our socket (ignore "no server" error)
        _tmux(sock, "kill-server")
        time.sleep(1)

        repo = tmp_path / "repo"
        repo.mkdir()
        (repo / "README").write_text("testfile")
        subprocess.run(["git", "init", "-q"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "config", "user.email", "t@t.local"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "config", "user.name", "T"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "add", "README"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "init"], cwd=repo, capture_output=True, check=True)

        session = "pmux-pytest-direct"
        _tmux(sock, "new-session", "-d", "-s", session, "-n", "main", "-c", str(repo), check=True)
        time.sleep(1)

        target = f"{session}:main"
        _send_keys(sock, target, f"cd {repo}")
        time.sleep(0.5)
        _send_keys(sock, target, "pwd")
        time.sleep(0.5)
        _send_keys(sock, target, "ls")
        time.sleep(1)

        captured = _capture_pane(sock, target)
        _tmux(sock, "kill-session", "-t", session)

        assert "pmux-pytest" in captured or str(repo.name) in captured, f"Path not in: {captured[:200]}"
        assert "README" in captured, f"README not in: {captured[:200]}"


class TestPmuxTmuxBackend:
    """pmux + tmux backend integration: start pmux, send-keys to its session, verify."""

    @pytest.fixture(autouse=True)
    def _require_tmux_and_outside(self):
        if os.environ.get("TMUX"):
            pytest.skip("Run outside tmux")
        try:
            subprocess.run(["tmux", "-V"], capture_output=True, check=True, timeout=5)
        except (subprocess.CalledProcessError, FileNotFoundError, subprocess.TimeoutExpired):
            pytest.skip("tmux not available")

    def test_pmux_tmux_session_shows_ls_output(self, tmp_path):
        """Start pmux with tmux backend, send cd/pwd/ls via tmux, assert capture contains path and README."""
        pmux_root = os.environ.get("PMUX_ROOT", os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(__file__)))))
        pmux_bin = os.path.join(pmux_root, "target", "debug", "pmux")
        if not os.path.exists(pmux_bin):
            pytest.skip(f"pmux not built: {pmux_bin}")

        # Session name = pmux-{workspace_path.file_name()}; use dir name that yields known session
        repo_dirname = f"pmux-tmux-verify-{os.getpid()}"
        repo = tmp_path / repo_dirname
        session = f"pmux-{repo_dirname}"  # matches session_name_for_workspace(workspace_path)

        repo.mkdir()
        (repo / "README").write_text("")
        subprocess.run(["git", "init", "-q"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "config", "user.email", "t@t.local"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "config", "user.name", "T"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "add", "README"], cwd=repo, capture_output=True, check=True)
        subprocess.run(["git", "commit", "-q", "-m", "init"], cwd=repo, capture_output=True, check=True)
        # pmux uses dirs::config_dir(): macOS = $HOME/Library/Application Support
        home = tmp_path / "home"
        home.mkdir()
        mac_config = home / "Library" / "Application Support" / "pmux"
        mac_config.mkdir(parents=True)
        (mac_config / "config.json").write_text(
            f'{{"workspace_paths":["{str(repo.resolve())}"],"active_workspace_index":0,"backend":"local"}}'
        )

        env = os.environ.copy()
        env["PMUX_BACKEND"] = "tmux"
        env["HOME"] = str(home)

        # Clean default tmux server, start pmux
        subprocess.run(["tmux", "kill-server"], capture_output=True, timeout=5)
        time.sleep(1)
        proc = subprocess.Popen(
            [pmux_bin],
            cwd=pmux_root,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            time.sleep(8)
            if proc.poll() is not None:
                pytest.fail("pmux exited early")
            # Use default socket (pmux creates session there)
            _send_keys_default(session + ":main", f"cd {repo}")
            time.sleep(1)
            _send_keys_default(session + ":main", "pwd")
            time.sleep(1)
            _send_keys_default(session + ":main", "ls")
            time.sleep(3)

            captured = _capture_pane_default(session + ":main")
            assert "pmux-tmux-verify" in captured or repo.name in captured, f"Path not in: {captured[:300]}"
            assert "README" in captured, f"README not in: {captured[:300]}"
        finally:
            proc.terminate()
            proc.wait(timeout=5)
            subprocess.run(["tmux", "kill-server"], capture_output=True, timeout=5)


def _send_keys_default(target: str, keys: str, enter: bool = True) -> None:
    args = ["tmux", "send-keys", "-t", target, keys]
    if enter:
        args.append("Enter")
    subprocess.run(args, capture_output=True, check=True, timeout=10)


def _capture_pane_default(target: str) -> str:
    r = subprocess.run(["tmux", "capture-pane", "-t", target, "-p"], capture_output=True, text=True, timeout=10, check=True)
    return r.stdout
