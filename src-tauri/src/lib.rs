//! Tauri shell of Decoy: window lifecycle and plugins around the app logic.
//!
//! * Windows and macOS: one process. The controller and the tray live here and
//!   the process keeps running without a window.
//! * Linux: this process is only the window. It talks to `decoy-daemon` (which
//!   owns the tray and the presence) and exits when the window closes.

mod backend;
mod commands;
#[cfg(target_os = "linux")]
mod linux;
mod perf;
#[cfg(not(target_os = "linux"))]
mod tray;
#[cfg(target_os = "linux")]
mod vblank;

use std::sync::Arc;

use decoy_app::{AppState, DaemonState, Games};
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt as _};

use backend::Backend;

pub(crate) const WINDOW_LABEL: &str = "main";
const HIDDEN_ARG: &str = "--hidden";

/// Open the main window, or focus it if it exists. The window is created on
/// demand and destroyed when closed, so an idle app holds no webview.
pub fn show_main_window(app: &AppHandle) {
    let handle = app.clone();
    // Window creation belongs on the main thread (avoids deadlocks on Windows).
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(WINDOW_LABEL) {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
        #[cfg(target_os = "linux")]
        linux::fix_broken_dpi();
        let built = WebviewWindowBuilder::new(
            &handle,
            WINDOW_LABEL,
            WebviewUrl::App(perf::start_page().into()),
        )
        .title("Decoy")
        .inner_size(perf::window_size().0, perf::window_size().1)
        .min_inner_size(780.0, 520.0)
        .center()
        .decorations(true)
        .on_navigation(allow_navigation);
        let built = if perf::enabled() {
            built.initialization_script(perf::SCRIPT)
        } else {
            built
        };
        match built.build() {
            #[cfg(target_os = "linux")]
            Ok(window) => linux::tune_webview(&window),
            #[cfg(not(target_os = "linux"))]
            Ok(_) => {}
            Err(e) => eprintln!("decoy: could not open the window: {e}"),
        }
    });
}

/// Keep the webview on the app itself; web links go to the system browser.
fn allow_navigation(url: &Url) -> bool {
    let local = matches!(url.scheme(), "tauri" | "about" | "data" | "blob")
        || matches!(
            url.host_str(),
            Some("localhost" | "tauri.localhost" | "127.0.0.1")
        );
    if !local && matches!(url.scheme(), "http" | "https") {
        let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
    }
    local
}

fn window_exists(app: &AppHandle) -> bool {
    app.get_webview_window(WINDOW_LABEL).is_some()
}

/// Send the window a fresh `app-state`, if there is a window.
fn emit_app_state(app: &AppHandle, state: DaemonState) {
    if !window_exists(app) {
        return;
    }
    if let Some(games) = app.try_state::<Arc<Games>>() {
        let _ = app.emit_to(
            WINDOW_LABEL,
            "app-state",
            AppState::new(state, games.info()),
        );
    }
}

/// The game list changed (loading started or finished, error).
fn on_games_changed(app: &AppHandle) {
    if !window_exists(app) {
        return;
    }
    if let Some(Ok(state)) = app.try_state::<Arc<dyn Backend>>().map(|b| b.state()) {
        emit_app_state(app, state);
    }
}

#[cfg(not(target_os = "linux"))]
mod single_process {
    use super::*;
    use decoy_app::{Change, Controller};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Set once "Quit Decoy" was chosen; only then may the process exit.
    pub static QUITTING: AtomicBool = AtomicBool::new(false);

    /// Clear the presence and exit.
    pub fn quit(app: &AppHandle) {
        QUITTING.store(true, Ordering::SeqCst);
        if let Some(ctrl) = app.try_state::<Arc<Controller>>() {
            // Waits until the engine dropped the Discord connection.
            ctrl.shutdown();
        }
        app.exit(0);
    }

    pub fn on_change(app: &AppHandle, change: Change) {
        match change {
            Change::Engine(state) => {
                if window_exists(app) {
                    let _ = app.emit_to(WINDOW_LABEL, "engine-state", state);
                }
                tray::refresh(app, false);
            }
            Change::App { external } => {
                if external {
                    if let Some(ctrl) = app.try_state::<Arc<Controller>>() {
                        emit_app_state(app, ctrl.state());
                    }
                }
                tray::refresh(app, true);
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) use single_process::quit;

/// Linux: the daemon pushed an event (or went away).
#[cfg(target_os = "linux")]
fn on_daemon_event(app: &AppHandle, event: &str, payload: serde_json::Value) {
    match event {
        decoy_app::rpc::EVENT_ENGINE_STATE if window_exists(app) => {
            let _ = app.emit_to(WINDOW_LABEL, "engine-state", payload);
        }
        decoy_app::rpc::EVENT_APP_STATE => {
            if let Ok(state) = serde_json::from_value::<DaemonState>(payload) {
                emit_app_state(app, state);
            }
        }
        _ => {}
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let hidden_arg = std::env::args().any(|a| a == HIDDEN_ARG);
    #[cfg(target_os = "linux")]
    vblank::init();

    // Autostart on Linux: no window and no tray here, just make sure the daemon runs.
    #[cfg(target_os = "linux")]
    if hidden_arg {
        linux::ensure_daemon_only();
        return;
    }

    let mut builder = tauri::Builder::default();
    // Must be the first plugin. The frame-rate self-test runs next to a normal
    // instance, so it does not take part.
    if !perf::isolated() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|a| a == HIDDEN_ARG) {
                show_main_window(app);
            }
        }));
    }
    let app = builder
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_profile,
            commands::create_profile,
            commands::duplicate_profile,
            commands::delete_profile,
            commands::set_active_profile,
            commands::export_profile_to_file,
            commands::import_profile,
            commands::set_showing,
            commands::validate,
            commands::search_games,
            commands::refresh_games,
            commands::lookup_app,
            commands::list_assets,
            commands::set_settings,
            commands::open_external,
            perf::perf_report,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let cache_dir = app.path().app_cache_dir()?;

            let games_handle = handle.clone();
            let games = Games::new(
                &cache_dir,
                Arc::new(move || on_games_changed(&games_handle)),
            );
            app.manage(games.clone());

            // Linux: the daemon holds the state. Elsewhere: the controller lives here.
            #[cfg(target_os = "linux")]
            let backend: Arc<dyn Backend> = {
                let (event_handle, close_handle) = (handle.clone(), handle.clone());
                let client = linux::connect(
                    move |event, payload| on_daemon_event(&event_handle, event, payload),
                    // The daemon quit (tray "Quit Decoy"): nothing left to show.
                    move || close_handle.exit(0),
                )?;
                Arc::new(backend::Remote(client))
            };
            #[cfg(not(target_os = "linux"))]
            let backend: Arc<dyn Backend> = {
                let config_dir = app.path().app_config_dir()?;
                let notify_handle = handle.clone();
                let ctrl = decoy_app::Controller::new(
                    &config_dir,
                    decoy_core::ConnectOptions::default(),
                    Arc::new(move |c: Change| single_process::on_change(&notify_handle, c)),
                )?;
                app.manage(ctrl.clone());
                Arc::new(backend::Local(ctrl))
            };
            app.manage(backend.clone());

            #[cfg(not(target_os = "linux"))]
            {
                tray::build(&handle)?;
                if let Some(ctrl) = app.try_state::<Arc<decoy_app::Controller>>() {
                    ctrl.start();
                }
            }
            games.start();

            // Keep the OS entry pointing at this executable if the user wants autostart.
            let settings = backend.state()?.settings;
            if settings.autostart && !handle.autolaunch().is_enabled().unwrap_or(true) {
                let _ = commands::set_os_autostart(&handle, true);
            }

            // On Linux `--hidden` returned above, so a launch here always opens the window.
            if cfg!(target_os = "linux") || (!hidden_arg && !settings.start_in_tray) {
                show_main_window(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Decoy");

    app.run(|app, event| {
        // Windows and macOS: closing the last window must not end the process;
        // only "Quit" does. Linux: the process ends with its window.
        #[cfg(not(target_os = "linux"))]
        match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                if code.is_none()
                    && !single_process::QUITTING.load(std::sync::atomic::Ordering::SeqCst)
                {
                    api.prevent_exit();
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show_main_window(app),
            _ => {
                let _ = app;
            }
        }
        #[cfg(target_os = "linux")]
        let _ = (app, event);
    });
}
