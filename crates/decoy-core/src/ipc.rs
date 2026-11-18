//! Discord local IPC: framing, transport and the handshake / SET_ACTIVITY exchange.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{map_io, Error, Result};

pub const OP_HANDSHAKE: u32 = 0;
pub const OP_FRAME: u32 = 1;
pub const OP_CLOSE: u32 = 2;
pub const OP_PING: u32 = 3;
pub const OP_PONG: u32 = 4;

/// Largest payload accepted or sent.
const MAX_PAYLOAD: usize = 1 << 20;

#[cfg(unix)]
type Stream = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Stream = crate::pipe::PipeStream;

#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Replaces the default candidate socket list (used by tests).
    pub paths: Option<Vec<PathBuf>>,
    /// Read/write timeout on the socket (Unix only).
    pub timeout: Duration,
    /// The current local UTC offset in seconds. With it the "time of day"
    /// timer follows midnight and clock changes; without it the offset given
    /// to `Engine::apply` is used for as long as that presence is shown.
    pub local_offset: Option<fn() -> i32>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            paths: None,
            timeout: Duration::from_secs(10),
            local_offset: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// Write one frame: opcode and length as u32 LE, then the payload.
pub fn write_frame<W: Write>(w: &mut W, op: u32, payload: &[u8]) -> Result<()> {
    if payload.len() > MAX_PAYLOAD {
        return Err(Error::Protocol("frame is too large".into()));
    }
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend_from_slice(&op.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload);
    w.write_all(&buf).map_err(map_io)?;
    w.flush().map_err(map_io)
}

/// Read one frame, returning its opcode and payload.
pub fn read_frame<R: Read>(r: &mut R) -> Result<(u32, Vec<u8>)> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head).map_err(map_io)?;
    let op = u32::from_le_bytes(head[..4].try_into().unwrap());
    let len = u32::from_le_bytes(head[4..].try_into().unwrap()) as usize;
    if len > MAX_PAYLOAD {
        return Err(Error::Protocol(format!(
            "frame of {len} bytes is too large"
        )));
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload).map_err(map_io)?;
    Ok((op, payload))
}

/// Candidate socket paths in the order they are tried.
#[cfg(unix)]
pub fn default_paths() -> Vec<PathBuf> {
    const SUBDIRS: [&str; 8] = [
        "",
        "app/com.discordapp.Discord/",
        "app/com.discordapp.DiscordCanary/",
        "app/dev.vencord.Vesktop/",
        ".flatpak/com.discordapp.Discord/xdg-run/",
        ".flatpak/dev.vencord.Vesktop/xdg-run/",
        "snap.discord/",
        "snap.discord-canary/",
    ];
    let mut bases: Vec<PathBuf> = Vec::new();
    for var in ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"] {
        if let Some(v) = std::env::var_os(var).filter(|v| !v.is_empty()) {
            let p = PathBuf::from(v);
            if !bases.contains(&p) {
                bases.push(p);
            }
        }
    }
    let tmp = PathBuf::from("/tmp");
    if !bases.contains(&tmp) {
        bases.push(tmp);
    }
    let mut out = Vec::new();
    for base in &bases {
        for sub in SUBDIRS {
            for i in 0..10 {
                out.push(base.join(format!("{sub}discord-ipc-{i}")));
            }
        }
    }
    out
}

#[cfg(windows)]
pub fn default_paths() -> Vec<PathBuf> {
    (0..10)
        .map(|i| PathBuf::from(format!(r"\\.\pipe\discord-ipc-{i}")))
        .collect()
}

#[cfg(unix)]
fn open_stream(path: &std::path::Path, timeout: Duration) -> std::io::Result<Stream> {
    let s = Stream::connect(path)?;
    s.set_read_timeout(Some(timeout))?;
    s.set_write_timeout(Some(timeout))?;
    Ok(s)
}

#[cfg(windows)]
fn open_stream(path: &std::path::Path, timeout: Duration) -> std::io::Result<Stream> {
    Stream::open(path, timeout)
}

static NONCE: AtomicU64 = AtomicU64::new(0);

fn new_nonce() -> String {
    format!(
        "decoy-{:x}-{}",
        crate::util::now_ms(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    )
}

/// Parse `{code, message}` from a CLOSE payload or an error `data` object.
fn code_message(v: &Value) -> (i64, String) {
    (
        v["code"].as_i64().unwrap_or(0),
        v["message"].as_str().unwrap_or("Unknown error").to_string(),
    )
}

/// An open, handshaken IPC connection.
pub struct Client {
    stream: Stream,
}

impl Client {
    /// Connect to the first reachable Discord socket and perform the handshake.
    pub fn connect(client_id: &str, opts: &ConnectOptions) -> Result<(Client, DiscordUser)> {
        let candidates = opts.paths.clone().unwrap_or_else(default_paths);
        let mut last_err = None;
        for path in candidates {
            let Ok(stream) = open_stream(&path, opts.timeout) else {
                continue;
            };
            let mut client = Client { stream };
            match client.handshake(client_id) {
                Ok(user) => return Ok((client, user)),
                Err(e @ Error::Rejected { .. }) => return Err(e),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or(Error::DiscordNotRunning))
    }

    fn send(&mut self, op: u32, v: &Value) -> Result<()> {
        write_frame(&mut self.stream, op, v.to_string().as_bytes())
    }

    fn handshake(&mut self, client_id: &str) -> Result<DiscordUser> {
        self.send(OP_HANDSHAKE, &json!({ "v": 1, "client_id": client_id }))?;
        loop {
            let (op, payload) = read_frame(&mut self.stream)?;
            match op {
                OP_CLOSE => {
                    let (code, message) = code_message(&parse(&payload)?);
                    return Err(Error::Rejected { code, message });
                }
                OP_PING => write_frame(&mut self.stream, OP_PONG, &payload)?,
                OP_FRAME => {
                    let v = parse(&payload)?;
                    match v["evt"].as_str() {
                        Some("READY") => {
                            return serde_json::from_value(v["data"]["user"].clone())
                                .map_err(|e| Error::Protocol(format!("bad READY payload: {e}")));
                        }
                        Some("ERROR") => {
                            let (code, message) = code_message(&v["data"]);
                            return Err(Error::Rejected { code, message });
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// Set (`Some`) or clear (`None`) the activity and wait for Discord's answer.
    pub fn set_activity(&mut self, activity: Option<&Value>) -> Result<()> {
        let nonce = new_nonce();
        self.send(
            OP_FRAME,
            &json!({
                "cmd": "SET_ACTIVITY",
                "args": { "pid": std::process::id(), "activity": activity },
                "nonce": nonce,
            }),
        )?;
        loop {
            let (op, payload) = read_frame(&mut self.stream)?;
            match op {
                OP_PING => write_frame(&mut self.stream, OP_PONG, &payload)?,
                OP_CLOSE => return Err(Error::Disconnected),
                OP_FRAME => {
                    let v = parse(&payload)?;
                    if v["nonce"].as_str() != Some(nonce.as_str()) {
                        continue;
                    }
                    if v["evt"].as_str() == Some("ERROR") {
                        let (code, message) = code_message(&v["data"]);
                        return Err(Error::Command { code, message });
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    /// Cheap liveness check: fails with `Disconnected` when the peer is gone.
    /// Answers a pending PING if Discord sent one.
    #[cfg(unix)]
    pub fn poll(&mut self) -> Result<()> {
        self.stream.set_nonblocking(true)?;
        let mut byte = [0u8; 1];
        let got = self.stream.read(&mut byte);
        self.stream.set_nonblocking(false)?;
        match got {
            Ok(0) => Err(Error::Disconnected),
            Ok(_) => {
                // One byte of an unsolicited frame was consumed; read the rest after it.
                // A partial frame is bounded by the read timeout.
                let (op, payload) = read_frame(&mut (&byte[..]).chain(&mut self.stream))?;
                match op {
                    OP_PING => write_frame(&mut self.stream, OP_PONG, &payload),
                    OP_CLOSE => Err(Error::Disconnected),
                    _ => Ok(()),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(map_io(e)),
        }
    }

    /// Cheap liveness check: fails with `Disconnected` when the peer is gone.
    /// Answers a pending PING if Discord sent one.
    #[cfg(windows)]
    pub fn poll(&mut self) -> Result<()> {
        loop {
            match self.stream.available() {
                Err(_) => return Err(Error::Disconnected),
                Ok(0) => return Ok(()),
                Ok(_) => {}
            }
            // An unsolicited frame is waiting. A partial one is bounded by the read timeout.
            let (op, payload) = read_frame(&mut self.stream)?;
            match op {
                OP_PING => write_frame(&mut self.stream, OP_PONG, &payload)?,
                OP_CLOSE => return Err(Error::Disconnected),
                _ => {}
            }
        }
    }
}

fn parse(payload: &[u8]) -> Result<Value> {
    serde_json::from_slice(payload).map_err(|e| Error::Protocol(format!("invalid JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{endpoint, Mock, MockConfig, USER_ID};
    use std::io::Cursor;

    const ID: &str = "123456789012345678";

    fn opts(m: &Mock) -> ConnectOptions {
        ConnectOptions {
            paths: Some(vec![m.path.clone()]),
            timeout: Duration::from_secs(5),
            local_offset: None,
        }
    }

    #[test]
    fn framing_roundtrip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, OP_FRAME, br#"{"a":1}"#).unwrap();
        assert_eq!(&buf[..8], &[1, 0, 0, 0, 7, 0, 0, 0]);
        let (op, payload) = read_frame(&mut Cursor::new(buf)).unwrap();
        assert_eq!(
            (op, payload.as_slice()),
            (OP_FRAME, br#"{"a":1}"#.as_slice())
        );
    }

    #[test]
    fn framing_rejects_oversized_and_truncated() {
        let mut head = Vec::new();
        head.extend_from_slice(&1u32.to_le_bytes());
        head.extend_from_slice(&(MAX_PAYLOAD as u32 + 1).to_le_bytes());
        assert!(matches!(
            read_frame(&mut Cursor::new(head)),
            Err(Error::Protocol(_))
        ));
        let mut short = Vec::new();
        write_frame(&mut short, OP_FRAME, b"hello").unwrap();
        short.truncate(10);
        assert!(matches!(
            read_frame(&mut Cursor::new(short)),
            Err(Error::Disconnected)
        ));
        assert!(write_frame(&mut Vec::new(), OP_FRAME, &vec![0; MAX_PAYLOAD + 1]).is_err());
    }

    #[test]
    fn connect_set_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (mut client, user) = Client::connect(ID, &opts(&mock)).unwrap();
        assert_eq!(user.id, USER_ID);
        assert_eq!(user.global_name.as_deref(), Some("Tester"));
        assert_eq!(user.avatar, None);

        let hs = mock.recv();
        assert_eq!(hs.op, OP_HANDSHAKE);
        assert_eq!(hs.value, json!({ "v": 1, "client_id": ID }));

        let activity = json!({ "type": 0, "details": "Hi", "instance": false });
        client.set_activity(Some(&activity)).unwrap();
        let sent = mock.recv_set_activity().value;
        assert_eq!(sent["args"]["pid"], std::process::id());
        assert_eq!(sent["args"]["activity"], activity);
        assert!(sent["nonce"].as_str().unwrap().starts_with("decoy-"));

        client.set_activity(None).unwrap();
        let sent = mock.recv_set_activity().value;
        assert_eq!(
            sent["args"],
            json!({ "pid": std::process::id(), "activity": null })
        );
    }

    #[test]
    fn rejected_client_id() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(
            dir.path(),
            MockConfig {
                close_at_handshake: Some((4000, "Invalid Client ID".into())),
                ..Default::default()
            },
        );
        match Client::connect(ID, &opts(&mock)) {
            Err(Error::Rejected { code, message }) => {
                assert_eq!(code, 4000);
                assert_eq!(message, "Invalid Client ID");
            }
            other => panic!("unexpected {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn command_error_is_typed() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(
            dir.path(),
            MockConfig {
                error_on_set: Some((4000, "Bad activity".into())),
                ..Default::default()
            },
        );
        let (mut client, _) = Client::connect(ID, &opts(&mock)).unwrap();
        match client.set_activity(Some(&json!({}))) {
            Err(Error::Command { code, message }) => {
                assert_eq!((code, message.as_str()), (4000, "Bad activity"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn ping_is_answered_and_unrelated_frames_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(
            dir.path(),
            MockConfig {
                ping_before_reply: true,
                ..Default::default()
            },
        );
        let (mut client, _) = Client::connect(ID, &opts(&mock)).unwrap();
        client.set_activity(Some(&json!({ "type": 0 }))).unwrap();
        let _ = mock.recv(); // handshake
        let _ = mock.recv_set_activity();
        let pong = mock.recv();
        assert_eq!(pong.op, OP_PONG);
        assert_eq!(pong.value, json!({ "hello": 1 }));
    }

    #[test]
    fn dropped_connection_is_disconnected() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(
            dir.path(),
            MockConfig {
                drop_on_set: true,
                ..Default::default()
            },
        );
        let (mut client, _) = Client::connect(ID, &opts(&mock)).unwrap();
        assert!(matches!(
            client.set_activity(Some(&json!({}))),
            Err(Error::Disconnected)
        ));
    }

    #[test]
    fn poll_detects_closed_peer() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (mut client, _) = Client::connect(ID, &opts(&mock)).unwrap();
        client.poll().unwrap();
        mock.kick();
        // The shutdown is asynchronous from the client's point of view.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            match client.poll() {
                Err(Error::Disconnected) => break,
                Ok(()) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn no_socket_means_discord_not_running() {
        let o = ConnectOptions {
            paths: Some(vec![]),
            ..Default::default()
        };
        assert!(matches!(
            Client::connect(ID, &o),
            Err(Error::DiscordNotRunning)
        ));
        let dir = tempfile::tempdir().unwrap();
        let o = ConnectOptions {
            paths: Some(vec![endpoint(dir.path(), "discord-ipc-0")]),
            ..Default::default()
        };
        assert!(matches!(
            Client::connect(ID, &o),
            Err(Error::DiscordNotRunning)
        ));
    }

    #[test]
    fn falls_through_to_the_next_candidate() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let o = ConnectOptions {
            paths: Some(vec![endpoint(dir.path(), "missing"), mock.path.clone()]),
            ..Default::default()
        };
        assert!(Client::connect(ID, &o).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn default_candidates_are_the_ten_discord_pipes() {
        let paths = default_paths();
        assert_eq!(paths.len(), 10);
        assert_eq!(paths[0], PathBuf::from(r"\\.\pipe\discord-ipc-0"));
        assert_eq!(paths[9], PathBuf::from(r"\\.\pipe\discord-ipc-9"));
    }

    #[cfg(unix)]
    #[test]
    fn default_candidates_cover_all_bases_and_subdirs() {
        let paths = default_paths();
        assert!(paths.iter().any(|p| p.ends_with("discord-ipc-9")));
        assert!(paths
            .iter()
            .any(|p| p.ends_with("app/dev.vencord.Vesktop/discord-ipc-0")));
        assert!(paths.iter().any(|p| p.starts_with("/tmp")));
    }
}
