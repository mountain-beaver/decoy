//! Core library of Decoy: Discord IPC, presence model, profile store and a
//! background engine. Blocking I/O on std threads only.

pub mod error;
pub mod presence;
mod util;

pub use error::{Error, Result};
pub use presence::{
    ActivityKind, Button, Field, Image, Issue, Party, Presence, StatusDisplay, TimeContext,
    TimestampMode,
};
