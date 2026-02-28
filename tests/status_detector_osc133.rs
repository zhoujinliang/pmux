//! Phase 4: Status detector OSC 133 shell phase integration tests.
//! Run with: cargo test --test status_detector_osc133

use pmux::agent_status::AgentStatus;
use pmux::shell_integration::{MarkerKind, ParsedMarker, ShellMarker, ShellPhase, ShellPhaseInfo};
use pmux::status_detector::StatusDetector;
use pmux::terminal::TerminalEngine;

#[test]
fn test_detect_with_shell_phase_running() {
    let detector = StatusDetector::new();
    let info = ShellPhaseInfo {
        phase: ShellPhase::Running,
        last_post_exec_exit_code: None,
    };
    let status = detector.detect_with_shell_phase("$ ls -la\nsome output", Some(info));
    assert_eq!(status, AgentStatus::Running);
}

#[test]
fn test_detect_with_shell_phase_output_error() {
    let detector = StatusDetector::new();
    let info = ShellPhaseInfo {
        phase: ShellPhase::Output,
        last_post_exec_exit_code: Some(1),
    };
    let status = detector.detect_with_shell_phase("command output", Some(info));
    assert_eq!(status, AgentStatus::Error);
}

#[test]
fn test_detect_with_shell_phase_output_success() {
    let detector = StatusDetector::new();
    let info = ShellPhaseInfo {
        phase: ShellPhase::Output,
        last_post_exec_exit_code: Some(0),
    };
    let status = detector.detect_with_shell_phase("$ echo done\ndone", Some(info));
    assert_eq!(status, AgentStatus::Idle);
}

#[test]
fn test_detect_with_shell_phase_unknown_fallback() {
    let detector = StatusDetector::new();
    let info = ShellPhaseInfo {
        phase: ShellPhase::Unknown,
        last_post_exec_exit_code: None,
    };
    assert_eq!(
        detector.detect_with_shell_phase("AI is thinking", Some(info)),
        AgentStatus::Running
    );
    assert_eq!(
        detector.detect_with_shell_phase("? What next?", Some(info)),
        AgentStatus::Waiting
    );
}

#[test]
fn test_detect_with_shell_phase_none_fallback() {
    let detector = StatusDetector::new();
    assert_eq!(
        detector.detect_with_shell_phase("AI is thinking", None),
        detector.detect("AI is thinking")
    );
}

#[test]
fn test_detect_with_shell_phase_priority_running_over_text() {
    let detector = StatusDetector::new();
    let info = ShellPhaseInfo {
        phase: ShellPhase::Running,
        last_post_exec_exit_code: None,
    };
    let status = detector.detect_with_shell_phase("error in log", Some(info));
    assert_eq!(status, AgentStatus::Running);
}

#[test]
fn test_detect_with_shell_phase_output_exit_2() {
    let detector = StatusDetector::new();
    let info = ShellPhaseInfo {
        phase: ShellPhase::Output,
        last_post_exec_exit_code: Some(2),
    };
    assert_eq!(
        detector.detect_with_shell_phase("any content", Some(info)),
        AgentStatus::Error
    );
}

#[test]
fn test_integration_with_terminal_engine() {
    let (tx, rx) = flume::unbounded();
    let engine = TerminalEngine::new(80, 24, rx);
    drop(tx);

    let detector = StatusDetector::new();

    let info = ShellPhaseInfo {
        phase: engine.shell_phase(),
        last_post_exec_exit_code: engine.last_post_exec_exit_code(),
    };
    assert_eq!(info.phase, ShellPhase::Unknown);
    assert_eq!(
        detector.detect_with_shell_phase("hello", Some(info)),
        AgentStatus::Idle
    );

    {
        let mut state = engine.shell_state();
        let marker = ShellMarker::from_parsed(
            ParsedMarker {
                kind: MarkerKind::PreExec,
                exit_code: None,
            },
            0,
            0,
        );
        state.add_marker(marker);
    }
    let info = ShellPhaseInfo {
        phase: engine.shell_phase(),
        last_post_exec_exit_code: engine.last_post_exec_exit_code(),
    };
    assert_eq!(info.phase, ShellPhase::Running);
    assert_eq!(
        detector.detect_with_shell_phase("any content", Some(info)),
        AgentStatus::Running
    );

    {
        let mut state = engine.shell_state();
        let marker = ShellMarker::from_parsed(
            ParsedMarker {
                kind: MarkerKind::PostExec,
                exit_code: Some(1),
            },
            1,
            0,
        );
        state.add_marker(marker);
    }
    let info = ShellPhaseInfo {
        phase: engine.shell_phase(),
        last_post_exec_exit_code: engine.last_post_exec_exit_code(),
    };
    assert_eq!(info.phase, ShellPhase::Output);
    assert_eq!(info.last_post_exec_exit_code, Some(1));
    assert_eq!(
        detector.detect_with_shell_phase("output", Some(info)),
        AgentStatus::Error
    );
}
