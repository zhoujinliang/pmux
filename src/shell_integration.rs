//! Shell integration via OSC 133 escape sequences.
//!
//! Parses OSC 133 sequences from terminal output to detect shell lifecycle markers:
//! - A: PromptStart
//! - B: PromptEnd
//! - C: PreExec (command about to run)
//! - D: PostExec (command finished, with optional exit code)
//!
//! ## Cursor Positioning API (ContentExtractor)
//!
//! OSC 133 markers are parsed by [`crate::terminal::content_extractor::ContentExtractor`]
//! in the status pipeline. Cursor positioning (prompt_line, click_to_prompt) may be implemented
//! when ContentExtractor exposes prompt line information for future UI features.

use std::time::Instant;

/// Shell phase derived from OSC 133 markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShellPhase {
    #[default]
    Unknown,
    Prompt,
    Input,
    Running,
    Output,
}

/// Maximum number of markers to retain (FIFO eviction).
pub const MAX_MARKERS: usize = 100;

/// Shell state tracked by ContentExtractor (OSC 133 parsing).
#[derive(Debug)]
pub struct ShellState {
    pub phase: ShellPhase,
    pub prompt_line: Option<usize>,
    pub last_command_start: Option<Instant>,
    /// Exit code from last PostExec marker (OSC 133;D;N). None if not yet seen or no code.
    pub last_post_exec_exit_code: Option<u8>,
    /// OSC 133 markers in order received (oldest first). Capped at MAX_MARKERS.
    pub markers: Vec<ShellMarker>,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            phase: ShellPhase::Unknown,
            prompt_line: None,
            last_command_start: None,
            last_post_exec_exit_code: None,
            markers: Vec::new(),
        }
    }

    /// Add a marker and update phase/prompt_line. Evicts oldest markers beyond MAX_MARKERS.
    pub fn add_marker(&mut self, marker: ShellMarker) {
        // State transitions per spec
        match marker.kind {
            MarkerKind::PromptStart => {
                self.phase = ShellPhase::Prompt;
                self.prompt_line = Some(marker.line);
            }
            MarkerKind::PromptEnd => {
                self.phase = ShellPhase::Input;
            }
            MarkerKind::PreExec => {
                self.phase = ShellPhase::Running;
                self.last_command_start = Some(marker.timestamp);
            }
            MarkerKind::PostExec => {
                self.phase = ShellPhase::Output;
                self.last_post_exec_exit_code = marker.exit_code;
            }
        }
        self.markers.push(marker);
        // Expire old markers (Task 3.3)
        if self.markers.len() > MAX_MARKERS {
            self.markers.drain(0..(self.markers.len() - MAX_MARKERS));
        }
    }
}

/// Shell phase info for status detection (phase + last PostExec exit code).
#[derive(Debug, Clone, Copy)]
pub struct ShellPhaseInfo {
    pub phase: ShellPhase,
    pub last_post_exec_exit_code: Option<u8>,
}

/// Marker kind parsed from OSC 133 sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// OSC 133;A - Prompt start
    PromptStart,
    /// OSC 133;B - Prompt end / input start
    PromptEnd,
    /// OSC 133;C - Pre-execution (command about to run)
    PreExec,
    /// OSC 133;D - Post-execution (command finished)
    PostExec,
}

/// A shell marker with optional exit code (for PostExec).
#[derive(Debug, Clone)]
pub struct ShellMarker {
    pub kind: MarkerKind,
    pub line: usize,
    pub column: usize,
    pub timestamp: Instant,
    /// Exit code for PostExec markers (e.g., 0 for success, 1 for failure).
    pub exit_code: Option<u8>,
}

/// Result of parsing one OSC 133 sequence.
#[derive(Debug, Clone)]
pub struct ParsedMarker {
    pub kind: MarkerKind,
    pub exit_code: Option<u8>,
}

/// Parser for OSC 133 escape sequences in a byte stream.
///
/// Handles both terminators: ST (ESC \) and BEL (\x07).
/// Call `feed()` with incoming bytes; returns `Some(ParsedMarker)` when a complete
/// sequence is parsed.
#[derive(Debug, Default)]
pub struct Osc133Parser {
    state: ParserState,
    buf: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ParserState {
    /// Looking for ESC
    #[default]
    Normal,
    /// Saw ESC, expect ] (OSC start) or \ (ST second byte)
    AfterEsc,
    /// Saw ESC ], reading OSC data
    InOsc,
    /// Saw ESC (potential ST) while in OSC - next byte determines if ST or escape in data
    InOscAfterEsc,
}

// OSC 133 constants
const ESC: u8 = 0x1b;
const OSC_START: u8 = b']';
const ST_SECOND: u8 = b'\\';
const BEL: u8 = 0x07;

impl Osc133Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed bytes into the parser. Returns any completed OSC 133 markers.
    /// Malformed sequences are discarded without panicking.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<ParsedMarker> {
        let mut results = Vec::new();
        for &b in bytes {
            if let Some(marker) = self.advance(b) {
                results.push(marker);
            }
        }
        results
    }

    fn advance(&mut self, b: u8) -> Option<ParsedMarker> {
        match self.state {
            ParserState::Normal => {
                if b == ESC {
                    self.state = ParserState::AfterEsc;
                }
                None
            }
            ParserState::AfterEsc => {
                if b == OSC_START {
                    self.state = ParserState::InOsc;
                    self.buf.clear();
                } else if b == ST_SECOND {
                    // ESC \ - ST without OSC, discard
                    self.state = ParserState::Normal;
                } else {
                    self.state = ParserState::Normal;
                }
                None
            }
            ParserState::InOsc => {
                if b == BEL {
                    // BEL terminator - parse buffer and reset
                    let result = self.parse_osc133_buffer();
                    self.state = ParserState::Normal;
                    self.buf.clear();
                    return result;
                }
                if b == ESC {
                    self.state = ParserState::InOscAfterEsc;
                    return None;
                }
                self.buf.push(b);
                None
            }
            ParserState::InOscAfterEsc => {
                if b == ST_SECOND {
                    // ST terminator - parse buffer and reset
                    let result = self.parse_osc133_buffer();
                    self.state = ParserState::Normal;
                    self.buf.clear();
                    return result;
                }
                // ESC was part of data (e.g., nested escape), push both and continue
                self.buf.push(ESC);
                self.buf.push(b);
                self.state = ParserState::InOsc;
                None
            }
        }
    }

    /// Parse the accumulated OSC data buffer. Returns None for non-133 or malformed.
    fn parse_osc133_buffer(&self) -> Option<ParsedMarker> {
        // Expected: "133;A" or "133;B" or "133;C" or "133;D" or "133;D;0" etc
        let s = std::str::from_utf8(&self.buf).ok()?;
        let s = s.trim();
        if !s.starts_with("133;") {
            return None;
        }
        let rest = &s[4..]; // after "133;"
        let mut parts = rest.split(';');
        let kind_char = parts.next()?.chars().next()?;
        let kind = match kind_char {
            'A' => MarkerKind::PromptStart,
            'B' => MarkerKind::PromptEnd,
            'C' => MarkerKind::PreExec,
            'D' => MarkerKind::PostExec,
            _ => return None,
        };
        // For A/B/C, there should be nothing else (or optional params we ignore)
        // For D, second param is exit code
        let exit_code = if kind == MarkerKind::PostExec {
            parts.next().and_then(|s| s.trim().parse::<u8>().ok())
        } else {
            None
        };
        Some(ParsedMarker { kind, exit_code })
    }

    /// Reset parser state (e.g., on stream discontinuity).
    pub fn reset(&mut self) {
        self.state = ParserState::Normal;
        self.buf.clear();
    }
}

impl ShellMarker {
    /// Create a ShellMarker from a ParsedMarker with coordinates.
    pub fn from_parsed(parsed: ParsedMarker, line: usize, column: usize) -> Self {
        Self {
            kind: parsed.kind,
            line,
            column,
            timestamp: Instant::now(),
            exit_code: parsed.exit_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn osc133_st(kind: char, exit_code: Option<u8>) -> Vec<u8> {
        let mut s = format!("\x1b]133;{}", kind);
        if let Some(code) = exit_code {
            s.push_str(&format!(";{}", code));
        }
        s.push_str("\x1b\\");
        s.into_bytes()
    }

    fn osc133_bel(kind: char, exit_code: Option<u8>) -> Vec<u8> {
        let mut s = format!("\x1b]133;{}", kind);
        if let Some(code) = exit_code {
            s.push_str(&format!(";{}", code));
        }
        s.push('\x07');
        s.into_bytes()
    }

    #[test]
    fn test_prompt_start_st() {
        let seq = osc133_st('A', None);
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PromptStart);
        assert_eq!(markers[0].exit_code, None);
    }

    #[test]
    fn test_prompt_end_st() {
        let seq = osc133_st('B', None);
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PromptEnd);
    }

    #[test]
    fn test_pre_exec_st() {
        let seq = osc133_st('C', None);
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PreExec);
    }

    #[test]
    fn test_post_exec_with_exit_code_st() {
        let seq = osc133_st('D', Some(0));
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PostExec);
        assert_eq!(markers[0].exit_code, Some(0));
    }

    #[test]
    fn test_post_exec_exit_code_1_st() {
        let seq = osc133_st('D', Some(1));
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PostExec);
        assert_eq!(markers[0].exit_code, Some(1));
    }

    #[test]
    fn test_post_exec_bel() {
        let seq = osc133_bel('D', Some(0));
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PostExec);
        assert_eq!(markers[0].exit_code, Some(0));
    }

    #[test]
    fn test_post_exec_without_exit_code() {
        // OSC 133;D with no second param - exit_code should be None
        let seq = b"\x1b]133;D\x1b\\";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PostExec);
        assert_eq!(markers[0].exit_code, None);
    }

    #[test]
    fn test_example_sequences_from_spec() {
        // Example: \x1b]133;A\x1b\ - PromptStart
        let seq = b"\x1b]133;A\x1b\\";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PromptStart);

        // Example: \x1b]133;D;0\x07 - PostExec with exit code 0
        let seq = b"\x1b]133;D;0\x07";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].kind, MarkerKind::PostExec);
        assert_eq!(markers[0].exit_code, Some(0));
    }

    #[test]
    fn test_all_markers_bel() {
        for (kind_char, expected_kind) in [
            ('A', MarkerKind::PromptStart),
            ('B', MarkerKind::PromptEnd),
            ('C', MarkerKind::PreExec),
            ('D', MarkerKind::PostExec),
        ] {
            let seq = osc133_bel(kind_char, None);
            let mut p = Osc133Parser::new();
            let markers = p.feed(&seq);
            assert_eq!(markers.len(), 1, "Failed for {:?}", expected_kind);
            assert_eq!(markers[0].kind, expected_kind);
        }
    }

    #[test]
    fn test_sequential_markers() {
        let seq = [osc133_st('A', None), osc133_st('B', None)].concat();
        let mut p = Osc133Parser::new();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].kind, MarkerKind::PromptStart);
        assert_eq!(markers[1].kind, MarkerKind::PromptEnd);
    }

    #[test]
    fn test_incremental_feed() {
        let seq = osc133_st('A', None);
        let mut p = Osc133Parser::new();
        let mut all_markers = Vec::new();
        for b in &seq {
            all_markers.extend(p.feed(std::slice::from_ref(b)));
        }
        assert_eq!(all_markers.len(), 1);
        assert_eq!(all_markers[0].kind, MarkerKind::PromptStart);
    }

    #[test]
    fn test_malformed_not_osc133() {
        // OSC 0 (window title) - should not match
        let seq = b"\x1b]0;title\x07";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_malformed_invalid_marker() {
        // OSC 133;X (invalid marker char)
        let seq = b"\x1b]133;X\x1b\\";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_malformed_truncated() {
        // Incomplete: ESC ] 133 ; A (no terminator)
        let seq = b"\x1b]133;A";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_malformed_garbage_no_panic() {
        let seq = b"hello world \x00 \xff random bytes";
        let mut p = Osc133Parser::new();
        let markers = p.feed(seq);
        assert!(markers.is_empty());
    }

    #[test]
    fn test_reset() {
        let seq = osc133_st('A', None);
        let mut p = Osc133Parser::new();
        p.feed(&seq[..4]); // partial
        p.reset();
        let markers = p.feed(&seq);
        assert_eq!(markers.len(), 1);
    }

    #[test]
    fn test_shell_marker_from_parsed() {
        let parsed = ParsedMarker {
            kind: MarkerKind::PostExec,
            exit_code: Some(1),
        };
        let m = ShellMarker::from_parsed(parsed, 10, 5);
        assert_eq!(m.kind, MarkerKind::PostExec);
        assert_eq!(m.exit_code, Some(1));
        assert_eq!(m.line, 10);
        assert_eq!(m.column, 5);
    }
}
