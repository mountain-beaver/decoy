//! Background worker that owns the IPC connection and keeps the presence applied.

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::ipc::{Client, ConnectOptions, DiscordUser};
use crate::presence::{local_day_start_ms, Presence, TimeContext, TimestampMode};
use crate::util::now_ms;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EngineState {
    /// Nothing to do: never applied, or disconnected on purpose.
    Idle,
    Connecting,
    /// Discord is not reachable; the engine keeps retrying.
    WaitingForDiscord,
    /// Connected and the presence is shown.
    Active {
        user: DiscordUser,
    },
    /// Connected, no activity shown.
    Connected {
        user: DiscordUser,
    },
    /// Not retryable (for example an invalid application id).
    Failed {
        message: String,
    },
}

/// Timing knobs; tests shrink them to milliseconds.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Timing {
    /// Delays between reconnect attempts; the last one repeats.
    pub backoff: Vec<Duration>,
    /// Minimum gap between two activity updates.
    pub min_update_interval: Duration,
    /// How often an idle connection is checked for being alive.
    pub liveness_interval: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            backoff: vec![
                Duration::from_secs(2),
                Duration::from_secs(5),
                Duration::from_secs(15),
            ],
            min_update_interval: Duration::from_secs(4),
            liveness_interval: Duration::from_secs(15),
        }
    }
}

enum Cmd {
    Apply(Box<Presence>, i32),
    Clear,
    Disconnect,
    Shutdown,
}

/// Handle to the worker thread. All methods return immediately.
pub struct Engine {
    tx: Sender<Cmd>,
    state: Arc<Mutex<EngineState>>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    /// Start the worker. `on_state` is called on the worker thread, only when the state changes.
    pub fn spawn(opts: ConnectOptions, on_state: impl Fn(EngineState) + Send + 'static) -> Engine {
        Self::spawn_with(opts, Timing::default(), on_state)
    }

    #[doc(hidden)]
    pub fn spawn_with(
        opts: ConnectOptions,
        timing: Timing,
        on_state: impl Fn(EngineState) + Send + 'static,
    ) -> Engine {
        let (tx, rx) = channel();
        let state = Arc::new(Mutex::new(EngineState::Idle));
        let worker = Worker::new(opts, timing, state.clone(), Box::new(on_state), rx);
        let thread = thread::Builder::new()
            .name("decoy-engine".into())
            .spawn(move || worker.run())
            .expect("failed to spawn engine thread");
        Engine {
            tx,
            state,
            thread: Some(thread),
        }
    }

    /// Show `presence`, connecting first if needed. `utc_offset_secs` is the local UTC offset.
    pub fn apply(&self, presence: Presence, utc_offset_secs: i32) {
        let _ = self
            .tx
            .send(Cmd::Apply(Box::new(presence), utc_offset_secs));
    }

    /// Remove the activity but stay connected.
    pub fn clear(&self) {
        let _ = self.tx.send(Cmd::Clear);
    }

    /// Drop the connection and stop retrying until the next `apply`.
    pub fn disconnect(&self) {
        let _ = self.tx.send(Cmd::Disconnect);
    }

    pub fn state(&self) -> EngineState {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Shutdown);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

struct Live {
    client: Client,
    user: DiscordUser,
    app_id: String,
    connected_at_ms: i64,
    /// Earliest moment the next activity update may go out.
    next_send: Instant,
    next_liveness: Instant,
}

struct Worker {
    opts: ConnectOptions,
    timing: Timing,
    shared: Arc<Mutex<EngineState>>,
    on_state: Box<dyn Fn(EngineState) + Send>,
    rx: Receiver<Cmd>,
    live: Option<Live>,
    /// Application id of the last `apply`; used to connect.
    app_id: Option<String>,
    /// Presence to show; `None` means cleared.
    want: Option<(Presence, i32)>,
    /// Whether the engine should hold a connection (and reconnect when it breaks).
    want_connected: bool,
    /// The wanted activity differs from what Discord has.
    dirty: bool,
    /// Start of the local day the "time of day" timer on Discord counts from.
    day_sent: Option<i64>,
    attempt: usize,
    retry_at: Instant,
}

impl Worker {
    fn new(
        opts: ConnectOptions,
        timing: Timing,
        shared: Arc<Mutex<EngineState>>,
        on_state: Box<dyn Fn(EngineState) + Send>,
        rx: Receiver<Cmd>,
    ) -> Worker {
        Worker {
            opts,
            timing,
            shared,
            on_state,
            rx,
            live: None,
            app_id: None,
            want: None,
            want_connected: false,
            dirty: false,
            day_sent: None,
            attempt: 0,
            retry_at: Instant::now(),
        }
    }

    fn set_state(&self, new: EngineState) {
        {
            let mut cur = self.shared.lock().unwrap_or_else(|p| p.into_inner());
            if *cur == new {
                return;
            }
            *cur = new.clone();
        }
        (self.on_state)(new);
    }

    /// Earliest moment something needs doing, or `None` to sleep until a command arrives.
    fn next_deadline(&self) -> Option<Instant> {
        match &self.live {
            Some(l) => {
                let mut at = l.next_liveness;
                if self.dirty {
                    at = at.min(l.next_send);
                }
                Some(at)
            }
            None => self.want_connected.then_some(self.retry_at),
        }
    }

    fn run(mut self) {
        loop {
            let cmd = match self.next_deadline() {
                None => self.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(at) => self
                    .rx
                    .recv_timeout(at.saturating_duration_since(Instant::now())),
            };
            match cmd {
                Ok(Cmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(c) => {
                    self.handle(c);
                    // Take everything already queued so a burst coalesces.
                    while let Ok(c) = self.rx.try_recv() {
                        if matches!(c, Cmd::Shutdown) {
                            return;
                        }
                        self.handle(c);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.tick();
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Apply(p, offset) => {
                let id = p.app_id.trim().to_string();
                if self.live.as_ref().is_some_and(|l| l.app_id != id) {
                    // The client id is fixed at handshake, so start over.
                    self.live = None;
                }
                if !self.want_connected || self.live.is_none() {
                    self.attempt = 0;
                    self.retry_at = Instant::now();
                }
                self.app_id = Some(id);
                self.want = Some((*p, offset));
                self.want_connected = true;
                self.dirty = true;
            }
            Cmd::Clear => {
                self.want = None;
                if self.live.is_some() {
                    self.dirty = true;
                }
            }
            Cmd::Disconnect => {
                self.live = None;
                self.want = None;
                self.want_connected = false;
                self.dirty = false;
                self.set_state(EngineState::Idle);
            }
            Cmd::Shutdown => {}
        }
    }

    /// Start of the current local day, if the wanted presence shows the time of day.
    fn local_day(&self) -> Option<i64> {
        let (p, applied) = self.want.as_ref()?;
        if !matches!(p.timestamps, TimestampMode::LocalTime) {
            return None;
        }
        let offset = self.opts.local_offset.map_or(*applied, |f| f());
        Some(local_day_start_ms(now_ms(), offset))
    }

    fn tick(&mut self) {
        // Past midnight (or after a clock change) the timer on Discord would
        // keep counting from yesterday: send it again.
        if self.live.is_some() && !self.dirty {
            let day = self.local_day();
            if day.is_some() && day != self.day_sent {
                self.dirty = true;
            }
        }
        let now = Instant::now();
        if self.live.is_none() && self.want_connected && now >= self.retry_at {
            self.connect();
        }
        let Some(l) = &self.live else { return };
        if self.dirty && Instant::now() >= l.next_send {
            self.send();
        }
        if let Some(l) = &mut self.live {
            if Instant::now() >= l.next_liveness {
                l.next_liveness = Instant::now() + self.timing.liveness_interval;
                if l.client.poll().is_err() {
                    self.connection_lost();
                }
            }
        }
    }

    fn connect(&mut self) {
        let Some(app_id) = self.app_id.clone() else {
            self.want_connected = false;
            return;
        };
        self.set_state(EngineState::Connecting);
        match Client::connect(&app_id, &self.opts) {
            Ok((client, user)) => {
                self.attempt = 0;
                // A fresh connection starts without any activity.
                self.dirty = self.want.is_some();
                self.set_state(EngineState::Connected { user: user.clone() });
                self.live = Some(Live {
                    client,
                    user,
                    app_id,
                    connected_at_ms: now_ms(),
                    next_send: Instant::now(),
                    next_liveness: Instant::now() + self.timing.liveness_interval,
                });
            }
            Err(Error::Rejected { message, .. }) => {
                self.want_connected = false;
                self.dirty = false;
                self.set_state(EngineState::Failed { message });
            }
            Err(_) => self.schedule_retry(),
        }
    }

    fn schedule_retry(&mut self) {
        let steps = &self.timing.backoff;
        let delay = steps
            .get(self.attempt)
            .or(steps.last())
            .copied()
            .unwrap_or(Duration::from_secs(15));
        self.attempt += 1;
        self.retry_at = Instant::now() + delay;
        self.set_state(EngineState::WaitingForDiscord);
    }

    fn connection_lost(&mut self) {
        self.live = None;
        self.dirty = self.want.is_some();
        self.attempt = 0;
        self.schedule_retry();
    }

    fn send(&mut self) {
        let day = self.local_day();
        let local_offset = self.opts.local_offset;
        let Some(l) = &mut self.live else { return };
        let activity = self.want.as_ref().map(|(p, offset)| {
            p.to_activity(&TimeContext {
                now_ms: now_ms(),
                connected_at_ms: l.connected_at_ms,
                utc_offset_secs: local_offset.map_or(*offset, |f| f()),
            })
        });
        let result = l.client.set_activity(activity.as_ref());
        l.next_send = Instant::now() + self.timing.min_update_interval;
        match result {
            Ok(()) => {
                self.dirty = false;
                self.day_sent = day;
                let user = l.user.clone();
                self.set_state(if activity.is_some() {
                    EngineState::Active { user }
                } else {
                    EngineState::Connected { user }
                });
            }
            Err(Error::Command { message, .. }) => {
                // Discord refused this content; keep the connection and wait for a new apply.
                self.dirty = false;
                self.set_state(EngineState::Failed { message });
            }
            Err(_) => self.connection_lost(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presence::{Presence, TimestampMode};
    use crate::testutil::{endpoint, Mock, MockConfig};
    use std::sync::mpsc::Receiver;

    const ID: &str = "123456789012345678";
    const WAIT: Duration = Duration::from_secs(5);

    fn timing() -> Timing {
        Timing {
            backoff: vec![Duration::from_millis(10), Duration::from_millis(20)],
            min_update_interval: Duration::from_millis(150),
            liveness_interval: Duration::from_millis(20),
        }
    }

    fn spawn(path: std::path::PathBuf) -> (Engine, Receiver<EngineState>) {
        let (tx, rx) = channel();
        let opts = ConnectOptions {
            paths: Some(vec![path]),
            timeout: Duration::from_secs(5),
            local_offset: Some(test_offset),
        };
        let e = Engine::spawn_with(opts, timing(), move |s| {
            let _ = tx.send(s);
        });
        (e, rx)
    }

    /// The "local UTC offset" the engines under test see.
    static OFFSET: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

    fn test_offset() -> i32 {
        OFFSET.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[test]
    fn time_of_day_timer_follows_the_clock() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        let mut p = presence("clock");
        p.timestamps = TimestampMode::LocalTime;
        engine.apply(p, 0);
        wait_for(&rx, is_active);
        let first = mock.recv_set_activity().value["args"]["activity"]["timestamps"]["start"]
            .as_i64()
            .unwrap();
        // Nothing changed: no second update.
        assert!(mock
            .drain_set_activity(Duration::from_millis(300))
            .is_empty());
        // Half a day later on the local clock, the day began at another moment.
        OFFSET.store(12 * 3600, std::sync::atomic::Ordering::SeqCst);
        let second = mock.recv_set_activity().value["args"]["activity"]["timestamps"]["start"]
            .as_i64()
            .unwrap();
        OFFSET.store(0, std::sync::atomic::Ordering::SeqCst);
        assert_eq!((second - first).abs(), 12 * 3600 * 1000);
    }

    fn presence(details: &str) -> Presence {
        Presence {
            app_id: ID.into(),
            details: Some(details.into()),
            ..Default::default()
        }
    }

    /// Wait until a state matching `pred` shows up; earlier states are skipped.
    fn wait_for(rx: &Receiver<EngineState>, pred: impl Fn(&EngineState) -> bool) -> EngineState {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match rx.recv_timeout(left) {
                Ok(s) if pred(&s) => return s,
                Ok(_) => {}
                Err(_) => panic!("timed out waiting for engine state"),
            }
        }
    }

    fn is_active(s: &EngineState) -> bool {
        matches!(s, EngineState::Active { .. })
    }

    #[test]
    fn connects_and_becomes_active() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        assert_eq!(engine.state(), EngineState::Idle);
        engine.apply(presence("Hello"), 0);
        assert_eq!(wait_for(&rx, |_| true), EngineState::Connecting);
        let s = wait_for(&rx, is_active);
        assert_eq!(engine.state(), s);
        let sent = mock.recv_set_activity().value;
        assert_eq!(sent["args"]["activity"]["details"], "Hello");
        assert_eq!(sent["args"]["activity"]["type"], 0);
    }

    #[test]
    fn state_serializes_as_tagged_json() {
        let v = serde_json::to_value(EngineState::WaitingForDiscord).unwrap();
        assert_eq!(v, serde_json::json!({ "status": "waiting_for_discord" }));
        let v = serde_json::to_value(EngineState::Failed {
            message: "x".into(),
        })
        .unwrap();
        assert_eq!(v, serde_json::json!({ "status": "failed", "message": "x" }));
    }

    #[test]
    fn burst_of_applies_coalesces_to_the_latest() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("first"), 0);
        wait_for(&rx, is_active);
        for i in 0..10 {
            engine.apply(presence(&format!("update {i}")), 0);
        }
        let sets = mock.drain_set_activity(Duration::from_millis(500));
        assert!(sets.len() <= 3, "got {} updates", sets.len());
        assert_eq!(
            sets.first().unwrap().value["args"]["activity"]["details"],
            "first"
        );
        assert_eq!(
            sets.last().unwrap().value["args"]["activity"]["details"],
            "update 9"
        );
    }

    #[test]
    fn updates_are_spaced_by_the_minimum_interval() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("a"), 0);
        wait_for(&rx, is_active);
        let t0 = Instant::now();
        engine.apply(presence("b"), 0);
        let sets = mock.drain_set_activity(Duration::from_millis(400));
        assert_eq!(sets.len(), 2);
        assert!(t0.elapsed() >= Duration::from_millis(100));
    }

    #[test]
    fn reconnects_and_reapplies_after_the_connection_drops() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        let mut p = presence("persist");
        p.timestamps = TimestampMode::SinceConnect;
        engine.apply(p, 0);
        wait_for(&rx, is_active);
        let first = mock.recv_set_activity();
        assert_eq!(first.conn, 0);

        mock.kick();
        wait_for(&rx, |s| *s == EngineState::WaitingForDiscord);
        wait_for(&rx, is_active);
        let second = mock.recv_set_activity();
        assert_eq!(second.conn, 1);
        assert_eq!(
            second.value["args"]["activity"]["details"],
            first.value["args"]["activity"]["details"]
        );
        // SinceConnect restarts with the new connection.
        let t1 = first.value["args"]["activity"]["timestamps"]["start"]
            .as_i64()
            .unwrap();
        let t2 = second.value["args"]["activity"]["timestamps"]["start"]
            .as_i64()
            .unwrap();
        assert!(t2 >= t1);
    }

    #[test]
    fn waits_for_discord_and_recovers_when_the_socket_appears() {
        let dir = tempfile::tempdir().unwrap();
        let path = endpoint(dir.path(), "discord-ipc-0");
        let (engine, rx) = spawn(path.clone());
        engine.apply(presence("late"), 0);
        wait_for(&rx, |s| *s == EngineState::WaitingForDiscord);
        let mock = Mock::start_at(path, MockConfig::default());
        wait_for(&rx, is_active);
        assert_eq!(
            mock.recv_set_activity().value["args"]["activity"]["details"],
            "late"
        );
    }

    #[test]
    fn rejected_client_id_fails_without_retrying() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(
            dir.path(),
            MockConfig {
                close_at_handshake: Some((4000, "Invalid Client ID".into())),
                ..Default::default()
            },
        );
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("x"), 0);
        let s = wait_for(&rx, |s| matches!(s, EngineState::Failed { .. }));
        assert_eq!(
            s,
            EngineState::Failed {
                message: "Invalid Client ID".into()
            }
        );
        // No further connection attempts: one handshake only.
        let mut handshakes = 0;
        while mock.try_recv(Duration::from_millis(200)).is_some() {
            handshakes += 1;
        }
        assert!(handshakes <= 1, "{handshakes} extra frames");
        assert!(matches!(engine.state(), EngineState::Failed { .. }));
    }

    #[test]
    fn clear_keeps_the_connection() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("x"), 0);
        wait_for(&rx, is_active);
        engine.clear();
        let s = wait_for(&rx, |s| matches!(s, EngineState::Connected { .. }));
        assert_eq!(engine.state(), s);
        let _ = mock.recv_set_activity();
        let cleared = mock.recv_set_activity();
        assert!(cleared.value["args"]["activity"].is_null());
        assert_eq!(cleared.conn, 0);
    }

    #[test]
    fn disconnect_goes_idle_and_stops_retrying() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("x"), 0);
        wait_for(&rx, is_active);
        engine.disconnect();
        wait_for(&rx, |s| *s == EngineState::Idle);
        while mock.try_recv(Duration::from_millis(200)).is_some() {}
        // Still idle, and no new connection shows up.
        assert!(mock.try_recv(Duration::from_millis(200)).is_none());
        assert_eq!(engine.state(), EngineState::Idle);
        // A later apply connects again.
        engine.apply(presence("again"), 0);
        wait_for(&rx, is_active);
    }

    #[test]
    fn changing_the_app_id_reconnects() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("one"), 0);
        wait_for(&rx, is_active);
        let mut other = presence("two");
        other.app_id = "987654321098765432".into();
        engine.apply(other, 0);
        wait_for(&rx, |s| *s == EngineState::Connecting);
        wait_for(&rx, is_active);
        let mut ids = Vec::new();
        while let Some(r) = mock.try_recv(Duration::from_millis(200)) {
            if r.op == crate::ipc::OP_HANDSHAKE {
                ids.push(r.value["client_id"].as_str().unwrap().to_string());
            }
        }
        // First handshake was consumed by nobody: both are in the queue.
        assert_eq!(ids, vec![ID.to_string(), "987654321098765432".to_string()]);
    }

    #[test]
    fn command_error_reports_failed_but_stays_connected() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(
            dir.path(),
            MockConfig {
                error_on_set: Some((4000, "Bad image".into())),
                ..Default::default()
            },
        );
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("x"), 0);
        wait_for(&rx, |s| {
            *s == EngineState::Failed {
                message: "Bad image".into(),
            }
        });
        mock.configure(|c| c.error_on_set = None);
        engine.apply(presence("fixed"), 0);
        wait_for(&rx, is_active);
    }

    #[test]
    fn drop_shuts_the_thread_down() {
        let dir = tempfile::tempdir().unwrap();
        let mock = Mock::start(dir.path(), MockConfig::default());
        let (engine, rx) = spawn(mock.path.clone());
        engine.apply(presence("x"), 0);
        wait_for(&rx, is_active);
        let t = Instant::now();
        drop(engine);
        assert!(t.elapsed() < Duration::from_secs(2));
    }
}
