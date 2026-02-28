# Remote Channels 实施计划

> **For Claude:** Use TDD when implementing. Consider `subagent-driven-development` for parallel tasks.
>
> **Context:** design.md §13 远程通知通道；提前搭架子；Discord/KOOK 优先，飞书滞后。

**Goal:** 通过 IM 平台（Discord、KOOK）推送 Agent 状态变化与告警，支持遥控命令；配置与敏感信息分离（secrets.json）。

**Architecture:** Event Bus 订阅 → RemoteChannelPublisher 过滤/格式化 → 各 Adapter（Discord/KOOK）发送。统一 `RemoteChannel` trait。config.json 存非敏感配置，secrets.json 存 webhook_url/bot_token。

**Tech Stack:** Rust, reqwest, serde, flume, design.md §13

---

## Phase 概览

| Phase | 内容 | 预估 |
|-------|------|------|
| **A** | 骨架 + Discord 推送（Webhook） | 1~2 天 |
| **B** | KOOK Adapter + 事件富化 workspace/worktree | 0.5~1 天 |
| **C** | 遥控命令（Discord 斜杠、status/restart） | 1~2 天 |

本计划详述 **Phase A**，B/C 为后续任务提纲。

---

## Phase A: 骨架 + Discord 推送

### 前置条件

- Event Bus 已存在（`src/runtime/event_bus.rs`）
- StatusPublisher 发布 `AgentStateChange`、`Notification`
- Config 从 `~/.config/pmux/config.json` 加载

### 依赖顺序

```
Task 1 (reqwest) → Task 2 (remotes mod) → Task 3 (channel trait) → Task 4 (secrets)
       → Task 5 (config remote_channels) → Task 6 (publisher) → Task 7 (discord) → Task 8 (wire up)
```

---

### Task 1: 添加 reqwest 依赖

**Files:** Modify `Cargo.toml`

**Step 1: Add dependency**

```toml
# In [dependencies]
reqwest = { version = "0.12", default-features = false, features = ["json"] }
```

**Step 2: Verify**

```bash
cargo check
```

Expected: Compiles. No test needed for dependency add.

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add reqwest for remote channels HTTP"
```

---

### Task 2: 创建 src/remotes 模块骨架

**Files:**
- Create: `src/remotes/mod.rs`
- Modify: `src/lib.rs`

**Step 1: Create mod.rs**

```rust
// src/remotes/mod.rs
//! remotes - Remote notification channels (Discord, KOOK, etc.)

pub mod channel;
pub mod publisher;
pub mod discord;

pub use channel::{RemoteChannel, RemoteMessage};
pub use publisher::RemoteChannelPublisher;
pub use discord::DiscordChannel;
```

**Step 2: Add to lib.rs**

In `src/lib.rs`, add:

```rust
pub mod remotes;
```

**Step 3: Create stub files (empty for now)**

- Create `src/remotes/channel.rs` with `pub struct RemoteMessage;` (placeholder)
- Create `src/remotes/publisher.rs` with `pub struct RemoteChannelPublisher;`
- Create `src/remotes/discord.rs` with `pub struct DiscordChannel;`

**Step 4: Verify**

```bash
cargo check
```

Expected: Compiles (may have dead_code warnings for empty structs).

**Step 5: Commit**

```bash
git add src/remotes/ src/lib.rs
git commit -m "feat(remotes): add module skeleton"
```

---

### Task 3: 定义 RemoteMessage 与 RemoteChannel trait

**Files:** Modify `src/remotes/channel.rs`

**Step 1: Write the failing test**

```rust
// src/remotes/channel.rs
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RemoteMessage {
    pub workspace: String,
    pub worktree: String,
    pub severity: RemoteSeverity,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteSeverity {
    Info,
    Warning,
    Error,
}

pub trait RemoteChannel: Send + Sync {
    fn name(&self) -> &str;
    fn send(&self, msg: &RemoteMessage) -> Result<(), RemoteSendError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RemoteSendError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_message_serialize() {
        let msg = RemoteMessage {
            workspace: "repo-a".to_string(),
            worktree: "feat-x".to_string(),
            severity: RemoteSeverity::Error,
            title: "Agent Error".to_string(),
            body: "Something went wrong".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("repo-a"));
        assert!(json.contains("Error"));
    }
}
```

**Step 2: Run test**

```bash
cargo test remotes::channel::tests::test_remote_message_serialize
```

Expected: PASS (no TDD red phase for simple struct).

**Step 3: Commit**

```bash
git add src/remotes/channel.rs
git commit -m "feat(remotes): add RemoteMessage and RemoteChannel trait"
```

---

### Task 4: Secrets 加载

**Files:**
- Create: `src/remotes/secrets.rs`
- Modify: `src/remotes/mod.rs`

**Step 1: Write the failing test**

```rust
// src/remotes/secrets.rs
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
pub struct Secrets {
    #[serde(default)]
    pub remote_channels: RemoteChannelSecrets,
}

#[derive(Debug, Default, Deserialize)]
pub struct RemoteChannelSecrets {
    #[serde(default)]
    pub discord: DiscordSecrets,
    #[serde(default)]
    pub kook: KookSecrets,
}

#[derive(Debug, Default, Deserialize)]
pub struct DiscordSecrets {
    pub webhook_url: Option<String>,
    pub bot_token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct KookSecrets {
    pub bot_token: Option<String>,
}

impl Secrets {
    pub fn load() -> Result<Self, SecretsLoadError> {
        let path = Self::path().ok_or(SecretsLoadError::ConfigDirNotFound)?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let secrets: Self = serde_json::from_str(&content)?;
        Ok(secrets)
    }

    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("pmux").join("secrets.json"))
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_secrets_load_missing_returns_default() {
        let temp = TempDir::new().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let s = Secrets::load().unwrap();
        assert!(s.remote_channels.discord.webhook_url.is_none());
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    fn test_secrets_load_from_file() {
        let temp = TempDir::new().unwrap();
        let pmux_dir = temp.path().join("pmux");
        std::fs::create_dir_all(&pmux_dir).unwrap();
        let path = pmux_dir.join("secrets.json");
        std::fs::write(
            &path,
            r#"{"remote_channels":{"discord":{"webhook_url":"https://discord.com/api/webhooks/xxx"}}}"#,
        )
        .unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());
        let s = Secrets::load().unwrap();
        assert_eq!(
            s.remote_channels.discord.webhook_url.as_deref(),
            Some("https://discord.com/api/webhooks/xxx")
        );
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
```

**Step 2: Run test**

```bash
cargo test remotes::secrets::tests
```

Expected: PASS. Note: `dirs::config_dir()` on macOS may use `~/Library/Application Support` when XDG_CONFIG_HOME is not set; test sets it for isolation.

**Step 3: Add to mod.rs**

In `src/remotes/mod.rs`:

```rust
pub mod secrets;
pub use secrets::Secrets;
```

**Step 4: Commit**

```bash
git add src/remotes/secrets.rs src/remotes/mod.rs
git commit -m "feat(remotes): add Secrets loading from ~/.config/pmux/secrets.json"
```

---

### Task 5: Config 添加 remote_channels

**Files:** Modify `src/config.rs`

**Step 1: Write the failing test**

```rust
// In config_test.rs or config #[cfg(test)] mod tests
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
    assert_eq!(config.remote_channels.kook.channel_id.as_deref(), Some("123"));
}
```

**Step 2: Add structs and Config field**

```rust
// In config.rs - add before Config struct

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemoteChannelsConfig {
    #[serde(default)]
    pub discord: DiscordChannelConfig,
    #[serde(default)]
    pub kook: KookChannelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KookChannelConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub channel_id: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for DiscordChannelConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

impl Default for KookChannelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            channel_id: None,
        }
    }
}
```

Add to Config:

```rust
#[serde(default)]
pub remote_channels: RemoteChannelsConfig,
```

Add to Config::default():

```rust
remote_channels: RemoteChannelsConfig::default(),
```

**Step 3: Run test**

```bash
cargo test config::tests::test_config_remote_channels
```

Expected: PASS.

**Step 4: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add remote_channels (enabled, channel_id)"
```

---

### Task 6: RemoteChannelPublisher

**Files:** Modify `src/remotes/publisher.rs`

**Step 1: Implement publisher**

```rust
// src/remotes/publisher.rs
//! Subscribes to Event Bus, filters AgentStateChange + Notification, formats and sends to channels.

use std::sync::Arc;
use std::thread;

use crate::agent_status::AgentStatus;
use crate::remotes::channel::{RemoteChannel, RemoteMessage, RemoteSeverity};
use crate::remotes::discord::DiscordChannel;
use crate::remotes::secrets::Secrets;
use crate::runtime::event_bus::{AgentStateChange, Notification, NotificationType, RuntimeEvent};
use flume::Receiver;

pub struct RemoteChannelPublisher {
    channels: Vec<Box<dyn RemoteChannel>>,
}

impl RemoteChannelPublisher {
    pub fn has_channels(&self) -> bool {
        !self.channels.is_empty()
    }

    pub fn from_config(config: &crate::config::Config, secrets: &Secrets) -> Self {
        let mut channels: Vec<Box<dyn RemoteChannel>> = Vec::new();
        if config.remote_channels.discord.enabled {
            if let Some(ref url) = secrets.remote_channels.discord.webhook_url {
                if let Ok(dc) = DiscordChannel::new(url.clone()) {
                    channels.push(Box::new(dc));
                }
            }
        }
        Self { channels }
    }

    pub fn run(self, rx: Receiver<RuntimeEvent>) {
        thread::spawn(move || {
            for ev in rx {
                if self.channels.is_empty() {
                    continue;
                }
                let msg = match &ev {
                    RuntimeEvent::AgentStateChange(a) => Self::state_to_message(a),
                    RuntimeEvent::Notification(n) => Self::notification_to_message(n),
                    _ => continue,
                };
                for ch in &self.channels {
                    if let Err(e) = ch.send(&msg) {
                        eprintln!("remotes: {} send failed: {}", ch.name(), e);
                    }
                }
            }
        });
    }

    fn state_to_message(a: &AgentStateChange) -> RemoteMessage {
        let (workspace, worktree) = Self::parse_agent_id(&a.agent_id);
        let title = format!("Agent: {}", a.state.display_text());
        RemoteMessage {
            workspace,
            worktree,
            severity: Self::status_to_severity(&a.state),
            title,
            body: a.state.display_text().to_string(),
        }
    }

    fn notification_to_message(n: &Notification) -> RemoteMessage {
        let (workspace, worktree) = Self::parse_agent_id(&n.agent_id);
        let severity = match n.notif_type {
            NotificationType::Error => RemoteSeverity::Error,
            NotificationType::WaitingInput | NotificationType::WaitingConfirm => RemoteSeverity::Warning,
            NotificationType::Info => RemoteSeverity::Info,
        };
        RemoteMessage {
            workspace,
            worktree,
            severity,
            title: format!("{:?}", n.notif_type),
            body: n.message.clone(),
        }
    }

    fn parse_agent_id(agent_id: &str) -> (String, String) {
        // agent_id can be "local:/path/to/worktree" or "session:window"
        if let Some(rest) = agent_id.strip_prefix("local:") {
            let path = std::path::Path::new(rest);
            let worktree = path.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| rest.to_string());
            let workspace = path.parent().map(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).unwrap_or_default();
            (workspace, worktree)
        } else {
            (agent_id.to_string(), agent_id.to_string())
        }
    }

    fn status_to_severity(s: &AgentStatus) -> RemoteSeverity {
        match s {
            AgentStatus::Error => RemoteSeverity::Error,
            AgentStatus::Waiting | AgentStatus::WaitingConfirm => RemoteSeverity::Warning,
            _ => RemoteSeverity::Info,
        }
    }
}
```

**Step 2: Fix compile errors**

- `run` consumes `self`; move `channels` into the closure so the thread owns them:

```rust
pub fn run(self, rx: Receiver<RuntimeEvent>) {
    let channels = self.channels;
    thread::spawn(move || {
        for ev in rx {
            if channels.is_empty() {
                continue;
            }
            let msg = match &ev {
                RuntimeEvent::AgentStateChange(a) => Self::state_to_message(a),
                RuntimeEvent::Notification(n) => Self::notification_to_message(n),
                _ => continue,
            };
            for ch in &channels {
                if let Err(e) = ch.send(&msg) {
                    eprintln!("remotes: {} send failed: {}", ch.name(), e);
                }
            }
        }
    });
}
```

- 将 Task 6 原始实现中的 loop 体改为上述形式；并添加 `has_channels()` 供 Task 8 使用：

```rust
pub fn has_channels(&self) -> bool {
    !self.channels.is_empty()
}
```

**Step 3: Verify**

```bash
cargo check
```

**Step 4: Commit**

```bash
git add src/remotes/publisher.rs
git commit -m "feat(remotes): add RemoteChannelPublisher"
```

---

### Task 7: Discord Adapter

**Files:** Modify `src/remotes/discord.rs`

**Step 1: Implement DiscordChannel**

```rust
// src/remotes/discord.rs
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
    use crate::remotes::channel::RemoteSeverity;

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
```

**Step 2: Run test**

```bash
cargo test remotes::discord::tests
```

Expected: PASS. (No live webhook test in CI.)

**Step 3: Commit**

```bash
git add src/remotes/discord.rs
git commit -m "feat(remotes): add Discord webhook adapter"
```

---

### Task 8: 接入 AppRoot

**Files:** Modify `src/ui/app_root.rs`

**Step 1: Spawn publisher on startup**

在 `AppRoot::new` 或首次有 Event Bus 订阅的地方，在 `ensure_event_bus_subscription` 附近：

- 加载 Config 和 Secrets
- 若 `config.remote_channels.discord.enabled` 且 secrets 有 webhook_url，创建 `RemoteChannelPublisher` 并 `run(rx)`

由于 flume `subscribe()` 返回的是 clone 的 receiver，每个 subscribe 都会收到一份事件。我们需要在已有 Event Bus 订阅循环之外，单独为 RemoteChannelPublisher 增加一个 subscribe。

查找 `event_bus.subscribe()` 的调用位置，在同样位置附近：

```rust
// In ensure_event_bus_subscription or where event_bus rx is used
let remote_rx = self.event_bus.subscribe();
if let (Ok(config), Ok(secrets)) = (Config::load(), crate::remotes::Secrets::load()) {
    let publisher = RemoteChannelPublisher::from_config(&config, &secrets);
    if !matches!(publisher.channels.len(), 0) {
        publisher.run(remote_rx);
    }
}
```

注意：`from_config` 返回的 publisher 的 `channels` 是私有的，无法直接 `channels.len()`。改为在 `RemoteChannelPublisher` 增加 `fn is_empty(&self) -> bool` 或 `fn has_channels(&self) -> bool`。

**Step 2: Add method**

```rust
impl RemoteChannelPublisher {
    pub fn has_channels(&self) -> bool {
        !self.channels.is_empty()
    }
}
```

**Step 3: Wire up**

在 `app_root.rs` 中，找到创建或持有 `event_bus` 的位置。在 `attach_runtime` 或应用启动后、Event Bus 订阅开始前，添加一次性启动 RemoteChannelPublisher 的逻辑。建议在 `ensure_event_bus_subscription` 内部，在 spawn 主循环之前：

```rust
// One-time: start remote publisher if not already started
// Use Option<()> or similar to avoid multiple spawns
if self.remote_publisher_started.get_or_insert(()) == &() {
    let remote_rx = self.event_bus.subscribe();
    let (config, secrets) = (Config::load().unwrap_or_default(), crate::remotes::Secrets::load().unwrap_or_default());
    let publisher = crate::remotes::RemoteChannelPublisher::from_config(&config, &secrets);
    if publisher.has_channels() {
        publisher.run(remote_rx);
    }
}
```

需要在 AppRoot 增加 `remote_publisher_started: std::cell::Cell<bool>` 或使用 `Once`。为简化，可在 `ensure_event_bus_subscription` 中检查一个 `RefCell<bool>`，首次为 false 时设为 true 并 spawn。

**Step 4: Add RemoteChannelPublisher::has_channels**

已在 Step 2 完成。

**Step 5: Verify**

```bash
cargo run
```

手动验证：在 `~/.config/pmux/secrets.json` 配置有效的 Discord webhook，`config.json` 中 `remote_channels.discord.enabled: true`，触发一次 Agent 状态变化（如 Waiting），检查 Discord 是否收到消息。

**Step 6: Commit**

```bash
git add src/ui/app_root.rs src/remotes/publisher.rs
git commit -m "feat(remotes): wire RemoteChannelPublisher to Event Bus"
```

---

## Phase B 提纲（后续）

- Task B1: KOOK Adapter（`src/remotes/kook.rs`），KOOK Bot API 发消息
- Task B2: 事件富化 — 在 `AgentStateChange`、`Notification` 中增加 `workspace: Option<String>`、`worktree: Option<String>`，StatusPublisher 在 `attach_runtime` 时注入，Publisher 优先使用

## Phase C 提纲（后续）

- Task C1: Discord Bot 斜杠命令注册
- Task C2: 命令解析 `status`、`restart <worktree>`，调用 Runtime API（需 Runtime 暴露 query/restart）

---

## 文件变更汇总

| 文件 | 变更 |
|------|------|
| `Cargo.toml` | 添加 reqwest |
| `src/lib.rs` | 添加 `pub mod remotes` |
| `src/remotes/mod.rs` | 新建 |
| `src/remotes/channel.rs` | RemoteMessage, RemoteChannel trait |
| `src/remotes/secrets.rs` | Secrets 加载 |
| `src/remotes/publisher.rs` | RemoteChannelPublisher |
| `src/remotes/discord.rs` | Discord webhook 发送 |
| `src/config.rs` | remote_channels 配置 |
| `src/ui/app_root.rs` | 启动时 spawn RemoteChannelPublisher |

---

## 索引

本计划完成后，在 `docs/plans/README.md` 的「Remote Channels」小节添加本文件链接。
