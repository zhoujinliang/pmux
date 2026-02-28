//! KOOK Bot - send via REST API, receive via Gateway WebSocket.
//! No webhook; uses bot_token + channel_id only.

use crate::remotes::channel::{RemoteChannel, RemoteMessage, RemoteSendError};
use reqwest::blocking::Client;
use serde::Serialize;

const KOOK_API_BASE: &str = "https://www.kookapp.cn/api/v3";

pub struct KookBot {
    bot_token: String,
    channel_id: String,
    client: Client,
}

#[derive(Serialize)]
struct CreateMessageBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<u8>,
    target_id: String,
    content: String,
}

impl KookBot {
    /// Token for Gateway connection (internal use).
    pub(crate) fn bot_token(&self) -> &str {
        &self.bot_token
    }

    pub fn new(bot_token: String, channel_id: String) -> Result<Self, RemoteSendError> {
        if bot_token.is_empty() {
            return Err(RemoteSendError::Api("bot_token empty".into()));
        }
        if channel_id.is_empty() {
            return Err(RemoteSendError::Api("channel_id empty".into()));
        }
        Ok(Self {
            bot_token,
            channel_id,
            client: Client::new(),
        })
    }

    /// Send a message to the configured channel via Bot REST API.
    /// Uses type 9 (KMarkdown) for formatting support.
    pub fn send_message(&self, content: &str) -> Result<(), RemoteSendError> {
        self.send_to_target(&self.channel_id, content)
    }

    /// Send a message to a specific target (channel_id for group, user_id for DM).
    pub fn send_to_target(&self, target_id: &str, content: &str) -> Result<(), RemoteSendError> {
        let url = format!("{}/message/create", KOOK_API_BASE);
        let body = CreateMessageBody {
            r#type: Some(9),
            target_id: target_id.to_string(),
            content: content.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            return Err(RemoteSendError::Api(format!("{}: {}", status, text)));
        }
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
        let code = json["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            let msg = json["message"].as_str().unwrap_or("unknown");
            return Err(RemoteSendError::Api(format!("KOOK API error {}: {}", code, msg)));
        }
        Ok(())
    }
}

impl RemoteChannel for KookBot {
    fn name(&self) -> &str {
        "kook"
    }

    fn send(&self, msg: &RemoteMessage) -> Result<(), RemoteSendError> {
        let content = format!(
            "[pmux] **{}** / **{}** — {}\n{}",
            msg.workspace,
            msg.worktree,
            msg.title,
            msg.body
        );
        self.send_message(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kook_bot_new_empty_token_fails() {
        assert!(KookBot::new(String::new(), "123".to_string()).is_err());
    }

    #[test]
    fn test_kook_bot_new_empty_channel_fails() {
        assert!(KookBot::new("token".to_string(), String::new()).is_err());
    }

    #[test]
    fn test_kook_bot_new_valid() {
        let b = KookBot::new("token".to_string(), "123".to_string()).unwrap();
        assert_eq!(b.name(), "kook");
    }
}
