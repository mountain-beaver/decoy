//! The daemon core: controller + RPC server, without any tray or GUI code.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use decoy_core::{ConnectOptions, Presence, Profile, Settings};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::controller::{Change, Controller};
use crate::rpc::{BindError, Server, Service, EVENT_APP_STATE, EVENT_ENGINE_STATE};

/// Callbacks into the process that hosts the daemon (tray, exit).
#[derive(Clone)]
pub struct Hooks {
    /// After every change, once it was broadcast to the clients (the tray refreshes here).
    pub on_change: Arc<dyn Fn(&Change) + Send + Sync>,
    /// Open the UI (the tray's "Open Decoy", or the `open_ui` command).
    pub open_ui: Arc<dyn Fn() + Send + Sync>,
    /// The `quit` command was answered; stop the process.
    pub quit: Arc<dyn Fn() + Send + Sync>,
}

pub struct Daemon {
    pub controller: Arc<Controller>,
    pub server: Server,
}

impl Daemon {
    /// Claim the socket, open the profile store and start serving.
    pub fn start(
        socket: &Path,
        config_dir: &Path,
        opts: ConnectOptions,
        hooks: Hooks,
    ) -> Result<Daemon, BindError> {
        let server = Server::bind(socket)?;
        let slot: Arc<OnceLock<Arc<Controller>>> = Arc::new(OnceLock::new());
        let notify = {
            let server = server.clone();
            let slot = slot.clone();
            let on_change = hooks.on_change.clone();
            move |change: Change| {
                match &change {
                    Change::Engine(state) => server.broadcast(EVENT_ENGINE_STATE, state),
                    Change::App { external: true } => {
                        if let Some(ctrl) = slot.get() {
                            server.broadcast(EVENT_APP_STATE, &ctrl.state());
                        }
                    }
                    Change::App { external: false } => {}
                }
                on_change(&change);
            }
        };
        let controller = Controller::new(config_dir, opts, Arc::new(notify))
            .map_err(|e| BindError::Io(std::io::Error::other(e)))?;
        let _ = slot.set(controller.clone());
        controller.start();
        server.serve(Arc::new(DaemonService {
            controller: controller.clone(),
            hooks,
        }));
        Ok(Daemon { controller, server })
    }

    /// Clear the presence (by dropping the Discord connection) and release the socket.
    pub fn shutdown(&self) {
        self.server.close();
        self.controller.shutdown();
    }
}

struct DaemonService {
    controller: Arc<Controller>,
    hooks: Hooks,
}

fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, String> {
    let v = args.get(key).cloned().unwrap_or(Value::Null);
    serde_json::from_value(v).map_err(|e| format!("Bad argument `{key}`: {e}"))
}

fn to_json<T: serde::Serialize>(v: T) -> Result<Value, String> {
    serde_json::to_value(v).map_err(|e| e.to_string())
}

impl Service for DaemonService {
    fn hello(&self) -> Value {
        json!({ "version": crate::VERSION, "pid": std::process::id() })
    }

    fn call(&self, cmd: &str, args: Value) -> Result<Value, String> {
        let c = &self.controller;
        match cmd {
            "get_state" => to_json(c.state()),
            "save_profile" => to_json(c.save_profile(arg::<Profile>(&args, "profile")?)?),
            "create_profile" => to_json(c.create_profile(&arg::<String>(&args, "name")?)?),
            "duplicate_profile" => to_json(c.duplicate_profile(&arg::<String>(&args, "id")?)?),
            "delete_profile" => to_json(c.delete_profile(&arg::<String>(&args, "id")?)?),
            "set_active_profile" => to_json(c.set_active_profile(&arg::<String>(&args, "id")?)?),
            "export_profile" => {
                let (name, json) = c.export_profile(&arg::<String>(&args, "id")?)?;
                Ok(json!({ "name": name, "json": json }))
            }
            "import_profile" => to_json(c.import_profile(&arg::<String>(&args, "json")?)?),
            "set_showing" => to_json(c.set_showing(arg::<bool>(&args, "on")?)?),
            "validate" => to_json(c.validate(&arg::<Presence>(&args, "presence")?)),
            "set_settings" => to_json(c.set_settings(arg::<Settings>(&args, "settings")?)?),
            "open_ui" => {
                (self.hooks.open_ui)();
                Ok(Value::Null)
            }
            "quit" => Ok(Value::Null),
            other => Err(format!("Unknown command: {other}")),
        }
    }

    fn after_reply(&self, cmd: &str) {
        if cmd == "quit" {
            (self.hooks.quit)();
        }
    }
}
