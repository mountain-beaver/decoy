//! Crate-wide error type.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No Discord IPC socket could be opened.
    #[error("Discord is not running")]
    DiscordNotRunning,
    /// Discord closed the handshake (for example an invalid application id).
    #[error("Discord rejected the connection ({code}): {message}")]
    Rejected { code: i64, message: String },
    /// Discord answered a command with an error event.
    #[error("Discord rejected the update ({code}): {message}")]
    Command { code: i64, message: String },
    /// The connection was closed or broke.
    #[error("Lost connection to Discord")]
    Disconnected,
    /// The peer did not answer within the read timeout.
    #[error("Discord did not answer in time")]
    Timeout,
    /// The peer sent something that is not valid IPC.
    #[error("Unexpected data from Discord: {0}")]
    Protocol(String),
    /// A public HTTP lookup returned 404, or a profile id is unknown.
    #[error("Not found")]
    NotFound,
    /// Network or HTTP failure.
    #[error("Network error: {0}")]
    Network(String),
    /// Input that cannot be used (for example a malformed profile file).
    #[error("Invalid data: {0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Map an I/O error to `Disconnected` / `Timeout` where it means that.
pub(crate) fn map_io(e: io::Error) -> Error {
    use io::ErrorKind::*;
    match e.kind() {
        UnexpectedEof | BrokenPipe | ConnectionReset | ConnectionAborted | NotConnected => {
            Error::Disconnected
        }
        WouldBlock | TimedOut => Error::Timeout,
        _ => Error::Io(e),
    }
}
