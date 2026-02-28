//! Discord Gateway - WebSocket listener for receiving commands.
//! Parses "!pmux status", "!pmux help" etc. and replies via Bot REST API.

use crate::remotes::discord::DiscordBot;
use crate::runtime::RuntimeState;
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

/// Command prefix for text-based commands.
const CMD_PREFIX: &str = "!pmux";

/// Run the Gateway in a background thread. Connects to Discord, listens for
/// MESSAGE_CREATE in the given channel, parses commands and replies.
pub fn spawn_gateway(bot: Arc<DiscordBot>, channel_id: String) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(run_gateway(bot, channel_id));
    });
}

async fn run_gateway(bot: Arc<DiscordBot>, channel_id: String) {
    loop {
        if let Err(e) = run_gateway_inner(Arc::clone(&bot), &channel_id).await {
            eprintln!("remotes: discord gateway error: {}. Reconnecting in 5s.", e);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_gateway_inner(bot: Arc<DiscordBot>, channel_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (ws_stream, _) = connect_async(GATEWAY_URL).await?;
    let (write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(8);
    let last_seq = Arc::new(AtomicU64::new(0));
    let last_seq_clone = Arc::clone(&last_seq);

    let write_handle = tokio::spawn(async move {
        let mut w = write;
        while let Some(msg) = rx.recv().await {
            if w.send(msg).await.is_err() {
                break;
            }
        }
    });

    #[allow(unused_assignments)]
    let mut heartbeat_interval_ms: u64 = 41250;

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            _ => continue,
        };

        let payload: serde_json::Value = serde_json::from_str(&text)?;
        let op = payload["op"].as_u64().unwrap_or(0);
        let seq = payload["s"].as_u64();
        if let Some(s) = seq {
            last_seq_clone.store(s, Ordering::SeqCst);
        }

        match op {
            10 => {
                heartbeat_interval_ms = payload["d"]["heartbeat_interval"].as_u64().unwrap_or(41250);
                let identify = serde_json::json!({
                    "op": 2,
                    "d": {
                        "token": bot.bot_token(),
                        "properties": {
                            "$os": std::env::consts::OS,
                            "$browser": "pmux",
                            "$device": "pmux"
                        },
                        "intents": 33280
                    }
                });
                let _ = tx.send(Message::Text(identify.to_string())).await;

                let tx_heartbeat = tx.clone();
                let seq_heartbeat = Arc::clone(&last_seq_clone);
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(heartbeat_interval_ms));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    loop {
                        interval.tick().await;
                        let s = seq_heartbeat.load(Ordering::SeqCst);
                        let d = if s == 0 {
                            serde_json::Value::Null
                        } else {
                            serde_json::json!(s)
                        };
                        let hb = serde_json::json!({ "op": 1, "d": d });
                        if tx_heartbeat.send(Message::Text(hb.to_string())).await.is_err() {
                            break;
                        }
                    }
                });
            }
            11 => {}
            0 => {
                let t = payload["t"].as_str().unwrap_or("");
                if t == "MESSAGE_CREATE" {
                    let ch = payload["d"]["channel_id"].as_str().unwrap_or("");
                    if ch == channel_id {
                        let content = payload["d"]["content"].as_str().unwrap_or("").trim();
                        if let Some(reply) = handle_command(content) {
                            let bot_clone = Arc::clone(&bot);
                            tokio::task::spawn_blocking(move || {
                                let _ = bot_clone.send_message(&reply);
                            })
                            .await
                            .ok();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    drop(tx);
    let _ = write_handle.await;
    Ok(())
}

fn handle_command(content: &str) -> Option<String> {
    let content = content.trim();
    let rest = content.strip_prefix(CMD_PREFIX)?.trim();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let cmd = parts.first().copied().unwrap_or("");

    match cmd {
        "status" => Some(format_status()),
        "help" => Some(help_text()),
        _ if cmd.is_empty() => Some(help_text()),
        _ => None,
    }
}

fn format_status() -> String {
    match RuntimeState::load() {
        Ok(state) => {
            if state.workspaces.is_empty() {
                return "No workspaces.".to_string();
            }
            let mut lines = vec!["**pmux status**".to_string()];
            for ws in &state.workspaces {
                let name = ws.path.file_name().map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ws.path.display().to_string());
                lines.push(format!("📁 **{}**", name));
                for wt in &ws.worktrees {
                    let branch = &wt.branch;
                    let path_name = wt.path.file_name().map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| wt.path.display().to_string());
                    lines.push(format!("  └ {} (`{}`)", branch, path_name));
                }
            }
            lines.join("\n")
        }
        Err(e) => format!("Failed to load state: {}", e),
    }
}

fn help_text() -> String {
    format!(
        "**pmux bot**\n\
         `{} status` — list workspaces and worktrees\n\
         `{} help` — this message",
        CMD_PREFIX, CMD_PREFIX
    )
}
