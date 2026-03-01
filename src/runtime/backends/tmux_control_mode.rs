//! tmux control mode (-CC) backend.
//!
//! Uses `tmux -CC attach` for structured I/O instead of pipe-pane + capture-pane.
//! This eliminates the dual-datasource problem: gpui-terminal receives clean bytes
//! from %output events, with no FIFO, no capture injection, no Enter workaround.
//!
//! Protocol events parsed:
//! - %output %pane_id data  — pane output bytes (main data path)
//! - %begin / %end          — command response brackets
//! - %exit                  — session detached/exited
//! - %session-changed       — session switch
//! - %window-add/close      — window lifecycle

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::runtime::agent_runtime::{AgentId, AgentRuntime, PaneId, RuntimeError};

// ── Protocol parser ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ControlModeEvent {
    Output { pane_id: String, data: Vec<u8> },
    BeginEnd { tag: String, response: Vec<u8> },
    Exit,
    SessionChanged { session_id: String, name: String },
    WindowAdd { window_id: String },
    WindowClose { window_id: String },
    LayoutChanged { window_id: String, layout: String },
    Unknown(String),
}

pub struct ControlModeParser {
    line_buf: Vec<u8>,
    in_begin: Option<String>,
    begin_response: Vec<u8>,
}

impl ControlModeParser {
    pub fn new() -> Self {
        Self {
            line_buf: Vec::new(),
            in_begin: None,
            begin_response: Vec::new(),
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<ControlModeEvent> {
        let mut events = Vec::new();
        for &b in bytes {
            if b == b'\n' {
                let line = std::mem::take(&mut self.line_buf);
                if let Some(ev) = self.parse_line(&line) {
                    events.push(ev);
                }
            } else {
                self.line_buf.push(b);
            }
        }
        events
    }

    fn parse_line(&mut self, line: &[u8]) -> Option<ControlModeEvent> {
        let s = String::from_utf8_lossy(line);
        let s = s.trim_end_matches('\r');

        // Inside a begin/end block
        if let Some(ref tag) = self.in_begin.clone() {
            if s.starts_with("%end ") && s.contains(tag.as_str()) {
                let response = std::mem::take(&mut self.begin_response);
                self.in_begin = None;
                return Some(ControlModeEvent::BeginEnd {
                    tag: tag.clone(),
                    response,
                });
            } else {
                self.begin_response.extend_from_slice(line);
                self.begin_response.push(b'\n');
                return None;
            }
        }

        if s.starts_with("%begin ") {
            let tag = s.strip_prefix("%begin ").unwrap_or("").to_string();
            self.in_begin = Some(tag);
            self.begin_response.clear();
            return None;
        }

        if s.starts_with("%output ") {
            let rest = &s["%output ".len()..];
            let space_idx = rest.find(' ')?;
            let pane_id = rest[..space_idx].to_string();
            let data = rest[space_idx + 1..].as_bytes().to_vec();
            return Some(ControlModeEvent::Output { pane_id, data });
        }

        if s == "%exit" {
            return Some(ControlModeEvent::Exit);
        }

        if s.starts_with("%session-changed ") {
            let rest = &s["%session-changed ".len()..];
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            return Some(ControlModeEvent::SessionChanged {
                session_id: parts.first().unwrap_or(&"").to_string(),
                name: parts.get(1).unwrap_or(&"").to_string(),
            });
        }

        if s.starts_with("%window-add ") {
            return Some(ControlModeEvent::WindowAdd {
                window_id: s["%window-add ".len()..].to_string(),
            });
        }

        if s.starts_with("%window-close ") {
            return Some(ControlModeEvent::WindowClose {
                window_id: s["%window-close ".len()..].to_string(),
            });
        }

        if s.starts_with("%layout-change ") {
            let rest = &s["%layout-change ".len()..];
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            return Some(ControlModeEvent::LayoutChanged {
                window_id: parts.first().unwrap_or(&"").to_string(),
                layout: parts.get(1).unwrap_or(&"").to_string(),
            });
        }

        Some(ControlModeEvent::Unknown(s.to_string()))
    }
}

// ── Runtime ───────────────────────────────────────────────────────────────────

pub struct TmuxControlModeRuntime {
    session_name: String,
    window_name: String,
    /// stdin of `tmux -CC attach` — send tmux commands here
    control_stdin: Arc<Mutex<std::process::ChildStdin>>,
    /// Per-pane output channels, fed by the parser thread from %output events
    pane_outputs: Arc<Mutex<HashMap<String, flume::Sender<Vec<u8>>>>>,
    /// Keeps the control mode child alive
    _control_child: Arc<Mutex<Child>>,
}

impl TmuxControlModeRuntime {
    /// Create (or attach to) a tmux session and connect via control mode.
    pub fn new(
        session_name: &str,
        window_name: &str,
        start_dir: Option<&Path>,
    ) -> Result<Self, RuntimeError> {
        // Ensure session exists
        let mut create_args = vec!["new-session", "-d", "-s", session_name, "-n", window_name];
        let dir_owned;
        if let Some(dir) = start_dir.and_then(|p| p.to_str()) {
            dir_owned = dir.to_string();
            create_args.extend(["-c", &dir_owned]);
        }
        // Ignore error — session may already exist
        let _ = Command::new("tmux").args(&create_args).output();

        // Attach in control mode (-CC)
        let mut child = Command::new("tmux")
            .args(["-CC", "attach", "-t", session_name])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| RuntimeError::Backend(format!("tmux -CC spawn failed: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| RuntimeError::Backend("tmux -CC has no stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RuntimeError::Backend("tmux -CC has no stdout".into()))?;

        let pane_outputs: Arc<Mutex<HashMap<String, flume::Sender<Vec<u8>>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Parser thread: read control mode stdout, dispatch %output to per-pane channels
        let outputs_for_thread = pane_outputs.clone();
        thread::spawn(move || {
            let mut parser = ControlModeParser::new();
            let mut reader = BufReader::new(stdout);
            let mut line_buf = Vec::new();
            loop {
                line_buf.clear();
                match reader.read_until(b'\n', &mut line_buf) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        for event in parser.feed(&line_buf) {
                            match event {
                                ControlModeEvent::Output { pane_id, data } => {
                                    if let Ok(map) = outputs_for_thread.lock() {
                                        if let Some(tx) = map.get(&pane_id) {
                                            let _ = tx.send(data);
                                        }
                                    }
                                }
                                ControlModeEvent::Exit => return,
                                _ => {} // ignore other events for now
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            session_name: session_name.to_string(),
            window_name: window_name.to_string(),
            control_stdin: Arc::new(Mutex::new(stdin)),
            pane_outputs,
            _control_child: Arc::new(Mutex::new(child)),
        })
    }

    /// Send a raw tmux command via the control mode stdin channel.
    fn send_command(&self, cmd: &str) -> Result<(), RuntimeError> {
        let mut stdin = self
            .control_stdin
            .lock()
            .map_err(|e| RuntimeError::Backend(format!("lock: {}", e)))?;
        writeln!(stdin, "{}", cmd)
            .map_err(|e| RuntimeError::Backend(format!("write: {}", e)))?;
        stdin
            .flush()
            .map_err(|e| RuntimeError::Backend(format!("flush: {}", e)))
    }
}

impl AgentRuntime for TmuxControlModeRuntime {
    fn backend_type(&self) -> &'static str {
        "tmux-cc"
    }

    fn send_input(&self, pane_id: &PaneId, bytes: &[u8]) -> Result<(), RuntimeError> {
        // Use send-keys -l (literal) to avoid tmux interpreting special sequences
        let text = String::from_utf8_lossy(bytes);
        let escaped = text.replace('\'', "\\'");
        self.send_command(&format!("send-keys -l -t {} '{}'", pane_id, escaped))
    }

    fn send_key(&self, pane_id: &PaneId, key: &str, use_literal: bool) -> Result<(), RuntimeError> {
        if use_literal {
            self.send_command(&format!("send-keys -l -t {} '{}'", pane_id, key))
        } else {
            self.send_command(&format!("send-keys -t {} {}", pane_id, key))
        }
    }

    fn resize(&self, pane_id: &PaneId, cols: u16, rows: u16) -> Result<(), RuntimeError> {
        self.send_command(&format!(
            "resize-pane -t {} -x {} -y {}",
            pane_id, cols, rows
        ))?;
        self.send_command(&format!("refresh-client -C {},{}", cols, rows))
    }

    fn subscribe_output(&self, pane_id: &PaneId) -> Option<flume::Receiver<Vec<u8>>> {
        let (tx, rx) = flume::unbounded();
        if let Ok(mut map) = self.pane_outputs.lock() {
            map.insert(pane_id.clone(), tx);
        }
        Some(rx)
    }

    fn capture_initial_content(&self, _pane_id: &PaneId) -> Option<Vec<u8>> {
        // Control mode replays pane output on attach — no separate capture injection needed
        None
    }

    fn list_panes(&self, _agent_id: &AgentId) -> Vec<PaneId> {
        let target = format!("{}:{}", self.session_name, self.window_name);
        Command::new("tmux")
            .args(["list-panes", "-t", &target, "-F", "#{pane_id}"])
            .output()
            .ok()
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn focus_pane(&self, pane_id: &PaneId) -> Result<(), RuntimeError> {
        self.send_command(&format!("select-pane -t {}", pane_id))
    }

    fn split_pane(&self, pane_id: &PaneId, vertical: bool) -> Result<PaneId, RuntimeError> {
        let flag = if vertical { "-h" } else { "-v" };
        self.send_command(&format!("split-window {} -t {}", flag, pane_id))?;
        // Return last pane in list (the newly created one)
        self.list_panes(&String::new())
            .into_iter()
            .last()
            .ok_or_else(|| RuntimeError::Backend("no pane after split".into()))
    }

    fn get_pane_dimensions(&self, _pane_id: &PaneId) -> (u16, u16) {
        // TODO: query from tmux display -p '#{pane_width} #{pane_height}'
        (80, 24)
    }

    fn open_diff(
        &self,
        worktree: &Path,
        _pane_id: Option<&PaneId>,
    ) -> Result<String, RuntimeError> {
        let primary = self
            .list_panes(&String::new())
            .into_iter()
            .next()
            .unwrap_or_default();
        let new_pane = self.split_pane(&primary, true)?;
        let cmd = format!(
            "nvim -c 'DiffviewOpen main...HEAD' '{}' 2>/dev/null || git diff main...HEAD --color=always | less -R\n",
            worktree.to_string_lossy()
        );
        self.send_input(&new_pane, cmd.as_bytes())?;
        Ok(new_pane)
    }

    fn open_review(&self, worktree: &Path) -> Result<String, RuntimeError> {
        self.open_diff(worktree, None)
    }

    fn kill_window(&self, _window_target: &str) -> Result<(), RuntimeError> {
        self.send_command(&format!(
            "kill-window -t {}:{}",
            self.session_name, self.window_name
        ))
    }

    fn session_info(&self) -> Option<(String, String)> {
        Some((self.session_name.clone(), self.window_name.clone()))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parser tests (no tmux required) ──────────────────────

    #[test]
    fn test_parse_output_event() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%output %0 hello world\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControlModeEvent::Output { pane_id, data } => {
                assert_eq!(pane_id, "%0");
                assert_eq!(data, b"hello world");
            }
            _ => panic!("expected Output event, got {:?}", events[0]),
        }
    }

    #[test]
    fn test_parse_exit_event() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%exit\n");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], ControlModeEvent::Exit));
    }

    #[test]
    fn test_parse_partial_line() {
        let mut parser = ControlModeParser::new();
        let events1 = parser.feed(b"%output %0 hel");
        assert!(events1.is_empty(), "partial line should not emit events");
        let events2 = parser.feed(b"lo\n");
        assert_eq!(events2.len(), 1);
        match &events2[0] {
            ControlModeEvent::Output { pane_id, data } => {
                assert_eq!(pane_id, "%0");
                assert_eq!(data, b"hello");
            }
            _ => panic!("expected Output event"),
        }
    }

    #[test]
    fn test_parse_begin_end() {
        let mut parser = ControlModeParser::new();
        let events =
            parser.feed(b"%begin 1234567890 1 0\nsome response\n%end 1234567890 1 0\n");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ControlModeEvent::BeginEnd { .. })),
            "should have BeginEnd event, got: {:?}",
            events
        );
    }

    #[test]
    fn test_parse_session_changed() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%session-changed $1 mysession\n");
        assert_eq!(events.len(), 1);
        match &events[0] {
            ControlModeEvent::SessionChanged { session_id, name } => {
                assert_eq!(session_id, "$1");
                assert_eq!(name, "mysession");
            }
            _ => panic!("expected SessionChanged"),
        }
    }

    #[test]
    fn test_parse_multiple_outputs() {
        let mut parser = ControlModeParser::new();
        let events = parser.feed(b"%output %0 line1\n%output %1 line2\n%exit\n");
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], ControlModeEvent::Output { pane_id, .. } if pane_id == "%0"));
        assert!(matches!(&events[1], ControlModeEvent::Output { pane_id, .. } if pane_id == "%1"));
        assert!(matches!(&events[2], ControlModeEvent::Exit));
    }

    // ── Runtime tests (require tmux) ──────────────────────────

    #[test]
    fn test_control_mode_runtime_backend_type() {
        if !crate::runtime::backends::tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let rt =
            TmuxControlModeRuntime::new("pmux-test-cc", "main", Some(dir.path()))
                .expect("should create control mode runtime");
        assert_eq!(rt.backend_type(), "tmux-cc");

        // Cleanup
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", "pmux-test-cc"])
            .output();
    }

    #[test]
    fn test_control_mode_subscribe_output_returns_receiver() {
        if !crate::runtime::backends::tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let rt =
            TmuxControlModeRuntime::new("pmux-test-sub", "main", Some(dir.path()))
                .expect("should create runtime");

        let panes = rt.list_panes(&String::new());
        let pane_id = panes.first().cloned().unwrap_or_else(|| "%0".to_string());
        let rx = rt.subscribe_output(&pane_id);
        assert!(rx.is_some(), "subscribe_output should return a receiver");

        // Cleanup
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", "pmux-test-sub"])
            .output();
    }

    #[test]
    fn test_control_mode_session_info() {
        if !crate::runtime::backends::tmux_available() {
            eprintln!("skipping: tmux not available");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let rt =
            TmuxControlModeRuntime::new("pmux-test-info", "main-window", Some(dir.path()))
                .expect("should create runtime");
        let info = rt.session_info();
        assert_eq!(info, Some(("pmux-test-info".to_string(), "main-window".to_string())));

        // Cleanup
        let _ = Command::new("tmux")
            .args(["kill-session", "-t", "pmux-test-info"])
            .output();
    }
}
