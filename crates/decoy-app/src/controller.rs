//! Profiles, settings, the "showing" flag and the presence engine.
//!
//! Free of any GUI or IPC types so it can be unit-tested. Every change is
//! reported through a [`Notifier`]; the outer layer turns those into window
//! events, tray updates or RPC pushes.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use decoy_core::{ConnectOptions, Engine, EngineState, Issue, Presence, Profile, Settings, Store};
use serde::{Deserialize, Serialize};

use crate::games::GamesInfo;

const STATE_FILE: &str = "state.json";

pub const MSG_NO_PROFILE: &str = "Create or select a profile first";
pub const MSG_FIX_FIELDS: &str = "Fix the highlighted fields first";
pub const MSG_NOT_FOUND: &str = "Profile not found";

/// Something the outside world should hear about.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// The engine connection state changed.
    Engine(EngineState),
    /// Profiles, active profile, showing or settings changed.
    /// `external` is true when the cause was not the window (tray, startup
    /// restore), so the window has to be told.
    App { external: bool },
}

pub trait Notifier: Send + Sync + 'static {
    fn notify(&self, change: Change);
}

impl<F: Fn(Change) + Send + Sync + 'static> Notifier for F {
    fn notify(&self, change: Change) {
        self(change)
    }
}

/// Everything the controller owns, as the tray and the RPC see it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonState {
    pub engine: EngineState,
    pub showing: bool,
    pub profiles: Vec<Profile>,
    pub settings: Settings,
    pub version: String,
}

/// What the window sees: the controller state plus the game-list status.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppState {
    #[serde(flatten)]
    pub state: DaemonState,
    pub games: GamesInfo,
}

/// Small extra state that the core `Settings` has no room for.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct StateFile {
    showing: bool,
}

pub struct Controller {
    store: Store,
    engine: Engine,
    showing: AtomicBool,
    /// What the previous run left in `state.json`; used once by `start`.
    persisted_showing: bool,
    config_dir: PathBuf,
    notifier: Arc<dyn Notifier>,
    /// Serialises everything that changes profiles, settings or `showing`.
    op: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn local_utc_offset_secs() -> i32 {
    chrono::Local::now().offset().local_minus_utc()
}

fn err(e: decoy_core::Error) -> String {
    e.to_string()
}

impl Controller {
    pub fn new(
        config_dir: &Path,
        opts: ConnectOptions,
        notifier: Arc<dyn Notifier>,
    ) -> Result<Arc<Controller>, String> {
        let store = Store::open(config_dir).map_err(err)?;
        // Repair a dangling active profile (file edited by hand, for example).
        let settings = store.settings();
        if let Some(id) = &settings.active_profile {
            if store.get(id).is_none() {
                store
                    .set_settings(Settings {
                        active_profile: None,
                        ..settings
                    })
                    .map_err(err)?;
            }
        }
        let persisted_showing = std::fs::read(config_dir.join(STATE_FILE))
            .ok()
            .and_then(|b| serde_json::from_slice::<StateFile>(&b).ok())
            .map(|s| s.showing)
            .unwrap_or(false);
        let n = notifier.clone();
        let engine = Engine::spawn(opts, move |s| n.notify(Change::Engine(s)));
        Ok(Arc::new(Controller {
            store,
            engine,
            showing: AtomicBool::new(false),
            persisted_showing,
            config_dir: config_dir.to_path_buf(),
            notifier,
            op: Mutex::new(()),
        }))
    }

    /// Restore the previous "showing" state if the user wants that.
    pub fn start(&self) {
        let restored = {
            let _g = lock(&self.op);
            if self.persisted_showing
                && self.store.settings().connect_on_launch
                && self.apply_active().is_ok()
            {
                self.showing.store(true, Ordering::SeqCst);
                true
            } else {
                false
            }
        };
        if restored {
            self.changed(true);
        }
    }

    /// Drop the Discord connection, which removes the activity, and wait until
    /// the engine has done so (bounded) so the caller can exit right after.
    /// Does not touch the persisted "showing" flag, so the next launch can
    /// restore it.
    pub fn shutdown(&self) {
        self.engine.disconnect();
        let end = Instant::now() + Duration::from_secs(1);
        while self.engine.state() != EngineState::Idle && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn changed(&self, external: bool) {
        self.notifier.notify(Change::App { external });
    }

    /// Run `f` under the operation lock, then notify (without the lock held).
    fn mutate<T>(
        &self,
        external: bool,
        f: impl FnOnce(&Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let (out, showing_changed) = {
            let _g = lock(&self.op);
            let before = self.is_showing();
            let out = f(self);
            (out, before != self.is_showing())
        };
        if out.is_ok() {
            // A change the window did not ask for directly (the presence was
            // hidden because the new active profile cannot be shown) must reach
            // it too, or its switch keeps saying "Showing in Discord".
            self.changed(external || showing_changed);
        }
        out
    }

    // ----- state -----

    pub fn is_showing(&self) -> bool {
        self.showing.load(Ordering::SeqCst)
    }

    pub fn state(&self) -> DaemonState {
        DaemonState {
            engine: self.engine.state(),
            showing: self.is_showing(),
            profiles: self.store.profiles(),
            settings: self.store.settings(),
            version: crate::VERSION.to_string(),
        }
    }

    // ----- showing -----

    fn persist_showing(&self, on: bool) {
        let path = self.config_dir.join(STATE_FILE);
        let tmp = self.config_dir.join(format!("{STATE_FILE}.tmp"));
        let write = || -> std::io::Result<()> {
            let json = serde_json::to_vec(&StateFile { showing: on })?;
            std::fs::write(&tmp, json)?;
            std::fs::rename(&tmp, &path)
        };
        if let Err(e) = write() {
            eprintln!("decoy: could not save {STATE_FILE}: {e}");
        }
    }

    fn set_showing_flag(&self, on: bool) {
        self.showing.store(on, Ordering::SeqCst);
        self.persist_showing(on);
    }

    /// Send the active profile to the engine. Does not change `showing`.
    fn apply_active(&self) -> Result<(), String> {
        let id = self.store.settings().active_profile.ok_or(MSG_NO_PROFILE)?;
        let profile = self.store.get(&id).ok_or(MSG_NO_PROFILE)?;
        if !profile.presence.validate().is_empty() {
            return Err(MSG_FIX_FIELDS.into());
        }
        self.engine.apply(profile.presence, local_utc_offset_secs());
        Ok(())
    }

    /// After the active profile changed: keep showing it if possible, otherwise hide.
    fn reapply_or_hide(&self) {
        if self.is_showing() && self.apply_active().is_err() {
            self.engine.clear();
            self.set_showing_flag(false);
        }
    }

    fn set_showing_inner(&self, on: bool) -> Result<(), String> {
        if on {
            self.apply_active()?;
        } else {
            self.engine.clear();
        }
        self.set_showing_flag(on);
        Ok(())
    }

    pub fn set_showing(&self, on: bool) -> Result<(), String> {
        self.mutate(false, |c| c.set_showing_inner(on))
    }

    /// Same as `set_showing`, for the tray (the window is told about it).
    pub fn set_showing_external(&self, on: bool) -> Result<(), String> {
        self.mutate(true, |c| c.set_showing_inner(on))
    }

    // ----- profiles -----

    fn set_active_id(&self, id: Option<String>) -> Result<(), String> {
        let settings = self.store.settings();
        self.store
            .set_settings(Settings {
                active_profile: id,
                ..settings
            })
            .map_err(err)
    }

    pub fn save_profile(&self, profile: Profile) -> Result<Vec<Issue>, String> {
        let issues = profile.presence.validate();
        self.mutate(false, |c| {
            let saved = c.store.upsert(profile).map_err(err)?;
            let active = c.store.settings().active_profile.as_deref() == Some(saved.id.as_str());
            if c.is_showing() && active && issues.is_empty() {
                c.apply_active()?;
            }
            Ok(())
        })?;
        Ok(issues)
    }

    pub fn create_profile(&self, name: &str) -> Result<Profile, String> {
        let name = name.trim();
        let name = if name.is_empty() { "New profile" } else { name };
        self.mutate(false, |c| {
            let p = c
                .store
                .upsert(Profile {
                    id: String::new(),
                    name: name.to_string(),
                    presence: Presence::default(),
                })
                .map_err(err)?;
            c.set_active_id(Some(p.id.clone()))?;
            c.reapply_or_hide();
            Ok(p)
        })
    }

    pub fn duplicate_profile(&self, id: &str) -> Result<Profile, String> {
        self.mutate(false, |c| c.store.duplicate(id).map_err(err))
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        self.mutate(false, |c| {
            let was_active = c.store.settings().active_profile.as_deref() == Some(id);
            if !c.store.remove(id).map_err(err)? {
                return Err(MSG_NOT_FOUND.into());
            }
            if was_active {
                // Never leave the app without an active profile while some exist.
                let next = c.store.profiles().first().map(|p| p.id.clone());
                c.set_active_id(next)?;
                if c.is_showing() {
                    c.engine.clear();
                    c.set_showing_flag(false);
                }
            }
            Ok(())
        })
    }

    fn set_active_inner(&self, id: &str) -> Result<(), String> {
        if self.store.get(id).is_none() {
            return Err(MSG_NOT_FOUND.into());
        }
        self.set_active_id(Some(id.to_string()))?;
        self.reapply_or_hide();
        Ok(())
    }

    pub fn set_active_profile(&self, id: &str) -> Result<(), String> {
        self.mutate(false, |c| c.set_active_inner(id))
    }

    /// Same as `set_active_profile`, for the tray.
    pub fn set_active_profile_external(&self, id: &str) -> Result<(), String> {
        self.mutate(true, |c| c.set_active_inner(id))
    }

    /// The profile's name and its export JSON.
    pub fn export_profile(&self, id: &str) -> Result<(String, String), String> {
        let p = self.store.get(id).ok_or(MSG_NOT_FOUND)?;
        let json = self.store.export_profile(id).map_err(err)?;
        Ok((p.name, json))
    }

    pub fn import_profile(&self, json: &str) -> Result<Profile, String> {
        self.mutate(false, |c| c.store.import_profile(json).map_err(err))
    }

    pub fn validate(&self, presence: &Presence) -> Vec<Issue> {
        presence.validate()
    }

    /// Store new settings. `active_profile` in the argument is ignored: it is
    /// changed only through `set_active_profile` so a stale copy can never undo it.
    pub fn set_settings(&self, settings: Settings) -> Result<(), String> {
        self.mutate(false, |c| {
            let active_profile = c.store.settings().active_profile;
            c.store
                .set_settings(Settings {
                    active_profile,
                    ..settings
                })
                .map_err(err)
        })
    }
}

impl AppState {
    pub fn new(state: DaemonState, games: GamesInfo) -> AppState {
        AppState { state, games }
    }
}

/// Tray tooltip for a state.
pub fn tooltip(state: &DaemonState) -> String {
    if !state.showing {
        return "Decoy: hidden".into();
    }
    match &state.engine {
        EngineState::Idle | EngineState::Connecting | EngineState::WaitingForDiscord => {
            "Decoy: waiting for Discord".into()
        }
        EngineState::Failed { .. } => "Decoy: connection failed".into(),
        EngineState::Active { .. } | EngineState::Connected { .. } => {
            let name = state
                .settings
                .active_profile
                .as_ref()
                .and_then(|id| state.profiles.iter().find(|p| &p.id == id))
                .map(|p| p.name.trim())
                .filter(|n| !n.is_empty());
            match name {
                Some(n) => format!("Decoy: showing {n}"),
                None => "Decoy: showing".into(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Log = Arc<Mutex<Vec<Change>>>;

    struct Env {
        config: tempfile::TempDir,
        log: Log,
    }

    impl Env {
        fn new() -> Env {
            Env {
                config: tempfile::tempdir().unwrap(),
                log: Arc::default(),
            }
        }

        fn controller(&self) -> Arc<Controller> {
            let log = self.log.clone();
            let opts = ConnectOptions {
                // Never the real Discord.
                paths: Some(vec![self.config.path().join("no-such-socket")]),
                ..Default::default()
            };
            Controller::new(
                self.config.path(),
                opts,
                Arc::new(move |c: Change| log.lock().unwrap().push(c)),
            )
            .unwrap()
        }

        fn app_changes(&self) -> Vec<bool> {
            self.log
                .lock()
                .unwrap()
                .iter()
                .filter_map(|c| match c {
                    Change::App { external } => Some(*external),
                    _ => None,
                })
                .collect()
        }
    }

    fn valid_profile(name: &str) -> Profile {
        Profile {
            id: String::new(),
            name: name.into(),
            presence: Presence {
                app_id: "123456789012345678".into(),
                details: Some("Hello".into()),
                ..Default::default()
            },
        }
    }

    fn wait_until(mut f: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(5);
        while !f() {
            assert!(Instant::now() < end, "timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn profile_crud_keeps_active_consistent() {
        let env = Env::new();
        let c = env.controller();
        assert_eq!(c.state().settings.active_profile, None);

        let a = c.create_profile("A").unwrap();
        assert_eq!(c.state().settings.active_profile, Some(a.id.clone()));
        let b = c.create_profile("  ").unwrap();
        assert_eq!(b.name, "New profile");
        assert_eq!(c.state().settings.active_profile, Some(b.id.clone()));

        let copy = c.duplicate_profile(&a.id).unwrap();
        assert_eq!(copy.name, "A copy");
        assert_eq!(c.state().settings.active_profile, Some(b.id.clone()));

        c.set_active_profile(&a.id).unwrap();
        assert_eq!(c.state().settings.active_profile, Some(a.id.clone()));
        assert_eq!(c.set_active_profile("nope").unwrap_err(), MSG_NOT_FOUND);

        // Deleting the active profile moves to another one.
        c.delete_profile(&a.id).unwrap();
        let st = c.state();
        assert_eq!(st.profiles.len(), 2);
        let active = st.settings.active_profile.unwrap();
        assert!(st.profiles.iter().any(|p| p.id == active));
        assert_eq!(c.delete_profile(&a.id).unwrap_err(), MSG_NOT_FOUND);

        // Deleting the rest leaves no active profile.
        for p in c.state().profiles {
            c.delete_profile(&p.id).unwrap();
        }
        assert_eq!(c.state().settings.active_profile, None);
    }

    #[test]
    fn set_settings_cannot_change_active_profile() {
        let env = Env::new();
        let c = env.controller();
        let a = c.create_profile("A").unwrap();
        c.set_settings(Settings {
            active_profile: Some("stale".into()),
            connect_on_launch: true,
            start_in_tray: true,
            autostart: false,
            skip_update_check: false,
        })
        .unwrap();
        let s = c.state().settings;
        assert_eq!(s.active_profile, Some(a.id));
        assert!(s.connect_on_launch && s.start_in_tray);
    }

    #[test]
    fn dangling_active_profile_is_repaired_on_open() {
        let env = Env::new();
        {
            let store = Store::open(env.config.path()).unwrap();
            store
                .set_settings(Settings {
                    active_profile: Some("gone".into()),
                    ..Default::default()
                })
                .unwrap();
        }
        let c = env.controller();
        assert_eq!(c.state().settings.active_profile, None);
    }

    #[test]
    fn showing_needs_a_valid_active_profile() {
        let env = Env::new();
        let c = env.controller();
        assert_eq!(c.set_showing(true).unwrap_err(), MSG_NO_PROFILE);
        assert!(!c.is_showing());

        // New profiles have an empty presence, which is invalid.
        c.create_profile("Empty").unwrap();
        assert_eq!(c.set_showing(true).unwrap_err(), MSG_FIX_FIELDS);
        assert!(!c.is_showing());

        let mut p = c.state().profiles[0].clone();
        p.presence = valid_profile("x").presence;
        assert!(c.save_profile(p).unwrap().is_empty());
        c.set_showing(true).unwrap();
        assert!(c.is_showing());
        c.set_showing(false).unwrap();
        assert!(!c.is_showing());
    }

    #[test]
    fn save_profile_returns_issues_and_does_not_apply_invalid() {
        let env = Env::new();
        let c = env.controller();
        let mut p = c.create_profile("P").unwrap();
        p.presence = valid_profile("x").presence;
        c.save_profile(p.clone()).unwrap();
        c.set_showing(true).unwrap();
        wait_until(|| c.state().engine != EngineState::Idle);

        // Break the presence: issues come back, showing stays on, nothing is re-applied.
        p.presence.app_id = "not digits".into();
        let issues = c.save_profile(p.clone()).unwrap();
        assert!(!issues.is_empty());
        assert!(c.is_showing());
        assert_eq!(c.state().profiles[0].presence.app_id, "not digits");

        // The invalid profile cannot be switched on again.
        c.set_showing(false).unwrap();
        assert_eq!(c.set_showing(true).unwrap_err(), MSG_FIX_FIELDS);
    }

    #[test]
    fn switching_to_an_invalid_profile_hides() {
        let env = Env::new();
        let c = env.controller();
        let mut a = c.create_profile("A").unwrap();
        a.presence = valid_profile("x").presence;
        c.save_profile(a.clone()).unwrap();
        let b = c.create_profile("B").unwrap();
        c.set_active_profile(&a.id).unwrap();
        c.set_showing(true).unwrap();
        c.set_active_profile(&b.id).unwrap();
        assert!(!c.is_showing());
        // The window asked for the switch, not for hiding: it must be told.
        assert_eq!(env.app_changes().last(), Some(&true));
    }

    #[test]
    fn deleting_the_active_profile_while_showing_hides() {
        let env = Env::new();
        let c = env.controller();
        let mut a = c.create_profile("A").unwrap();
        a.presence = valid_profile("x").presence;
        c.save_profile(a.clone()).unwrap();
        c.set_showing(true).unwrap();
        c.delete_profile(&a.id).unwrap();
        assert!(!c.is_showing());
        assert_eq!(env.app_changes().last(), Some(&true));
    }

    #[test]
    fn showing_persists_across_restart() {
        let env = Env::new();
        {
            let c = env.controller();
            let mut p = c.create_profile("P").unwrap();
            p.presence = valid_profile("x").presence;
            c.save_profile(p).unwrap();
            c.set_settings(Settings {
                connect_on_launch: true,
                ..Default::default()
            })
            .unwrap();
            c.set_showing(true).unwrap();
        }
        let c = env.controller();
        assert!(!c.is_showing(), "nothing is restored before start()");
        c.start();
        assert!(c.is_showing());
        assert!(env.config.path().join(STATE_FILE).exists());
        drop(c);

        // Without connect_on_launch the next launch starts hidden.
        let c = env.controller();
        c.set_settings(Settings {
            connect_on_launch: false,
            ..Default::default()
        })
        .unwrap();
        drop(c);
        let c = env.controller();
        c.start();
        assert!(!c.is_showing());
    }

    #[test]
    fn restore_is_skipped_when_profile_became_invalid() {
        let env = Env::new();
        {
            let c = env.controller();
            let mut p = c.create_profile("P").unwrap();
            p.presence = valid_profile("x").presence;
            c.save_profile(p.clone()).unwrap();
            c.set_settings(Settings {
                connect_on_launch: true,
                ..Default::default()
            })
            .unwrap();
            c.set_showing(true).unwrap();
            p.presence.app_id.clear();
            c.save_profile(p).unwrap();
        }
        let c = env.controller();
        c.start();
        assert!(!c.is_showing());
    }

    #[test]
    fn notifications_fire_with_origin() {
        let env = Env::new();
        let c = env.controller();
        c.create_profile("A").unwrap();
        assert_eq!(env.app_changes(), vec![false]);
        let mut p = c.state().profiles[0].clone();
        p.presence = valid_profile("x").presence;
        c.save_profile(p).unwrap();
        c.set_showing_external(true).unwrap();
        c.set_showing_external(false).unwrap();
        c.set_settings(Settings::default()).unwrap();
        assert_eq!(env.app_changes(), vec![false, false, true, true, false]);

        // Failures do not notify.
        let before = env.app_changes().len();
        assert!(c.set_active_profile("nope").is_err());
        assert!(c.delete_profile("nope").is_err());
        assert_eq!(env.app_changes().len(), before);
    }

    #[test]
    fn engine_changes_are_forwarded() {
        let env = Env::new();
        let c = env.controller();
        let mut p = c.create_profile("A").unwrap();
        p.presence = valid_profile("x").presence;
        c.save_profile(p).unwrap();
        c.set_showing(true).unwrap();
        wait_until(|| {
            env.log
                .lock()
                .unwrap()
                .iter()
                .any(|c| matches!(c, Change::Engine(EngineState::WaitingForDiscord)))
        });
        assert_eq!(tooltip(&c.state()), "Decoy: waiting for Discord");
    }

    #[test]
    fn shutdown_leaves_the_engine_idle_and_keeps_showing_persisted() {
        let env = Env::new();
        let c = env.controller();
        let mut p = c.create_profile("A").unwrap();
        p.presence = valid_profile("x").presence;
        c.save_profile(p).unwrap();
        c.set_showing(true).unwrap();
        wait_until(|| c.state().engine == EngineState::WaitingForDiscord);
        c.shutdown();
        assert_eq!(c.state().engine, EngineState::Idle);
        let saved: StateFile =
            serde_json::from_slice(&std::fs::read(env.config.path().join(STATE_FILE)).unwrap())
                .unwrap();
        assert!(saved.showing);
    }

    #[test]
    fn import_export_roundtrip() {
        let env = Env::new();
        let c = env.controller();
        let a = c.create_profile("My: game?").unwrap();
        let (name, json) = c.export_profile(&a.id).unwrap();
        assert_eq!(name, "My: game?");
        let b = c.import_profile(&json).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(c.state().profiles.len(), 2);
        assert!(c.import_profile("[]").is_err());
        assert_eq!(c.export_profile("nope").unwrap_err(), MSG_NOT_FOUND);
    }

    #[test]
    fn tooltips() {
        let env = Env::new();
        let c = env.controller();
        let mut st = c.state();
        assert_eq!(tooltip(&st), "Decoy: hidden");
        let user = decoy_core::DiscordUser {
            id: "1".into(),
            username: "u".into(),
            global_name: None,
            avatar: None,
        };
        st.showing = true;
        st.engine = EngineState::Active { user };
        st.profiles = vec![Profile {
            id: "p".into(),
            name: "Night shift".into(),
            ..Default::default()
        }];
        st.settings.active_profile = Some("p".into());
        assert_eq!(tooltip(&st), "Decoy: showing Night shift");
        st.engine = EngineState::Connecting;
        assert_eq!(tooltip(&st), "Decoy: waiting for Discord");
    }

    #[test]
    fn app_state_json_is_flat() {
        let env = Env::new();
        let c = env.controller();
        let st = AppState::new(
            c.state(),
            GamesInfo {
                count: 3,
                fetched_at: Some(7),
                loading: false,
                error: None,
            },
        );
        let v = serde_json::to_value(&st).unwrap();
        for key in [
            "engine", "showing", "profiles", "settings", "games", "version",
        ] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert_eq!(v["games"]["count"], 3);
        assert_eq!(v["engine"]["status"], "idle");
    }
}
