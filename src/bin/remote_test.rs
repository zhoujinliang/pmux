//! One-off test: load KOOK config, discover first channel, send test message.
//! Usage: cargo run --bin pmux-remote-test

use pmux::remotes::kook::KookBot;
use pmux::remotes::secrets::Secrets;
use std::path::PathBuf;

fn main() {
    let secrets_path = dirs::config_dir()
        .map(|d| d.join("pmux").join("secrets.json"))
        .unwrap_or_else(|| PathBuf::from("secrets.json"));

    let secrets = match Secrets::load_from_path(&secrets_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to load secrets from {:?}: {}", secrets_path, e);
            std::process::exit(1);
        }
    };

    let token = match &secrets.remote_channels.kook.bot_token {
        Some(t) if !t.is_empty() => t.clone(),
        _ => {
            eprintln!("KOOK bot_token not found in secrets. Add to {:?}", secrets_path);
            std::process::exit(1);
        }
    };

    // Discover first guild and first text channel
    let client = reqwest::blocking::Client::new();
    let guild_list_url = "https://www.kookapp.cn/api/v3/guild/list";
    let resp = client
        .get(guild_list_url)
        .header("Authorization", format!("Bot {}", token))
        .send()
        .expect("guild list request failed");
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        eprintln!("Guild list failed {}: {}", status, text);
        std::process::exit(1);
    }
    let json: serde_json::Value = serde_json::from_str(&text).expect("guild list JSON parse failed");
    let code = json["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        eprintln!("KOOK guild list error {}: {}", code, json["message"].as_str().unwrap_or(""));
        std::process::exit(1);
    }
    let items = json["data"]["items"].as_array().expect("guild items");
    let guild_id = match items.first() {
        Some(g) => g["id"].as_str().expect("guild id").to_string(),
        None => {
            eprintln!("No guilds found. Invite the bot to a server first.");
            std::process::exit(1);
        }
    };
    let guild_name = items.first().and_then(|g| g["name"].as_str()).unwrap_or("");

    // Get guild view for channels (includes channels in response)
    let view_url = format!("https://www.kookapp.cn/api/v3/guild/view?guild_id={}", guild_id);
    let resp = client
        .get(&view_url)
        .header("Authorization", format!("Bot {}", token))
        .send()
        .expect("guild view request failed");
    let text = resp.text().unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&text).expect("guild view JSON parse failed");
    let code = json["code"].as_i64().unwrap_or(-1);
    if code != 0 {
        eprintln!("KOOK guild view error {}: {}", code, json["message"].as_str().unwrap_or(""));
        std::process::exit(1);
    }
    let channels = json["data"]["channels"].as_array().expect("channels");
    // type 1 = TEXT channel
    let channel = channels
        .iter()
        .find(|c| c["type"].as_i64() == Some(1) && !c["is_category"].as_bool().unwrap_or(false))
        .or_else(|| channels.first());
    let channel_id = match channel {
        Some(c) => {
            let id = &c["id"];
            // API may return number or string
            if id.is_string() {
                id.as_str().unwrap().to_string()
            } else {
                id.as_i64().unwrap().to_string()
            }
        }
        None => {
            eprintln!("No text channel found in guild");
            std::process::exit(1);
        }
    };
    let channel_name = channel.and_then(|c| c["name"].as_str()).unwrap_or("");

    println!(
        "Using guild: {} ({}) channel: {} ({})",
        guild_name, guild_id, channel_name, channel_id
    );

    let bot = KookBot::new(token, channel_id.clone()).expect("KookBot::new");
    let msg = "[pmux] 测试消息 — remote test from pmux-remote-test binary";
    match bot.send_message(msg) {
        Ok(()) => {
            println!("✓ Message sent successfully to channel {} ({})", channel_name, channel_id);
            println!("  Add to config.json: \"channel_id\": \"{}\"", channel_id);
        }
        Err(e) => {
            eprintln!("Send failed: {}", e);
            std::process::exit(1);
        }
    }
}
