use crate::remotes::channel::{RemoteChannel, RemoteMessage, RemoteSendError};
use reqwest::blocking::Client;
use serde::Serialize;

pub struct DiscordChannel {
    webhook_url: String,
    client: Client,
}

#[derive(Serialize)]
struct DiscordWebhookBody {
    content: String,
}

impl DiscordChannel {
    pub fn new(webhook_url: String) -> Result<Self, RemoteSendError> {
        if webhook_url.is_empty() {
            return Err(RemoteSendError::Api("webhook_url empty".into()));
        }
        Ok(Self {
            webhook_url,
            client: Client::new(),
        })
    }
}

impl RemoteChannel for DiscordChannel {
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
        let body = DiscordWebhookBody { content };
        let resp = self
            .client
            .post(&self.webhook_url)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_channel_new_empty_fails() {
        assert!(DiscordChannel::new(String::new()).is_err());
    }

    #[test]
    fn test_discord_channel_new_valid() {
        let c = DiscordChannel::new("https://discord.com/api/webhooks/123/abc".to_string()).unwrap();
        assert_eq!(c.name(), "discord");
    }
}
