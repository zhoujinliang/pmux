// tests/config_test.rs - Integration tests for config loading

use pmux::config::Config;
use tempfile::TempDir;

#[test]
fn test_config_remote_channels() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("config.json");
    std::fs::write(
        &path,
        r#"{"remote_channels":{"discord":{"enabled":true},"kook":{"enabled":true,"channel_id":"123"}}}"#,
    )
    .unwrap();
    let config = Config::load_from_path(&path).unwrap();
    assert!(config.remote_channels.discord.enabled);
    assert_eq!(
        config.remote_channels.kook.channel_id.as_deref(),
        Some("123")
    );
}
