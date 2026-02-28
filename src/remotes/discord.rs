//! Discord Bot - send via REST API, receive via Gateway WebSocket.
//! No webhook; uses bot_token + channel_id only.

use crate::remotes::channel::{RemoteChannel, RemoteMessage, RemoteSendError};
use reqwest::blocking::Client;
use serde::Serialize;

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

pub struct DiscordBot {
    bot_token: String,
    channel_id: String,
    client: Client,
}

#[derive(Serialize)]
struct CreateMessageBody {
    content: String,
}

impl DiscordBot {
    /// Token for Gateway Identify (internal use).
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
    pub fn send_message(&self, content: &str) -> Result<(), RemoteSendError> {
        let url = format!(
            "{}/channels/{}/messages",
            DISCORD_API_BASE, self.channel_id
        );
        let body = CreateMessageBody {
            content: content.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(RemoteSendError::Api(format!("{}: {}", status, text)));
        }
        Ok(())
    }
}

impl RemoteChannel for DiscordBot {
    fn name(&self) -> &str {
        "discord"
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
    fn test_discord_bot_new_empty_token_fails() {
        assert!(DiscordBot::new(String::new(), "123".to_string()).is_err());
    }

    #[test]
    fn test_discord_bot_new_empty_channel_fails() {
        assert!(DiscordBot::new("token".to_string(), String::new()).is_err());
    }

    #[test]
    fn test_discord_bot_new_valid() {
        let b = DiscordBot::new("token".to_string(), "123".to_string()).unwrap();
        assert_eq!(b.name(), "discord");
    }
}
