//! `decoy-daemon`: the part of Decoy that lives in the tray.
//!
//! It owns the profile store, the Discord presence engine and the tray icon, and
//! talks to the UI process over a local socket. No GUI toolkit, no web view and
//! no HTTP client are linked, so it stays tiny while the window is closed.

#[cfg(target_os = "linux")]
mod tray;

#[cfg(target_os = "linux")]
fn main() {
    tray::run();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("decoy-daemon is only used on Linux; Windows and macOS run in one process.");
}
