//! ContentExtractor integration tests.
//! Run with: RUSTUP_TOOLCHAIN=stable cargo test --test content_extractor_test
//!
//! These live in integration tests to avoid gpui_macros SIGBUS during lib test compilation.

use pmux::shell_integration::ShellPhase;
use pmux::terminal::ContentExtractor;

#[test]
fn test_extracts_osc133_phase() {
    let mut ext = ContentExtractor::new();
    let st = b"\x1b]133;A\x1b\\"; // PromptStart
    ext.feed(st);
    assert_eq!(ext.shell_phase(), ShellPhase::Prompt);
}

#[test]
fn test_extracts_visible_text() {
    let mut ext = ContentExtractor::new();
    ext.feed(b"hello\r\n");
    let (text, _) = ext.take_content();
    assert!(text.contains("hello"));
}
