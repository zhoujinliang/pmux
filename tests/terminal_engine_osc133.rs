//! Integration tests for TerminalEngine OSC 133 shell integration (Phase 3).
//! Run with: cargo test --test terminal_engine_osc133

use pmux::terminal::TerminalEngine;
use pmux::shell_integration::{MarkerKind, ShellPhase};

fn osc133_st(kind: char, exit_code: Option<u8>) -> Vec<u8> {
    let mut s = format!("\x1b]133;{}", kind);
    if let Some(code) = exit_code {
        s.push_str(&format!(";{}", code));
    }
    s.push_str("\x1b\\");
    s.into_bytes()
}

#[test]
fn test_engine_shell_state_initialized() {
    let (tx, rx) = flume::unbounded();
    let engine = TerminalEngine::new(80, 24, rx);
    drop(tx);
    assert_eq!(engine.shell_phase(), ShellPhase::Unknown);
    assert_eq!(engine.prompt_line(), None);
    assert_eq!(engine.last_post_exec_exit_code(), None);
    assert!(engine.shell_state().markers.is_empty());
}

#[test]
fn test_advance_with_osc133_phase_transitions() {
    let (tx, rx) = flume::unbounded();
    let engine = TerminalEngine::new(80, 24, rx);

    // Unknown -> Prompt (A)
    tx.send(osc133_st('A', None)).unwrap();
    engine.advance_with_osc133();
    assert_eq!(engine.shell_phase(), ShellPhase::Prompt);
    assert!(engine.prompt_line().is_some());

    // Prompt -> Input (B)
    tx.send(osc133_st('B', None)).unwrap();
    engine.advance_with_osc133();
    assert_eq!(engine.shell_phase(), ShellPhase::Input);

    // Input -> Running (C)
    tx.send(osc133_st('C', None)).unwrap();
    engine.advance_with_osc133();
    assert_eq!(engine.shell_phase(), ShellPhase::Running);

    // Running -> Output (D)
    tx.send(osc133_st('D', Some(0))).unwrap();
    engine.advance_with_osc133();
    assert_eq!(engine.shell_phase(), ShellPhase::Output);
    assert_eq!(engine.last_post_exec_exit_code(), Some(0));

    drop(tx);
}

#[test]
fn test_advance_with_osc133_markers_stored() {
    let (tx, rx) = flume::unbounded();
    let engine = TerminalEngine::new(80, 24, rx);
    tx.send(osc133_st('A', None)).unwrap();
    tx.send(osc133_st('B', None)).unwrap();
    engine.advance_with_osc133();
    drop(tx);

    let state = engine.shell_state();
    assert_eq!(state.markers.len(), 2);
    assert_eq!(state.markers[0].kind, MarkerKind::PromptStart);
    assert_eq!(state.markers[1].kind, MarkerKind::PromptEnd);
}

#[test]
fn test_resize_removes_off_screen_markers() {
    let (tx, rx) = flume::unbounded();
    let engine = TerminalEngine::new(80, 24, rx);
    tx.send(osc133_st('A', None)).unwrap();
    engine.advance_with_osc133();
    drop(tx);

    // Resize to fewer rows - marker at line 0 should remain (line 0 < 12)
    engine.resize(80, 12);
    assert!(!engine.shell_state().markers.is_empty());

    // Resize to 1 row - marker at line 0 remains (line 0 < 1)
    engine.resize(80, 1);
    assert!(!engine.shell_state().markers.is_empty());
}
