//! Scriptable mock Discord IPC server, for tests only: a Unix socket, or a
//! named pipe on Windows.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::{json, Value};

use crate::ipc::{read_frame, write_frame, OP_CLOSE, OP_FRAME, OP_HANDSHAKE, OP_PING};

#[derive(Clone, Default)]
pub struct MockConfig {
    /// Answer the handshake with a CLOSE frame carrying this code and message.
    pub close_at_handshake: Option<(i64, String)>,
    /// Answer SET_ACTIVITY with an ERROR event.
    pub error_on_set: Option<(i64, String)>,
    /// Send a PING before answering SET_ACTIVITY.
    pub ping_before_reply: bool,
    /// Drop the connection when SET_ACTIVITY arrives, without answering.
    pub drop_on_set: bool,
}

/// A frame the server received.
#[derive(Debug, Clone)]
pub struct Received {
    pub conn: usize,
    pub op: u32,
    pub value: Value,
}

#[cfg(unix)]
mod net {
    use std::io;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};

    pub type Conn = UnixStream;
    pub struct Listener(UnixListener);

    impl Listener {
        pub fn bind(path: &Path) -> io::Result<Listener> {
            UnixListener::bind(path).map(Listener)
        }
        pub fn accept(&self) -> io::Result<Conn> {
            self.0.accept().map(|(conn, _)| conn)
        }
    }

    pub fn dup(conn: &Conn) -> Conn {
        conn.try_clone().unwrap()
    }
    pub fn close(conn: &Conn) {
        let _ = conn.shutdown(std::net::Shutdown::Both);
    }
    /// Unblock a pending `accept`.
    pub fn wake(path: &Path) {
        let _ = UnixStream::connect(path);
    }
    pub fn endpoint(dir: &Path, name: &str) -> PathBuf {
        dir.join(name)
    }
}

#[cfg(windows)]
mod net {
    use std::path::{Path, PathBuf};

    pub use crate::pipe::server::{Conn, Listener};

    pub fn dup(conn: &Conn) -> Conn {
        conn.clone()
    }
    pub fn close(conn: &Conn) {
        conn.close();
    }
    /// Unblock a pending `accept`.
    pub fn wake(path: &Path) {
        let _ = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path);
    }
    /// Pipes live in one flat namespace; the temp dir's name keeps tests apart.
    pub fn endpoint(dir: &Path, name: &str) -> PathBuf {
        let unique = dir.file_name().unwrap_or_default().to_string_lossy();
        PathBuf::from(format!(r"\\.\pipe\decoy-test-{unique}-{name}"))
    }
}

pub use net::endpoint;
use net::Conn;

struct Shared {
    cfg: Mutex<MockConfig>,
    conns: Mutex<Vec<Conn>>,
    stop: AtomicBool,
}

pub struct Mock {
    pub path: PathBuf,
    rx: Receiver<Received>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

pub const USER_ID: &str = "42";

impl Mock {
    pub fn start(dir: &Path, cfg: MockConfig) -> Mock {
        Mock::start_at(endpoint(dir, "discord-ipc-0"), cfg)
    }

    pub fn start_at(path: PathBuf, cfg: MockConfig) -> Mock {
        let listener = net::Listener::bind(&path).unwrap();
        let shared = Arc::new(Shared {
            cfg: Mutex::new(cfg),
            conns: Mutex::new(Vec::new()),
            stop: AtomicBool::new(false),
        });
        let (tx, rx) = channel();
        let sh = shared.clone();
        let thread = thread::spawn(move || {
            let mut n = 0;
            loop {
                let conn = listener.accept();
                if sh.stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(conn) = conn else { continue };
                sh.conns.lock().unwrap().push(net::dup(&conn));
                let (sh, tx) = (sh.clone(), tx.clone());
                thread::spawn(move || handle(conn, n, sh, tx));
                n += 1;
            }
        });
        Mock {
            path,
            rx,
            shared,
            thread: Some(thread),
        }
    }

    pub fn configure(&self, f: impl FnOnce(&mut MockConfig)) {
        f(&mut self.shared.cfg.lock().unwrap());
    }

    /// Shut down every accepted connection (the listener keeps accepting).
    pub fn kick(&self) {
        for c in self.shared.conns.lock().unwrap().drain(..) {
            net::close(&c);
        }
    }

    pub fn recv(&self) -> Received {
        match self.rx.recv_timeout(Duration::from_secs(5)) {
            Ok(r) => r,
            Err(RecvTimeoutError::Timeout) => panic!("mock: no frame within 5 s"),
            Err(e) => panic!("mock: {e}"),
        }
    }

    pub fn try_recv(&self, wait: Duration) -> Option<Received> {
        self.rx.recv_timeout(wait).ok()
    }

    /// Next SET_ACTIVITY frame (skips handshakes and PONGs).
    pub fn recv_set_activity(&self) -> Received {
        loop {
            let r = self.recv();
            if r.op == OP_FRAME && r.value["cmd"] == "SET_ACTIVITY" {
                return r;
            }
        }
    }

    /// All SET_ACTIVITY frames that arrive until `quiet` passes with nothing new.
    pub fn drain_set_activity(&self, quiet: Duration) -> Vec<Received> {
        let mut out = Vec::new();
        while let Some(r) = self.try_recv(quiet) {
            if r.op == OP_FRAME && r.value["cmd"] == "SET_ACTIVITY" {
                out.push(r);
            }
        }
        out
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        net::wake(&self.path);
        self.kick();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn handle(mut s: Conn, conn: usize, sh: Arc<Shared>, tx: Sender<Received>) {
    let report = |op: u32, payload: &[u8]| {
        let value = serde_json::from_slice(payload).unwrap_or(Value::Null);
        let _ = tx.send(Received { conn, op, value });
    };
    let Ok((op, payload)) = read_frame(&mut s) else {
        return;
    };
    report(op, &payload);
    if op != OP_HANDSHAKE {
        return;
    }
    let cfg = sh.cfg.lock().unwrap().clone();
    if let Some((code, message)) = cfg.close_at_handshake {
        let body = json!({ "code": code, "message": message }).to_string();
        let _ = write_frame(&mut s, OP_CLOSE, body.as_bytes());
        return;
    }
    let ready = json!({
        "cmd": "DISPATCH", "evt": "READY",
        "data": { "v": 1, "user": { "id": USER_ID, "username": "tester", "global_name": "Tester", "avatar": null } }
    });
    if write_frame(&mut s, OP_FRAME, ready.to_string().as_bytes()).is_err() {
        return;
    }
    while let Ok((op, payload)) = read_frame(&mut s) {
        report(op, &payload);
        let v: Value = serde_json::from_slice(&payload).unwrap_or(Value::Null);
        if op != OP_FRAME || v["cmd"] != "SET_ACTIVITY" {
            continue;
        }
        let cfg = sh.cfg.lock().unwrap().clone();
        if cfg.drop_on_set {
            net::close(&s);
            return;
        }
        if cfg.ping_before_reply {
            // An unrelated frame first, then a PING that must be echoed back.
            let noise = json!({ "cmd": "DISPATCH", "evt": "NOISE", "nonce": "other" });
            let _ = write_frame(&mut s, OP_FRAME, noise.to_string().as_bytes());
            let _ = write_frame(&mut s, OP_PING, br#"{"hello":1}"#);
        }
        let reply = match cfg.error_on_set {
            Some((code, message)) => json!({
                "cmd": "SET_ACTIVITY", "evt": "ERROR", "nonce": v["nonce"],
                "data": { "code": code, "message": message }
            }),
            None => json!({
                "cmd": "SET_ACTIVITY", "evt": null, "nonce": v["nonce"],
                "data": v["args"]["activity"]
            }),
        };
        if write_frame(&mut s, OP_FRAME, reply.to_string().as_bytes()).is_err() {
            return;
        }
    }
}
