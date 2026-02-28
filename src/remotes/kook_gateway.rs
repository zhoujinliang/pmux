//! KOOK Gateway - WebSocket listener for receiving commands.
//! Parses "!pmux status", "!pmux help" etc. and replies via Bot REST API.

use crate::remotes::channel::RemoteSendError;
use crate::remotes::kook::KookBot;
use crate::runtime::RuntimeState;

const KOOK_API_BASE: &str = "https://www.kookapp.cn/api/v3";
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Command prefixes for text-based commands (support both ! and /)
const CMD_PREFIXES: &[&str] = &["!pmux", "/pmux", "/agents"];

/// Run the Gateway in a background thread.
pub fn spawn_gateway(bot: Arc<KookBot>, channel_id: String) {
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        rt.block_on(run_gateway(bot, channel_id));
    });
}

async fn run_gateway(bot: Arc<KookBot>, channel_id: String) {
    loop {
        if let Err(e) = run_gateway_inner(Arc::clone(&bot), &channel_id).await {
            eprintln!("remotes: kook gateway error: {}. Reconnecting in 5s.", e);
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_gateway_inner(bot: Arc<KookBot>, channel_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let gateway_url = get_gateway_url(bot.bot_token())?;
    let ws_url = if gateway_url.contains('?') {
        format!("{}&token={}", gateway_url, bot.bot_token())
    } else {
        format!("{}?compress=0&token={}", gateway_url, bot.bot_token())
    };

    let (ws_stream, _) = connect_async(&ws_url).await?;
    let (write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(8);
    let last_sn = Arc::new(AtomicU64::new(0));
    let last_sn_clone = Arc::clone(&last_sn);

    let write_handle = tokio::spawn(async move {
        let mut w = write;
        while let Some(msg) = rx.recv().await {
            if w.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(t) => t,
            _ => continue,
        };

        let payload: serde_json::Value = serde_json::from_str(&text)?;
        let s = payload["s"].as_i64().unwrap_or(-1);
        let sn = payload["sn"].as_u64();

        if let Some(n) = sn {
            last_sn_clone.store(n, Ordering::SeqCst);
        }

        match s {
            1 => {
                let code = payload["d"]["code"].as_i64().unwrap_or(-1);
                if code != 0 {
                    return Err(format!("KOOK gateway auth failed: code {}", code).into());
                }
                let tx_ping = tx.clone();
                let sn_ping = Arc::clone(&last_sn_clone);
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(25));
                    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    loop {
                        interval.tick().await;
                        let n = sn_ping.load(Ordering::SeqCst);
                        let ping = serde_json::json!({ "s": 2, "sn": n });
                        if tx_ping.send(Message::Text(ping.to_string())).await.is_err() {
                            break;
                        }
                    }
                });
            }
            3 => {}
            0 => {
                let d = &payload["d"];
                let target = d["target_id"].as_str().unwrap_or("");
                let channel_type = d["channel_type"].as_str().unwrap_or("");
                let extra = &d["extra"];
                if extra["author"]["bot"].as_bool().unwrap_or(false) {
                    continue;
                }
                let msg_type = d["type"].as_i64().unwrap_or(0);
                if msg_type != 1 && msg_type != 9 {
                    continue;
                }
                // Accept: 1) channel message (target == channel_id), or 2) DM (channel_type == PERSON)
                let reply_target = if target == channel_id {
                    channel_id.to_string()
                } else if channel_type == "PERSON" {
                    // DM: reply to the user who sent the message
                    let author_id = extra["author"]["id"].as_str().unwrap_or("");
                    if author_id.is_empty() {
                        continue;
                    }
                    author_id.to_string()
                } else {
                    continue;
                };
                let content = d["content"].as_str().unwrap_or("").trim();
                if let Some(reply) = handle_command(content) {
                    let bot_clone = Arc::clone(&bot);
                    let target = reply_target.clone();
                    tokio::task::spawn_blocking(move || {
                        let _ = bot_clone.send_to_target(&target, &reply);
                    })
                    .await
                    .ok();
                }
            }
            _ => {}
        }
    }

    drop(tx);
    let _ = write_handle.await;
    Ok(())
}

fn get_gateway_url(token: &str) -> Result<String, RemoteSendError> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/gateway/index", KOOK_API_BASE))
        .header("Authorization", format!("Bot {}", token))
        .query(&[("compress", "0")])
        .send()?;
    let text = resp.text()?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| RemoteSendError::Api(format!("JSON parse: {}", e)))?;
    let code = json["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        let msg = json["message"].as_str().unwrap_or("unknown");
        return Err(RemoteSendError::Api(format!("KOOK gateway index error {}: {}", code, msg)));
    }
    let url = json["data"]["url"]
        .as_str()
        .ok_or_else(|| RemoteSendError::Api("KOOK gateway url missing".into()))?;
    Ok(url.to_string())
}


fn handle_command(content: &str) -> Option<String> {
    let content = content.trim();
    let rest = CMD_PREFIXES
        .iter()
        .find_map(|p| content.strip_prefix(p))
        .map(|s| s.trim())?;
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let cmd = parts.first().copied().unwrap_or("");
    let args = if parts.len() > 1 {
        parts[1..].join(" ")
    } else {
        String::new()
    };

    match cmd {
        "status" | "agents" | "" => Some(format_agents()),
        "send" => handle_send(&args),
        "help" => Some(help_text()),
        _ => None,
    }
}

/// Agent identifier: repo-worktree (e.g. pmux-main, pmux-feature-x)
fn repo_worktree_id(ws: &crate::runtime::WorkspaceState, wt: &crate::runtime::WorktreeState) -> String {
    let repo = ws
        .path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| ws.path.display().to_string());
    let worktree = wt
        .path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| wt.branch.clone());
    format!("{}-{}", repo, worktree)
}

fn format_agents() -> String {
    match RuntimeState::load() {
        Ok(state) => {
            if state.workspaces.is_empty() {
                return "No agents.".to_string();
            }
            let mut lines = vec!["**Agents (repo-worktree)**".to_string()];
            for ws in &state.workspaces {
                for wt in &ws.worktrees {
                    let id = repo_worktree_id(ws, wt);
                    lines.push(format!("• `{}`", id));
                }
            }
            lines.push(String::new());
            lines.push("Use `!pmux send <repo-worktree> <message>` to send input.".to_string());
            lines.join("\n")
        }
        Err(e) => format!("Failed to load state: {}", e),
    }
}

fn handle_send(args: &str) -> Option<String> {
    let mut parts = args.splitn(2, char::is_whitespace);
    let target = parts.next()?.trim();
    let text = parts.next().unwrap_or("").trim();
    if target.is_empty() || text.is_empty() {
        return Some("Usage: !pmux send <repo-worktree> <message>".to_string());
    }

    let state = match RuntimeState::load() {
        Ok(s) => s,
        Err(e) => return Some(format!("Failed to load state: {}", e)),
    };

    let (_ws, wt) = match state
        .workspaces
        .iter()
        .flat_map(|ws| ws.worktrees.iter().map(move |wt| (ws, wt)))
        .find(|(ws, wt)| repo_worktree_id(ws, wt) == target)
    {
        Some(x) => x,
        None => return Some(format!("Agent `{}` not found. Use `/agents` to list.", target)),
    };

    if wt.backend != "tmux" {
        return Some(format!(
            "Send is only supported for tmux backend. `{}` uses {}.",
            target, wt.backend
        ));
    }

    let session = &wt.backend_session_id;
    let window = &wt.backend_window_id;
    let target_spec = format!("{}:{}", session, window);

    // Normalize newlines to space for single-line send
    let text_flat: String = text.replace('\n', " ");

    let output = std::process::Command::new("tmux")
        .args(["send-keys", "-t", &target_spec, &text_flat, "Enter"])
        .output();

    match output {
        Ok(o) if o.status.success() => Some(format!("✓ Sent to `{}`", target)),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            Some(format!("Send failed: {}", err.trim()))
        }
        Err(e) => Some(format!("tmux error: {}", e)),
    }
}

fn help_text() -> String {
    "**pmux bot**
`/agents` or `!pmux status` — list all agents (repo-worktree)
`!pmux send <repo-worktree> <message>` — send input to an agent
`!pmux help` — this message"
        .to_string()
}
