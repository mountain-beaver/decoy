//! One interface for everything the UI asks of the app logic, with two
//! implementations: the controller in this process (Windows, macOS) and the
//! tray daemon over the local socket (Linux). `commands.rs` does not care which.

use std::sync::Arc;

use decoy_app::{Controller, DaemonState};
use decoy_core::{Issue, Presence, Profile, Settings};

pub trait Backend: Send + Sync {
    fn state(&self) -> Result<DaemonState, String>;
    fn save_profile(&self, profile: Profile) -> Result<Vec<Issue>, String>;
    fn create_profile(&self, name: &str) -> Result<Profile, String>;
    fn duplicate_profile(&self, id: &str) -> Result<Profile, String>;
    fn delete_profile(&self, id: &str) -> Result<(), String>;
    fn set_active_profile(&self, id: &str) -> Result<(), String>;
    /// The profile's name and its export JSON.
    fn export_profile(&self, id: &str) -> Result<(String, String), String>;
    fn import_profile(&self, json: &str) -> Result<Profile, String>;
    fn set_showing(&self, on: bool) -> Result<(), String>;
    fn validate(&self, presence: &Presence) -> Result<Vec<Issue>, String>;
    fn set_settings(&self, settings: Settings) -> Result<(), String>;
}

/// The controller lives in this process.
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub struct Local(pub Arc<Controller>);

impl Backend for Local {
    fn state(&self) -> Result<DaemonState, String> {
        Ok(self.0.state())
    }
    fn save_profile(&self, profile: Profile) -> Result<Vec<Issue>, String> {
        self.0.save_profile(profile)
    }
    fn create_profile(&self, name: &str) -> Result<Profile, String> {
        self.0.create_profile(name)
    }
    fn duplicate_profile(&self, id: &str) -> Result<Profile, String> {
        self.0.duplicate_profile(id)
    }
    fn delete_profile(&self, id: &str) -> Result<(), String> {
        self.0.delete_profile(id)
    }
    fn set_active_profile(&self, id: &str) -> Result<(), String> {
        self.0.set_active_profile(id)
    }
    fn export_profile(&self, id: &str) -> Result<(String, String), String> {
        self.0.export_profile(id)
    }
    fn import_profile(&self, json: &str) -> Result<Profile, String> {
        self.0.import_profile(json)
    }
    fn set_showing(&self, on: bool) -> Result<(), String> {
        self.0.set_showing(on)
    }
    fn validate(&self, presence: &Presence) -> Result<Vec<Issue>, String> {
        Ok(self.0.validate(presence))
    }
    fn set_settings(&self, settings: Settings) -> Result<(), String> {
        self.0.set_settings(settings)
    }
}

/// The controller lives in the tray daemon; talk to it over the socket.
#[cfg(target_os = "linux")]
pub struct Remote(pub decoy_app::rpc::Client);

#[cfg(target_os = "linux")]
impl Backend for Remote {
    fn state(&self) -> Result<DaemonState, String> {
        self.0.call("get_state", serde_json::Value::Null)
    }
    fn save_profile(&self, profile: Profile) -> Result<Vec<Issue>, String> {
        self.0
            .call("save_profile", serde_json::json!({ "profile": profile }))
    }
    fn create_profile(&self, name: &str) -> Result<Profile, String> {
        self.0
            .call("create_profile", serde_json::json!({ "name": name }))
    }
    fn duplicate_profile(&self, id: &str) -> Result<Profile, String> {
        self.0
            .call("duplicate_profile", serde_json::json!({ "id": id }))
    }
    fn delete_profile(&self, id: &str) -> Result<(), String> {
        self.0
            .call("delete_profile", serde_json::json!({ "id": id }))
    }
    fn set_active_profile(&self, id: &str) -> Result<(), String> {
        self.0
            .call("set_active_profile", serde_json::json!({ "id": id }))
    }
    fn export_profile(&self, id: &str) -> Result<(String, String), String> {
        #[derive(serde::Deserialize)]
        struct Exported {
            name: String,
            json: String,
        }
        let e: Exported = self
            .0
            .call("export_profile", serde_json::json!({ "id": id }))?;
        Ok((e.name, e.json))
    }
    fn import_profile(&self, json: &str) -> Result<Profile, String> {
        self.0
            .call("import_profile", serde_json::json!({ "json": json }))
    }
    fn set_showing(&self, on: bool) -> Result<(), String> {
        self.0.call("set_showing", serde_json::json!({ "on": on }))
    }
    fn validate(&self, presence: &Presence) -> Result<Vec<Issue>, String> {
        self.0
            .call("validate", serde_json::json!({ "presence": presence }))
    }
    fn set_settings(&self, settings: Settings) -> Result<(), String> {
        self.0
            .call("set_settings", serde_json::json!({ "settings": settings }))
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use decoy_app::daemon::{Daemon, Hooks};
    use decoy_app::rpc::Connection;
    use decoy_core::ConnectOptions;
    use std::sync::Arc;

    /// The remote backend behaves like the local one, against a real in-process daemon.
    #[test]
    fn remote_and_local_backends_agree() {
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("decoy.sock");
        let daemon = Daemon::start(
            &socket,
            &dir.path().join("remote-config"),
            ConnectOptions {
                // Never the real Discord.
                paths: Some(vec![dir.path().join("no-discord")]),
                ..Default::default()
            },
            Hooks {
                on_change: Arc::new(|_| {}),
                open_ui: Arc::new(|| {}),
                quit: Arc::new(|| {}),
            },
        )
        .unwrap();
        let remote = Remote(Connection::open(&socket).unwrap().start(|_, _| {}, || {}));
        let local = Local(
            decoy_app::Controller::new(
                &dir.path().join("local-config"),
                ConnectOptions {
                    paths: Some(vec![dir.path().join("no-discord")]),
                    ..Default::default()
                },
                Arc::new(|_| {}),
            )
            .unwrap(),
        );

        for be in [&remote as &dyn Backend, &local as &dyn Backend] {
            let p = be.create_profile("Game").unwrap();
            let mut edited = p.clone();
            edited.presence.app_id = "123456789012345678".into();
            assert!(be.save_profile(edited.clone()).unwrap().is_empty());
            edited.presence.app_id = "bad".into();
            assert!(!be.save_profile(edited).unwrap().is_empty());
            assert!(!be.validate(&Presence::default()).unwrap().is_empty());
            let copy = be.duplicate_profile(&p.id).unwrap();
            be.set_active_profile(&copy.id).unwrap();
            let (name, json) = be.export_profile(&copy.id).unwrap();
            assert_eq!(name, "Game copy");
            assert_eq!(be.import_profile(&json).unwrap().name, "Game copy");
            assert_eq!(
                be.set_showing(true).unwrap_err(),
                "Fix the highlighted fields first"
            );
            be.set_settings(Settings {
                start_in_tray: true,
                ..Default::default()
            })
            .unwrap();
            let st = be.state().unwrap();
            assert_eq!(st.profiles.len(), 3);
            assert!(st.settings.start_in_tray);
            assert_eq!(
                st.settings.active_profile.as_deref(),
                Some(copy.id.as_str())
            );
            be.delete_profile(&p.id).unwrap();
            assert_eq!(be.state().unwrap().profiles.len(), 2);
            assert!(be.delete_profile("nope").is_err());
        }
        drop(daemon);
    }
}
