//! RPC round trips against in-process daemons on temp sockets. Never touches
//! the real Discord socket: the engine only knows a path that does not exist.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use decoy_app::daemon::{Daemon, Hooks};
use decoy_app::rpc::{
    ensure_daemon, BindError, Client, Connection, Server, Service, EVENT_APP_STATE,
    EVENT_ENGINE_STATE,
};
use decoy_app::DaemonState;
use decoy_core::{ConnectOptions, Issue, Presence, Profile, Settings};
use serde_json::{json, Value};

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        Env {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    fn socket(&self) -> PathBuf {
        self.dir.path().join("decoy.sock")
    }
    fn config(&self) -> PathBuf {
        self.dir.path().join("config")
    }
}

#[derive(Default)]
struct Counters {
    open_ui: AtomicUsize,
    quit: AtomicUsize,
}

fn hooks(counters: &Arc<Counters>) -> Hooks {
    let (a, b) = (counters.clone(), counters.clone());
    Hooks {
        on_change: Arc::new(|_| {}),
        open_ui: Arc::new(move || {
            a.open_ui.fetch_add(1, Ordering::SeqCst);
        }),
        quit: Arc::new(move || {
            b.quit.fetch_add(1, Ordering::SeqCst);
        }),
    }
}

fn start(env: &Env, counters: &Arc<Counters>) -> Result<Daemon, BindError> {
    start_in(&env.socket(), &env.config(), counters)
}

fn start_in(socket: &Path, config: &Path, counters: &Arc<Counters>) -> Result<Daemon, BindError> {
    Daemon::start(
        socket,
        config,
        ConnectOptions {
            // Never the real Discord.
            paths: Some(vec![socket.with_extension("no-discord")]),
            ..Default::default()
        },
        hooks(counters),
    )
}

type Events = Receiver<(String, Value)>;

fn connect(path: &Path) -> (Client, Events) {
    let (tx, rx) = channel();
    let tx = Mutex::new(tx);
    let client = Connection::open(path).unwrap().start(
        move |ev, payload| {
            let _ = tx.lock().unwrap().send((ev.to_string(), payload));
        },
        || {},
    );
    (client, rx)
}

fn valid_presence() -> Presence {
    Presence {
        app_id: "123456789012345678".into(),
        details: Some("Hello".into()),
        ..Default::default()
    }
}

fn next_event(rx: &Events, name: &str) -> Value {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let left = end.saturating_duration_since(Instant::now());
        let (ev, payload) = rx.recv_timeout(left).expect("event did not arrive");
        if ev == name {
            return payload;
        }
    }
}

#[test]
fn every_command_round_trips() {
    let env = Env::new();
    let counters = Arc::new(Counters::default());
    let _d = start(&env, &counters).unwrap();
    let (c, _events) = connect(&env.socket());

    let st: DaemonState = c.call("get_state", Value::Null).unwrap();
    assert!(st.profiles.is_empty() && !st.showing);
    assert_eq!(st.version, VERSION);

    let p: Profile = c.call("create_profile", json!({ "name": "Game" })).unwrap();
    assert_eq!(p.name, "Game");

    let mut edited = p.clone();
    edited.presence = valid_presence();
    let issues: Vec<Issue> = c
        .call("save_profile", json!({ "profile": edited }))
        .unwrap();
    assert!(issues.is_empty());
    let mut broken = edited.clone();
    broken.presence.app_id = "nope".into();
    let issues: Vec<Issue> = c
        .call("save_profile", json!({ "profile": broken }))
        .unwrap();
    assert!(!issues.is_empty());
    c.call::<Vec<Issue>>("save_profile", json!({ "profile": edited }))
        .unwrap();

    let issues: Vec<Issue> = c
        .call("validate", json!({ "presence": Presence::default() }))
        .unwrap();
    assert!(!issues.is_empty());
    let issues: Vec<Issue> = c
        .call("validate", json!({ "presence": valid_presence() }))
        .unwrap();
    assert!(issues.is_empty());

    let copy: Profile = c.call("duplicate_profile", json!({ "id": p.id })).unwrap();
    assert_eq!(copy.name, "Game copy");

    c.call::<()>("set_active_profile", json!({ "id": copy.id }))
        .unwrap();
    c.call::<()>("set_active_profile", json!({ "id": p.id }))
        .unwrap();

    let exported: Value = c.call("export_profile", json!({ "id": p.id })).unwrap();
    assert_eq!(exported["name"], "Game");
    let imported: Profile = c
        .call(
            "import_profile",
            json!({ "json": exported["json"].as_str().unwrap() }),
        )
        .unwrap();
    assert_ne!(imported.id, p.id);

    c.call::<()>(
        "set_settings",
        json!({ "settings": Settings { connect_on_launch: true, ..Default::default() } }),
    )
    .unwrap();
    let st: DaemonState = c.call("get_state", Value::Null).unwrap();
    assert!(st.settings.connect_on_launch);
    assert_eq!(st.settings.active_profile.as_deref(), Some(p.id.as_str()));
    assert_eq!(st.profiles.len(), 3);

    c.call::<()>("set_showing", json!({ "on": true })).unwrap();
    assert!(
        c.call::<DaemonState>("get_state", Value::Null)
            .unwrap()
            .showing
    );
    c.call::<()>("set_showing", json!({ "on": false })).unwrap();

    c.call::<()>("delete_profile", json!({ "id": copy.id }))
        .unwrap();
    assert_eq!(
        c.call::<DaemonState>("get_state", Value::Null)
            .unwrap()
            .profiles
            .len(),
        2
    );

    c.call::<()>("open_ui", Value::Null).unwrap();
    assert_eq!(counters.open_ui.load(Ordering::SeqCst), 1);
    assert_eq!(counters.quit.load(Ordering::SeqCst), 0);
    c.call::<()>("quit", Value::Null).unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while counters.quit.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < end, "quit hook never ran");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn errors_come_back_as_messages() {
    let env = Env::new();
    let _d = start(&env, &Arc::default()).unwrap();
    let (c, _e) = connect(&env.socket());

    assert_eq!(
        c.call::<()>("set_showing", json!({ "on": true }))
            .unwrap_err(),
        "Create or select a profile first"
    );
    let p: Profile = c
        .call("create_profile", json!({ "name": "Empty" }))
        .unwrap();
    assert_eq!(
        c.call::<()>("set_showing", json!({ "on": true }))
            .unwrap_err(),
        "Fix the highlighted fields first"
    );
    assert_eq!(
        c.call::<()>("delete_profile", json!({ "id": "nope" }))
            .unwrap_err(),
        "Profile not found"
    );
    assert!(c
        .call::<()>("set_active_profile", json!({ "id": "nope" }))
        .is_err());
    assert!(c
        .call::<Profile>("import_profile", json!({ "json": "[1]" }))
        .is_err());
    assert!(c
        .call::<Value>("export_profile", json!({ "id": "nope" }))
        .is_err());
    assert_eq!(
        c.call::<Value>("frobnicate", Value::Null).unwrap_err(),
        "Unknown command: frobnicate"
    );
    let bad = c
        .call::<()>("set_showing", json!({ "on": "yes" }))
        .unwrap_err();
    assert!(bad.starts_with("Bad argument `on`"), "{bad}");
    let bad = c
        .call::<Profile>("create_profile", Value::Null)
        .unwrap_err();
    assert!(bad.starts_with("Bad argument `name`"), "{bad}");
    // The connection survives all of that.
    assert!(c.call::<DaemonState>("get_state", Value::Null).is_ok());
    assert!(!p.id.is_empty());
}

#[test]
fn garbage_gets_a_bad_request_reply_and_the_connection_lives() {
    use std::io::{BufRead, BufReader, Write};
    let env = Env::new();
    let _d = start(&env, &Arc::default()).unwrap();
    let mut s = std::os::unix::net::UnixStream::connect(env.socket()).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    assert!(
        line.contains("\"hello\"") && line.contains(VERSION),
        "{line}"
    );
    s.write_all(b"this is not json\n").unwrap();
    line.clear();
    r.read_line(&mut line).unwrap();
    assert_eq!(line.trim(), r#"{"err":"Bad request","id":0}"#);
    s.write_all(b"{\"id\":5,\"cmd\":\"get_state\"}\n").unwrap();
    line.clear();
    r.read_line(&mut line).unwrap();
    let v: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["id"], 5);
    assert_eq!(v["ok"]["version"], VERSION);
}

#[test]
fn state_changes_are_pushed_to_every_client() {
    let env = Env::new();
    let _d = start(&env, &Arc::default()).unwrap();
    let (a, ea) = connect(&env.socket());
    let (_b, eb) = connect(&env.socket());

    let mut p: Profile = a.call("create_profile", json!({ "name": "P" })).unwrap();
    p.presence = valid_presence();
    a.call::<Vec<Issue>>("save_profile", json!({ "profile": p }))
        .unwrap();
    // Turning it on makes the engine leave Idle: an engine-state event for both.
    a.call::<()>("set_showing", json!({ "on": true })).unwrap();
    for rx in [&ea, &eb] {
        let engine = next_event(rx, EVENT_ENGINE_STATE);
        assert!(engine["status"].is_string(), "{engine}");
    }
    // Window-originated changes are not pushed as app-state (the window knows).
    assert!(ea
        .try_recv()
        .map(|(e, _)| e != EVENT_APP_STATE)
        .unwrap_or(true));
}

#[test]
fn tray_originated_changes_push_app_state() {
    let env = Env::new();
    let d = start(&env, &Arc::default()).unwrap();
    let (a, ea) = connect(&env.socket());
    let mut p: Profile = a.call("create_profile", json!({ "name": "P" })).unwrap();
    p.presence = valid_presence();
    a.call::<Vec<Issue>>("save_profile", json!({ "profile": p }))
        .unwrap();
    d.controller.set_showing_external(true).unwrap();
    let state = next_event(&ea, EVENT_APP_STATE);
    assert_eq!(state["showing"], true);
    assert_eq!(state["profiles"][0]["name"], "P");
    assert!(state.get("games").is_none());
}

#[test]
fn socket_is_private_and_stale_files_are_replaced() {
    let env = Env::new();
    // A stale socket: bound once, listener gone, file left behind.
    drop(UnixListener::bind(env.socket()).unwrap());
    assert!(env.socket().exists());
    let d = start(&env, &Arc::default()).unwrap();
    let mode = std::fs::metadata(env.socket())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let (c, _e) = connect(&env.socket());
    assert!(c.call::<DaemonState>("get_state", Value::Null).is_ok());
    drop(d);
}

#[test]
fn a_second_daemon_gives_up_and_the_first_keeps_working() {
    let env = Env::new();
    let _d = start(&env, &Arc::default()).unwrap();
    let t = Instant::now();
    // Different config dir, same socket.
    let other = start_in(
        &env.socket(),
        &env.dir.path().join("config2"),
        &Arc::default(),
    );
    assert!(matches!(other, Err(BindError::AlreadyRunning)));
    assert!(t.elapsed() < Duration::from_secs(1));
    let (c, _e) = connect(&env.socket());
    assert!(c.call::<DaemonState>("get_state", Value::Null).is_ok());
}

#[test]
fn close_lets_a_new_daemon_take_over() {
    let env = Env::new();
    let d = start(&env, &Arc::default()).unwrap();
    d.shutdown();
    assert!(Connection::open(&env.socket()).is_err());
    let d2 = start(&env, &Arc::default()).unwrap();
    assert!(Connection::open(&env.socket()).is_ok());
    drop(d2);
}

#[test]
fn ensure_daemon_connects_without_spawning_when_versions_match() {
    let env = Env::new();
    let _d = start(&env, &Arc::default()).unwrap();
    let c = ensure_daemon(
        &env.socket(),
        VERSION,
        &|| panic!("must not spawn"),
        |_, _| {},
        || {},
    )
    .unwrap();
    assert!(c.call::<DaemonState>("get_state", Value::Null).is_ok());
}

#[test]
fn ensure_daemon_spawns_when_nothing_listens() {
    let env = Env::new();
    let spawned = Arc::new(Mutex::new(None::<Daemon>));
    let slot = spawned.clone();
    let (sock, cfg) = (env.socket(), env.config());
    let c = ensure_daemon(
        &env.socket(),
        VERSION,
        &move || {
            // Start it a little late, like a real process would come up.
            let slot = slot.clone();
            let (sock, cfg) = (sock.clone(), cfg.clone());
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(150));
                *slot.lock().unwrap() = Some(start_in(&sock, &cfg, &Arc::default()).unwrap());
            });
            Ok(())
        },
        |_, _| {},
        || {},
    )
    .unwrap();
    assert!(c.call::<DaemonState>("get_state", Value::Null).is_ok());
}

#[test]
fn ensure_daemon_reports_a_daemon_that_never_comes_up() {
    let env = Env::new();
    let started = Instant::now();
    let err = ensure_daemon(&env.socket(), VERSION, &|| Ok(()), |_, _| {}, || {})
        .err()
        .unwrap();
    assert!(err.contains("did not start"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(6));
    let err = ensure_daemon(
        &env.socket(),
        VERSION,
        &|| Err(std::io::Error::other("no such file")),
        |_, _| {},
        || {},
    )
    .err()
    .unwrap();
    assert!(err.starts_with("Could not start the Decoy background service"));
}

/// A stand-in for an old daemon: another version that closes itself on `quit`.
struct OldDaemon {
    server: Mutex<Option<Server>>,
    quits: AtomicUsize,
}

impl Service for OldDaemon {
    fn call(&self, cmd: &str, _args: Value) -> Result<Value, String> {
        match cmd {
            "quit" => Ok(Value::Null),
            _ => Err("old".into()),
        }
    }
    fn hello(&self) -> Value {
        json!({ "version": "0.0.1" })
    }
    fn after_reply(&self, cmd: &str) {
        if cmd == "quit" {
            self.quits.fetch_add(1, Ordering::SeqCst);
            if let Some(s) = self.server.lock().unwrap().take() {
                s.close();
            }
        }
    }
}

#[test]
fn a_daemon_of_another_version_is_replaced() {
    let env = Env::new();
    let server = Server::bind(&env.socket()).unwrap();
    let old = Arc::new(OldDaemon {
        server: Mutex::new(Some(server.clone())),
        quits: AtomicUsize::new(0),
    });
    server.serve(old.clone());

    let new_daemon = Arc::new(Mutex::new(None::<Daemon>));
    let slot = new_daemon.clone();
    let (sock, cfg) = (env.socket(), env.config());
    let c = ensure_daemon(
        &env.socket(),
        VERSION,
        &move || {
            *slot.lock().unwrap() = Some(start_in(&sock, &cfg, &Arc::default()).unwrap());
            Ok(())
        },
        |_, _| {},
        || {},
    )
    .unwrap();
    assert_eq!(old.quits.load(Ordering::SeqCst), 1);
    let st: DaemonState = c.call("get_state", Value::Null).unwrap();
    assert_eq!(st.version, VERSION);
}

#[test]
fn losing_the_daemon_closes_the_client() {
    let env = Env::new();
    let listener = UnixListener::bind(env.socket()).unwrap();
    let t = std::thread::spawn(move || {
        use std::io::Write;
        let (mut s, _) = listener.accept().unwrap();
        s.write_all(b"{\"event\":\"hello\",\"payload\":{\"version\":\"1\"}}\n")
            .unwrap();
        std::thread::sleep(Duration::from_millis(200));
        // Dropping `s` closes the connection.
    });
    let (tx, rx) = channel();
    let c = Connection::open(&env.socket())
        .unwrap()
        .start(|_, _| {}, move || tx.send(()).unwrap());
    rx.recv_timeout(Duration::from_secs(5)).expect("on_close");
    t.join().unwrap();
    assert!(c.is_closed());
    assert_eq!(
        c.call::<()>("get_state", Value::Null).unwrap_err(),
        "The Decoy background service is not running"
    );
}

#[test]
fn a_pending_call_fails_when_the_daemon_dies() {
    let env = Env::new();
    let listener = UnixListener::bind(env.socket()).unwrap();
    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader, Write};
        let (mut s, _) = listener.accept().unwrap();
        s.write_all(b"{\"event\":\"hello\",\"payload\":{\"version\":\"1\"}}\n")
            .unwrap();
        let mut line = String::new();
        BufReader::new(s.try_clone().unwrap())
            .read_line(&mut line)
            .unwrap();
        // Never answer; just hang up.
    });
    let c = Connection::open(&env.socket())
        .unwrap()
        .start(|_, _| {}, || {});
    let t = Instant::now();
    assert_eq!(
        c.call::<()>("get_state", Value::Null).unwrap_err(),
        "The Decoy background service is not running"
    );
    assert!(t.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_non_decoy_peer_is_rejected() {
    let env = Env::new();
    let listener = UnixListener::bind(env.socket()).unwrap();
    std::thread::spawn(move || {
        use std::io::Write;
        let (mut s, _) = listener.accept().unwrap();
        s.write_all(b"HTTP/1.1 400 Bad Request\r\n").unwrap();
    });
    assert!(Connection::open(&env.socket()).is_err());
}
