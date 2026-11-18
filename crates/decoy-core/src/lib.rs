//! Core library of Decoy: Discord IPC, presence model, profile store and a
//! background engine. Blocking I/O on std threads only.

pub mod discord_api;
pub mod engine;
pub mod error;
pub mod games;
mod http;
pub mod ipc;
pub mod presence;
pub mod store;
mod util;

#[cfg(all(test, unix))]
mod testutil;

pub use discord_api::{AppInfo, Asset};
pub use engine::{Engine, EngineState};
pub use error::{Error, Result};
pub use games::{Game, GameIndex};
pub use ipc::{Client, ConnectOptions, DiscordUser};
pub use presence::{
    ActivityKind, Button, Field, Image, Issue, Party, Presence, StatusDisplay, TimeContext,
    TimestampMode,
};
pub use store::{Profile, Settings, Store};
