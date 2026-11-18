//! Local RPC between the tray daemon and the UI process (Unix sockets, std only).
//!
//! Newline-delimited JSON on one stream per UI process:
//!
//! * server, first line: `{"event":"hello","payload":{"version":"…"}}`
//! * client request: `{"id":1,"cmd":"save_profile","args":{…}}`
//! * server reply: `{"id":1,"ok":…}` or `{"id":1,"err":"message"}`
//! * server push at any time: `{"event":"engine-state"|"app-state","payload":…}`

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

pub const EVENT_HELLO: &str = "hello";
pub const EVENT_ENGINE_STATE: &str = "engine-state";
pub const EVENT_APP_STATE: &str = "app-state";

/// A request line longer than this is treated as garbage and drops the connection.
const MAX_LINE: u64 = 8 << 20;
const HELLO_TIMEOUT: Duration = Duration::from_secs(3);
const CALL_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
/// How long a new daemon waits for a dying one to let go of the socket.
const TAKEOVER_WAIT: Duration = Duration::from_secs(2);
/// How long the UI waits for a freshly started daemon.
const STARTUP_WAIT: Duration = Duration::from_secs(3);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// `$DECOY_SOCKET`, else `$XDG_RUNTIME_DIR/decoy.sock`, else `/tmp/decoy-<uid>/decoy.sock`.
pub fn socket_path() -> PathBuf {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    if let Some(p) = env("DECOY_SOCKET") {
        return PathBuf::from(p);
    }
    if let Some(dir) = env("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("decoy.sock");
    }
    fallback_dir().join("decoy.sock")
}

fn uid() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

/// Where the socket lives when there is no `XDG_RUNTIME_DIR`.
fn fallback_dir() -> PathBuf {
    PathBuf::from(format!("/tmp/decoy-{}", uid()))
}

/// `/tmp` is shared with every user of the machine, so a socket there needs a
/// directory nobody else can enter or prepare in advance: ours, a real
/// directory, and closed to group and others.
fn check_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    match std::fs::DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e),
    }
    let meta = std::fs::symlink_metadata(dir)?;
    if !meta.is_dir() || meta.uid() != uid() || meta.mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} is not a private directory of this user", dir.display()),
        ));
    }
    Ok(())
}

/// Refuse a socket in the shared fallback location unless its directory is private.
fn check_location(path: &Path) -> io::Result<()> {
    match path.parent() {
        Some(dir) if dir == fallback_dir() => check_private_dir(dir),
        _ => Ok(()),
    }
}

// ===================================================================== server

/// What the server asks of the application.
pub trait Service: Send + Sync + 'static {
    /// Handle one command. `Err` becomes an `err` reply.
    fn call(&self, cmd: &str, args: Value) -> Result<Value, String>;
    /// Payload of the hello event every new connection receives first.
    fn hello(&self) -> Value;
    /// Runs on the connection thread after the reply to `cmd` was written.
    fn after_reply(&self, _cmd: &str) {}
}

#[derive(Debug)]
pub enum BindError {
    /// Another daemon owns the socket.
    AlreadyRunning,
    Io(io::Error),
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindError::AlreadyRunning => write!(f, "another Decoy daemon is already running"),
            BindError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<io::Error> for BindError {
    fn from(e: io::Error) -> Self {
        BindError::Io(e)
    }
}

struct Peer {
    id: u64,
    writer: Arc<Mutex<UnixStream>>,
}

struct ServerInner {
    path: PathBuf,
    listener: Mutex<Option<UnixListener>>,
    /// Held for the life of the daemon; the OS drops it when the process dies.
    lock_file: Mutex<Option<File>>,
    closed: AtomicBool,
    peers: Mutex<Vec<Peer>>,
    next_peer: AtomicU64,
}

/// Cheap to clone; all clones share one socket.
#[derive(Clone)]
pub struct Server(Arc<ServerInner>);

fn try_flock(file: &File) -> bool {
    // SAFETY: the descriptor is valid for as long as `file` lives.
    unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 }
}

impl Server {
    /// Take ownership of `path`. A lock file next to it makes sure only one
    /// daemon runs; with the lock held, a leftover socket file is stale and is
    /// replaced. The socket ends up with mode 0600.
    pub fn bind(path: &Path) -> Result<Server, BindError> {
        check_location(path)?;
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let mut lock_name = path.as_os_str().to_owned();
        lock_name.push(".lock");
        let lock_file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .open(PathBuf::from(lock_name))?;

        let end = Instant::now() + TAKEOVER_WAIT;
        while !try_flock(&lock_file) {
            // Locked but not answering: the owner is shutting down, wait a bit.
            if UnixStream::connect(path).is_ok() || Instant::now() >= end {
                return Err(BindError::AlreadyRunning);
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        // No umask juggling (it is process-wide). Connecting needs write
        // permission, which the default umask never grants to others, so the
        // short gap before the chmod below is harmless.
        let listener = UnixListener::bind(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;

        Ok(Server(Arc::new(ServerInner {
            path: path.to_path_buf(),
            listener: Mutex::new(Some(listener)),
            lock_file: Mutex::new(Some(lock_file)),
            closed: AtomicBool::new(false),
            peers: Mutex::new(Vec::new()),
            next_peer: AtomicU64::new(1),
        })))
    }

    pub fn path(&self) -> &Path {
        &self.0.path
    }

    /// Start accepting connections on a background thread.
    pub fn serve(&self, service: Arc<dyn Service>) {
        let Some(listener) = lock(&self.0.listener).take() else {
            return;
        };
        let inner = self.0.clone();
        let _ = std::thread::Builder::new()
            .name("decoy-rpc-accept".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if inner.closed.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let inner = inner.clone();
                    let service = service.clone();
                    let _ = std::thread::Builder::new()
                        .name("decoy-rpc-conn".into())
                        .spawn(move || serve_connection(inner, service, stream));
                }
            });
    }

    /// Push an event to every connected client.
    pub fn broadcast(&self, event: &str, payload: &impl Serialize) {
        let Ok(payload) = serde_json::to_value(payload) else {
            return;
        };
        let line = encode(&json!({ "event": event, "payload": payload }));
        let peers: Vec<(u64, Arc<Mutex<UnixStream>>)> = lock(&self.0.peers)
            .iter()
            .map(|p| (p.id, p.writer.clone()))
            .collect();
        let mut dead = Vec::new();
        for (id, writer) in peers {
            if lock(&writer).write_all(&line).is_err() {
                dead.push(id);
            }
        }
        if !dead.is_empty() {
            lock(&self.0.peers).retain(|p| !dead.contains(&p.id));
        }
    }

    pub fn client_count(&self) -> usize {
        lock(&self.0.peers).len()
    }

    /// Stop accepting, remove the socket file and release the lock so another
    /// daemon can take over. Existing connections are left alone.
    pub fn close(&self) {
        self.0.closed.store(true, Ordering::SeqCst);
        // Wake the accept thread so it notices and lets go of the listener.
        let _ = UnixStream::connect(&self.0.path);
        let _ = std::fs::remove_file(&self.0.path);
        lock(&self.0.lock_file).take();
    }
}

impl Drop for ServerInner {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn encode(v: &Value) -> Vec<u8> {
    let mut line = serde_json::to_vec(v).unwrap_or_default();
    line.push(b'\n');
    line
}

fn serve_connection(inner: Arc<ServerInner>, service: Arc<dyn Service>, stream: UnixStream) {
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let writer = Arc::new(Mutex::new(stream));
    let id = inner.next_peer.fetch_add(1, Ordering::SeqCst);
    {
        // Register and greet under the writer lock so a broadcast cannot
        // overtake the hello.
        let mut w = lock(&writer);
        lock(&inner.peers).push(Peer {
            id,
            writer: writer.clone(),
        });
        let hello = encode(&json!({ "event": EVENT_HELLO, "payload": service.hello() }));
        if w.write_all(&hello).is_err() {
            drop(w);
            lock(&inner.peers).retain(|p| p.id != id);
            return;
        }
    }

    let mut reader = BufReader::new(read_half);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.by_ref().take(MAX_LINE).read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) if !line.ends_with('\n') => break,
            Ok(_) => {}
        }
        let (reply, cmd) = match serde_json::from_str::<Value>(&line) {
            Ok(req) => {
                let rid = req.get("id").and_then(Value::as_u64).unwrap_or(0);
                let cmd = req.get("cmd").and_then(Value::as_str).unwrap_or("");
                let args = req.get("args").cloned().unwrap_or(Value::Null);
                let reply = match service.call(cmd, args) {
                    Ok(v) => json!({ "id": rid, "ok": v }),
                    Err(e) => json!({ "id": rid, "err": e }),
                };
                (reply, cmd.to_string())
            }
            Err(_) => (json!({ "id": 0, "err": "Bad request" }), String::new()),
        };
        if lock(&writer).write_all(&encode(&reply)).is_err() {
            break;
        }
        if !cmd.is_empty() {
            service.after_reply(&cmd);
        }
    }
    lock(&inner.peers).retain(|p| p.id != id);
}

// ===================================================================== client

type Pending = Mutex<HashMap<u64, Sender<Result<Value, String>>>>;

struct ClientInner {
    writer: Mutex<UnixStream>,
    pending: Pending,
    next_id: AtomicU64,
    closed: AtomicBool,
}

/// A connection that has read the daemon's hello but is not yet being served.
pub struct Connection {
    stream: UnixStream,
    reader: BufReader<UnixStream>,
    version: String,
}

impl Connection {
    /// Connect and read the hello.
    pub fn open(path: &Path) -> io::Result<Connection> {
        check_location(path)?;
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(HELLO_TIMEOUT))?;
        stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut line = String::new();
        reader.by_ref().take(MAX_LINE).read_line(&mut line)?;
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
        let hello: Value = serde_json::from_str(&line).map_err(|_| bad("bad hello"))?;
        if hello.get("event").and_then(Value::as_str) != Some(EVENT_HELLO) {
            return Err(bad("not a Decoy daemon"));
        }
        let version = hello["payload"]["version"]
            .as_str()
            .ok_or_else(|| bad("hello without version"))?
            .to_string();
        stream.set_read_timeout(None)?;
        Ok(Connection {
            stream,
            reader,
            version,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Start the reader thread. `on_event` gets pushed events; `on_close` runs
    /// once when the connection ends for any reason.
    pub fn start(
        self,
        on_event: impl Fn(&str, Value) + Send + 'static,
        on_close: impl FnOnce() + Send + 'static,
    ) -> Client {
        let inner = Arc::new(ClientInner {
            writer: Mutex::new(self.stream),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            closed: AtomicBool::new(false),
        });
        let weak = inner.clone();
        let mut reader = self.reader;
        let _ = std::thread::Builder::new()
            .name("decoy-rpc-client".into())
            .spawn(move || {
                let mut line = String::new();
                loop {
                    line.clear();
                    match reader.by_ref().take(MAX_LINE).read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) if !line.ends_with('\n') => break,
                        Ok(_) => {}
                    }
                    let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    if let Some(event) = msg.get("event").and_then(Value::as_str) {
                        on_event(event, msg.get("payload").cloned().unwrap_or(Value::Null));
                    } else if let Some(id) = msg.get("id").and_then(Value::as_u64) {
                        if let Some(tx) = lock(&weak.pending).remove(&id) {
                            let result = match msg.get("err").and_then(Value::as_str) {
                                Some(e) => Err(e.to_string()),
                                None => Ok(msg.get("ok").cloned().unwrap_or(Value::Null)),
                            };
                            let _ = tx.send(result);
                        }
                    }
                }
                weak.closed.store(true, Ordering::SeqCst);
                for (_, tx) in lock(&weak.pending).drain() {
                    let _ = tx.send(Err(MSG_GONE.into()));
                }
                on_close();
            });
        Client(inner)
    }
}

const MSG_GONE: &str = "The Decoy background service is not running";

#[derive(Clone)]
pub struct Client(Arc<ClientInner>);

impl Client {
    pub fn call_raw(&self, cmd: &str, args: Value) -> Result<Value, String> {
        let inner = &self.0;
        if inner.closed.load(Ordering::SeqCst) {
            return Err(MSG_GONE.into());
        }
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = channel();
        lock(&inner.pending).insert(id, tx);
        let line = encode(&json!({ "id": id, "cmd": cmd, "args": args }));
        // The daemon drops a connection that sends a longer line.
        if line.len() as u64 >= MAX_LINE {
            lock(&inner.pending).remove(&id);
            return Err("That is too much data for one request".into());
        }
        if lock(&inner.writer).write_all(&line).is_err() {
            lock(&inner.pending).remove(&id);
            return Err(MSG_GONE.into());
        }
        match rx.recv_timeout(CALL_TIMEOUT) {
            Ok(r) => r,
            Err(_) => {
                lock(&inner.pending).remove(&id);
                Err("The Decoy background service did not answer in time".into())
            }
        }
    }

    pub fn call<T: DeserializeOwned>(&self, cmd: &str, args: Value) -> Result<T, String> {
        serde_json::from_value(self.call_raw(cmd, args)?)
            .map_err(|e| format!("Unexpected answer from the Decoy background service: {e}"))
    }

    pub fn is_closed(&self) -> bool {
        self.0.closed.load(Ordering::SeqCst)
    }
}

// ============================================================ starting daemons

/// Tells the daemon which command opens the window (see [`spawn_detached_with`]).
pub const ENV_UI: &str = "DECOY_UI";

/// Start `exe` fully detached: its own session, no terminal, no stdio.
pub fn spawn_detached(exe: &Path) -> io::Result<()> {
    spawn_detached_with(exe, &[])
}

/// [`spawn_detached`] with extra environment variables for the child.
pub fn spawn_detached_with(exe: &Path, env: &[(&str, &std::ffi::OsStr)]) -> io::Result<()> {
    let mut cmd = Command::new(exe);
    cmd.envs(env.iter().copied());
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: setsid is async-signal-safe and the closure does nothing else.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut child = cmd.spawn()?;
    // Reap it when it exits (a second daemon exits at once) so no zombie stays.
    let _ = std::thread::Builder::new()
        .name("decoy-reap".into())
        .spawn(move || {
            let _ = child.wait();
        });
    Ok(())
}

/// Ask a running daemon to quit and wait until it stopped answering.
fn stop_daemon(path: &Path) -> Result<(), String> {
    if let Ok(conn) = Connection::open(path) {
        let client = conn.start(|_, _| {}, || {});
        let _ = client.call_raw("quit", Value::Null);
    }
    let end = Instant::now() + STARTUP_WAIT;
    while Connection::open(path).is_ok() {
        if Instant::now() >= end {
            return Err("The old Decoy background service did not stop".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

/// Connect to the daemon at `path`, starting it with `spawn` when it is not
/// running. A daemon of another version is asked to quit and replaced.
pub fn ensure_daemon(
    path: &Path,
    version: &str,
    spawn: &dyn Fn() -> io::Result<()>,
    on_event: impl Fn(&str, Value) + Send + 'static,
    on_close: impl FnOnce() + Send + 'static,
) -> Result<Client, String> {
    let spawn =
        || spawn().map_err(|e| format!("Could not start the Decoy background service: {e}"));
    match Connection::open(path) {
        Ok(conn) if conn.version() == version => {
            return Ok(conn.start(on_event, on_close));
        }
        Ok(conn) => {
            drop(conn);
            stop_daemon(path)?;
            spawn()?;
        }
        Err(_) => spawn()?,
    }
    let end = Instant::now() + STARTUP_WAIT;
    loop {
        match Connection::open(path) {
            Ok(conn) if conn.version() == version => return Ok(conn.start(on_event, on_close)),
            Ok(conn) => {
                return Err(format!(
                    "The Decoy background service has version {} but this app is {version}",
                    conn.version()
                ))
            }
            Err(_) if Instant::now() < end => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => return Err("The Decoy background service did not start".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn fallback_socket_needs_a_private_directory() {
        let tmp = tempfile::tempdir().unwrap();
        // Created by us: private.
        let dir = tmp.path().join("fresh");
        check_private_dir(&dir).unwrap();
        assert_eq!(
            std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        // Open to others: refused.
        let open = tmp.path().join("open");
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(check_private_dir(&open).is_err());
        // Not a directory at all (a link someone left there): refused.
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&dir, &link).unwrap();
        assert!(check_private_dir(&link).is_err());
    }

    #[test]
    fn other_locations_are_left_alone() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        check_location(&tmp.path().join("decoy.sock")).unwrap();
    }
}
