//! Feishu (飞书) Bot - send via REST API.
//! Uses app_id + app_secret → tenant_access_token (refresh every 2h).
//! receive_id (chat_id) for target group.

use crate::remotes::channel::{RemoteChannel, RemoteMessage, RemoteSendError};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Instant;

const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";

/// Token expiry buffer: refresh 5 min before actual expiry.
const TOKEN_EXPIRE_BUFFER_SECS: u64 = 300;

pub struct FeishuBot {
    app_id: String,
    app_secret: String,
    chat_id: String,
    client: Client,
    token_cache: Mutex<TokenCache>,
}

#[derive(Default)]
struct TokenCache {
    token: Option<String>,
    expires_at: Option<Instant>,
}

#[derive(Serialize)]
struct TokenRequest {
    app_id: String,
    app_secret: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    #[serde(default)]
    code: i64,
    #[serde(default)]
    msg: String,
    #[serde(default)]
    tenant_access_token: String,
    #[serde(default)]
    expire: u64,
}

#[derive(Serialize)]
struct CreateMessageBody {
    receive_id: String,
    msg_type: String,
    content: String,
}

impl FeishuBot {
    pub fn new(app_id: String, app_secret: String, chat_id: String) -> Result<Self, RemoteSendError> {
        if app_id.is_empty() {
            return Err(RemoteSendError::Api("app_id empty".into()));
        }
        if app_secret.is_empty() {
            return Err(RemoteSendError::Api("app_secret empty".into()));
        }
        if chat_id.is_empty() {
            return Err(RemoteSendError::Api("chat_id empty".into()));
        }
        Ok(Self {
            app_id,
            app_secret,
            chat_id,
            client: Client::new(),
            token_cache: Mutex::new(TokenCache::default()),
        })
    }

    fn get_token(&self) -> Result<String, RemoteSendError> {
        let mut cache = self.token_cache.lock().map_err(|e| {
            RemoteSendError::Api(format!("token cache lock: {}", e))
        })?;
        let now = Instant::now();
        if let (Some(ref token), Some(expires)) = (&cache.token, cache.expires_at) {
            if now < expires {
                return Ok(token.clone());
            }
        }

        let body = TokenRequest {
            app_id: self.app_id.clone(),
            app_secret: self.app_secret.clone(),
        };
        let resp = self
            .client
            .post(format!("{}/auth/v3/tenant_access_token/internal", FEISHU_API_BASE))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()?;
        let text = resp.text()?;
        let json: TokenResponse = serde_json::from_str(&text)
            .map_err(|e| RemoteSendError::Api(format!("Feishu token JSON: {}", e)))?;
        if json.code != 0 {
            return Err(RemoteSendError::Api(format!(
                "Feishu token error {}: {}",
                json.code, json.msg
            )));
        }
        if json.tenant_access_token.is_empty() {
            return Err(RemoteSendError::Api("Feishu token empty".into()));
        }

        let expire_secs = json.expire.saturating_sub(TOKEN_EXPIRE_BUFFER_SECS);
        cache.token = Some(json.tenant_access_token.clone());
        cache.expires_at = Some(now + std::time::Duration::from_secs(expire_secs));

        Ok(json.tenant_access_token)
    }

    /// Send a message to the configured chat via Bot REST API.
    pub fn send_message(&self, content: &str) -> Result<(), RemoteSendError> {
        let token = self.get_token()?;
        let url = format!(
            "{}/im/v1/messages?receive_id_type=chat_id",
            FEISHU_API_BASE
        );
        let content_json = serde_json::json!({ "text": content }).to_string();
        let body = CreateMessageBody {
            receive_id: self.chat_id.clone(),
            msg_type: "text".to_string(),
            content: content_json,
        };
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            return Err(RemoteSendError::Api(format!("{}: {}", status, text)));
        }
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| RemoteSendError::Api(format!("Feishu msg JSON: {}", e)))?;
        let code = json["code"].as_i64().unwrap_or(-1);
        if code != 0 {
            let msg = json["msg"].as_str().unwrap_or("unknown");
            return Err(RemoteSendError::Api(format!("Feishu API {}: {}", code, msg)));
        }
        Ok(())
    }
}

impl RemoteChannel for FeishuBot {
    fn name(&self) -> &str {
        "feishu"
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
    fn test_feishu_bot_new_empty_app_id_fails() {
        assert!(FeishuBot::new(
            String::new(),
            "secret".to_string(),
            "chat".to_string()
        )
        .is_err());
    }

    #[test]
    fn test_feishu_bot_new_valid() {
        let b = FeishuBot::new(
            "app".to_string(),
            "secret".to_string(),
            "chat".to_string(),
        )
        .unwrap();
        assert_eq!(b.name(), "feishu");
    }
}
