//! Stream adapters for gpui-terminal: RuntimeReader, RuntimeWriter, tee_output.

use std::io::{Read, Result as IoResult, Write};
use std::sync::Arc;

use crate::runtime::{AgentRuntime, PaneId};

/// Wraps flume::Receiver<Vec<u8>> as std::io::Read for gpui-terminal.
pub struct RuntimeReader {
    rx: flume::Receiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
}

impl RuntimeReader {
    pub fn new(rx: flume::Receiver<Vec<u8>>) -> Self {
        Self {
            rx,
            buf: Vec::new(),
            pos: 0,
        }
    }
}

impl Read for RuntimeReader {
    fn read(&mut self, out: &mut [u8]) -> IoResult<usize> {
        while self.pos >= self.buf.len() {
            match self.rx.recv() {
                Ok(chunk) => {
                    self.buf = chunk;
                    self.pos = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let n = (self.buf.len() - self.pos).min(out.len());
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Forwards std::io::Write to runtime.send_input for gpui-terminal.
pub struct RuntimeWriter {
    runtime: Arc<dyn AgentRuntime>,
    pane_id: PaneId,
}

impl RuntimeWriter {
    pub fn new(runtime: Arc<dyn AgentRuntime>, pane_id: PaneId) -> Self {
        Self { runtime, pane_id }
    }
}

impl Write for RuntimeWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        // #region agent log
        {
            let is_enter = buf == b"\r" || buf == b"\n" || buf == b"\r\n";
            let preview = if buf.len() <= 20 { format!("{:?}", buf) } else { format!("{:?}...", &buf[..20]) };
            crate::debug_log::dbg_session_log(
                "stream_adapter.rs:RuntimeWriter::write",
                "gpui_terminal write to PTY",
                &serde_json::json!({"bytes_len": buf.len(), "is_enter": is_enter, "preview": preview, "pane_id": self.pane_id}),
                "H1",
            );
        }
        // #endregion
        self.runtime
            .send_input(&self.pane_id, buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

/// Spawns a thread that forwards each chunk from `rx` to two new receivers.
/// Use one for gpui-terminal, one for ContentExtractor.
pub fn tee_output(rx: flume::Receiver<Vec<u8>>) -> (flume::Receiver<Vec<u8>>, flume::Receiver<Vec<u8>>) {
    let (tx1, rx1) = flume::unbounded();
    let (tx2, rx2) = flume::unbounded();
    std::thread::spawn(move || {
        while let Ok(chunk) = rx.recv() {
            let _ = tx1.send(chunk.clone());
            let _ = tx2.send(chunk);
        }
    });
    (rx1, rx2)
}

// Tests live in tests/stream_adapter_test.rs to avoid gpui_macros SIGBUS
// during lib test compilation on some macOS setups.
