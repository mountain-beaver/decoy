//! Tray icon (StatusNotifierItem over D-Bus, via `ksni`) and process lifecycle.

use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use decoy_app::daemon::{Daemon, Hooks};
use decoy_app::rpc::{self, BindError};
use decoy_app::{tooltip, Controller, DaemonState};
use decoy_core::ConnectOptions;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, MenuItem, StandardItem, SubMenu};
use ksni::{Icon, ToolTip, Tray};

mod icon {
    include!(concat!(env!("OUT_DIR"), "/tray_icon.rs"));
}

const IDENTIFIER: &str = "dev.decoy.app";
/// While the tray is missing (no panel yet), try again this often.
const TRAY_RETRY: Duration = Duration::from_secs(15);
/// Two "open the window" requests closer than this count as one.
const OPEN_DEBOUNCE: Duration = Duration::from_millis(1500);

struct DecoyTray {
    state: DaemonState,
    controller: Arc<Controller>,
    open_ui: Arc<dyn Fn() + Send + Sync>,
    quit: Arc<dyn Fn() + Send + Sync>,
}

impl Tray for DecoyTray {
    fn id(&self) -> String {
        IDENTIFIER.into()
    }

    fn title(&self) -> String {
        "Decoy".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![Icon {
            width: icon::WIDTH,
            height: icon::HEIGHT,
            data: icon::ARGB.to_vec(),
        }]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: tooltip(&self.state),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        (self.open_ui)();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = vec![
            StandardItem {
                label: "Open Decoy".into(),
                activate: Box::new(|t: &mut Self| (t.open_ui)()),
                ..Default::default()
            }
            .into(),
            CheckmarkItem {
                label: "Showing in Discord".into(),
                checked: self.state.showing,
                activate: Box::new(|t: &mut Self| {
                    let on = !t.state.showing;
                    if t.controller.set_showing_external(on).is_err() {
                        // Nothing valid to show yet: let the user fix it in the window.
                        (t.open_ui)();
                    }
                }),
                ..Default::default()
            }
            .into(),
        ];

        if !self.state.profiles.is_empty() {
            let active = self.state.settings.active_profile.as_deref();
            let submenu = self
                .state
                .profiles
                .iter()
                .map(|p| {
                    let id = p.id.clone();
                    let name = p.name.trim();
                    CheckmarkItem {
                        label: if name.is_empty() { "Untitled" } else { name }.into(),
                        checked: active == Some(p.id.as_str()),
                        activate: Box::new(move |t: &mut Self| {
                            let _ = t.controller.set_active_profile_external(&id);
                        }),
                        ..Default::default()
                    }
                    .into()
                })
                .collect();
            items.push(
                SubMenu {
                    label: "Profiles".into(),
                    submenu,
                    ..Default::default()
                }
                .into(),
            );
        }

        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: "Quit Decoy".into(),
                activate: Box::new(|t: &mut Self| (t.quit)()),
                ..Default::default()
            }
            .into(),
        );
        items
    }
}

fn env(key: &str) -> Option<std::ffi::OsString> {
    std::env::var_os(key).filter(|v| !v.is_empty())
}

/// Same directory the UI used before the split: `$XDG_CONFIG_HOME/dev.decoy.app`.
fn config_dir() -> Option<PathBuf> {
    let base = env("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join(IDENTIFIER))
}

/// Start the `decoy` UI binary that sits next to this executable.
fn open_ui() {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    {
        let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
        if last.is_some_and(|t| t.elapsed() < OPEN_DEBOUNCE) {
            return;
        }
        *last = Some(Instant::now());
    }
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("decoy")));
    match exe {
        Some(exe) => {
            if let Err(e) = rpc::spawn_detached(&exe) {
                eprintln!("decoy-daemon: could not start {}: {e}", exe.display());
            }
        }
        None => eprintln!("decoy-daemon: cannot locate the decoy executable"),
    }
}

fn spawn_tray(
    controller: &Arc<Controller>,
    open: &Arc<dyn Fn() + Send + Sync>,
    quit: &Arc<dyn Fn() + Send + Sync>,
) -> Option<Handle<DecoyTray>> {
    let tray = DecoyTray {
        state: controller.state(),
        controller: controller.clone(),
        open_ui: open.clone(),
        quit: quit.clone(),
    };
    // A missing StatusNotifierWatcher is not fatal: the icon shows up as soon as a panel starts.
    match tray.assume_sni_available(true).spawn() {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("decoy-daemon: no tray available yet: {e}");
            None
        }
    }
}

pub fn run() {
    let socket = rpc::socket_path();
    let Some(config) = config_dir() else {
        eprintln!("decoy-daemon: cannot find the config directory (HOME is not set)");
        std::process::exit(1);
    };

    let (refresh_tx, refresh_rx) = channel::<()>();
    let refresh_tx = Mutex::new(refresh_tx);
    let daemon_cell: Arc<OnceLock<Arc<Daemon>>> = Arc::new(OnceLock::new());

    let open: Arc<dyn Fn() + Send + Sync> = Arc::new(open_ui);
    let quit: Arc<dyn Fn() + Send + Sync> = {
        let cell = daemon_cell.clone();
        Arc::new(move || {
            // Clears the presence (drops the Discord connection) before the process ends.
            if let Some(d) = cell.get() {
                d.shutdown();
            }
            std::process::exit(0);
        })
    };
    let hooks = Hooks {
        // The tray is refreshed from its own thread: ksni callbacks run on the
        // tray's thread and must never wait on it.
        on_change: Arc::new(move |_| {
            let _ = refresh_tx.lock().map(|tx| tx.send(()));
        }),
        open_ui: open.clone(),
        quit: quit.clone(),
    };

    let daemon = match Daemon::start(&socket, &config, ConnectOptions::default(), hooks) {
        Ok(d) => Arc::new(d),
        // A daemon is already serving this socket: nothing to do.
        Err(BindError::AlreadyRunning) => return,
        Err(e) => {
            eprintln!("decoy-daemon: {e}");
            std::process::exit(1);
        }
    };
    let _ = daemon_cell.set(daemon.clone());
    let controller = daemon.controller.clone();

    let mut handle = spawn_tray(&controller, &open, &quit);
    loop {
        match refresh_rx.recv_timeout(TRAY_RETRY) {
            Ok(()) => while refresh_rx.try_recv().is_ok() {},
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if handle.as_ref().is_some_and(|h| h.is_closed()) {
            handle = None;
        }
        match &handle {
            Some(h) => {
                let state = controller.state();
                h.update(|t| t.state = state);
            }
            None => handle = spawn_tray(&controller, &open, &quit),
        }
    }
}
