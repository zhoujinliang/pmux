//! remotes - Remote notification channels (Discord, KOOK, Feishu)

pub mod channel;
pub mod discord;
pub mod discord_gateway;
pub mod feishu;
pub mod kook;
pub mod kook_gateway;
pub mod publisher;
pub mod secrets;

pub use channel::{RemoteChannel, RemoteMessage};
pub use discord::DiscordBot;
pub use feishu::FeishuBot;
pub use kook::KookBot;
pub use publisher::{RemoteChannelPublisher, spawn_remote_gateways};
pub use secrets::Secrets;
