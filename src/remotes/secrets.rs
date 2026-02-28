//! Secrets loading and saving for ~/.config/pmux/secrets.json

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Secrets {
    #[serde(default)]
    pub remote_channels: RemoteChannelSecrets,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RemoteChannelSecrets {
    #[serde(default)]
    pub discord: DiscordSecrets,
    #[serde(default)]
    pub kook: KookSecrets,
    #[serde(default)]
    pub feishu: FeishuSecrets,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiscordSecrets {
    /// Bot token for Discord Bot API (send + Gateway receive)
    pub bot_token: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct KookSecrets {
    pub bot_token: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FeishuSecrets {
    /// Feishu app_id (for tenant_access_token)
    pub app_id: Option<String>,
    /// Feishu app_secret (for tenant_access_token)
    pub app_secret: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretsLoadError {
    #[error("Config directory not found")]
    ConfigDirNotFound,
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl Secrets {
    /// Load secrets from the default path (~/.config/pmux/secrets.json).
    /// Returns default if the file does not exist.
    pub fn load() -> Result<Self, SecretsLoadError> {
        let path = Self::path().ok_or(SecretsLoadError::ConfigDirNotFound)?;
        Self::load_from_path(&path)
    }

    /// Load secrets from a specific path.
    /// Returns default if the file does not exist.
    pub fn load_from_path(path: &PathBuf) -> Result<Self, SecretsLoadError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let secrets: Self = serde_json::from_str(&content)?;
        Ok(secrets)
    }

    /// Returns the default secrets file path, or None if config dir is not available.
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("pmux").join("secrets.json"))
    }

    /// Save secrets to the default path (~/.config/pmux/secrets.json).
    pub fn save(&self) -> Result<(), SecretsLoadError> {
        let path = Self::path().ok_or(SecretsLoadError::ConfigDirNotFound)?;
        self.save_to_path(&path)
    }

    /// Save secrets to a specific path.
    pub fn save_to_path(&self, path: &PathBuf) -> Result<(), SecretsLoadError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_secrets_load_missing_returns_default() {
        let temp = TempDir::new().unwrap();
        let pmux_dir = temp.path().join("pmux");
        std::fs::create_dir_all(&pmux_dir).unwrap();
        // No secrets.json - load_from_path on non-existent returns default
        let path = pmux_dir.join("secrets.json");
        let s = Secrets::load_from_path(&path).unwrap();
        assert!(s.remote_channels.discord.bot_token.is_none());
    }

    #[test]
    fn test_secrets_load_from_file() {
        let temp = TempDir::new().unwrap();
        let pmux_dir = temp.path().join("pmux");
        std::fs::create_dir_all(&pmux_dir).unwrap();
        let path = pmux_dir.join("secrets.json");
        std::fs::write(
            &path,
            r#"{"remote_channels":{"discord":{"bot_token":"my-bot-token-xxx"},"feishu":{"app_id":"cli_abc","app_secret":"secret123"}}}"#,
        )
        .unwrap();
        let s = Secrets::load_from_path(&path).unwrap();
        assert_eq!(
            s.remote_channels.discord.bot_token.as_deref(),
            Some("my-bot-token-xxx")
        );
        assert_eq!(
            s.remote_channels.feishu.app_id.as_deref(),
            Some("cli_abc")
        );
        assert_eq!(
            s.remote_channels.feishu.app_secret.as_deref(),
            Some("secret123")
        );
    }

    #[test]
    fn test_secrets_save_and_load() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("secrets.json");
        let secrets = Secrets {
            remote_channels: RemoteChannelSecrets {
                discord: DiscordSecrets {
                    bot_token: Some("token-xyz".to_string()),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        secrets.save_to_path(&path).unwrap();
        let loaded = Secrets::load_from_path(&path).unwrap();
        assert_eq!(
            loaded.remote_channels.discord.bot_token.as_deref(),
            Some("token-xyz")
        );
    }
}
