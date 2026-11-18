//! Shared, GUI-free app logic of Decoy.
//!
//! * [`controller`]: profiles, settings, the "showing" flag and the presence engine.
//! * [`games`]: the Discord game list (cache, background refresh, search).
//! * [`shell`]: small helpers of the UI shell (URL check, app lookups).
//! * `rpc` (Unix): the local protocol between the Linux tray daemon and the UI.

/// Version shared by the daemon and the UI; a mismatch makes the UI replace the daemon.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod controller;
pub mod games;
pub mod shell;

pub use controller::{
    tooltip, AppState, Change, Controller, DaemonState, Notifier, MSG_FIX_FIELDS, MSG_NOT_FOUND,
    MSG_NO_PROFILE,
};
pub use games::{GameHit, Games, GamesInfo};
