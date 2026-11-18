//! Thin `#[tauri::command]` wrappers. Profile and presence work goes through
//! [`Backend`] (local controller or tray daemon); the game list, Discord lookups,
//! dialogs and links always stay in this process.

use std::sync::Arc;

use decoy_app::shell::{self, check_external_url, export_file_name, AppLookup, AssetInfo};
use decoy_app::{AppState, GameHit, Games};
use decoy_core::{Issue, Presence, Profile, Settings};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_dialog::DialogExt as _;

use crate::backend::Backend;

type Be<'a> = State<'a, Arc<dyn Backend>>;

#[tauri::command]
pub fn get_state(be: Be, games: State<'_, Arc<Games>>) -> Result<AppState, String> {
    Ok(AppState::new(be.state()?, games.info()))
}

#[tauri::command]
pub fn save_profile(be: Be, profile: Profile) -> Result<Vec<Issue>, String> {
    be.save_profile(profile)
}

#[tauri::command]
pub fn create_profile(be: Be, name: String) -> Result<Profile, String> {
    be.create_profile(&name)
}

#[tauri::command]
pub fn duplicate_profile(be: Be, id: String) -> Result<Profile, String> {
    be.duplicate_profile(&id)
}

#[tauri::command]
pub fn delete_profile(be: Be, id: String) -> Result<(), String> {
    be.delete_profile(&id)
}

#[tauri::command]
pub fn set_active_profile(be: Be, id: String) -> Result<(), String> {
    be.set_active_profile(&id)
}

/// Native save dialog; `None` when cancelled. Async so the blocking dialog never
/// runs on the main thread.
#[tauri::command]
pub async fn export_profile_to_file(
    app: AppHandle,
    be: Be<'_>,
    id: String,
) -> Result<Option<String>, String> {
    let (name, json) = be.export_profile(&id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app
            .dialog()
            .file()
            .set_file_name(export_file_name(&name))
            .add_filter("Decoy profile", &["json"]);
        if let Some(w) = app.get_webview_window(crate::WINDOW_LABEL) {
            dialog = dialog.set_parent(&w);
        }
        let Some(path) = dialog.blocking_save_file() else {
            return Ok(None);
        };
        let path = path.into_path().map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| format!("Could not write the file: {e}"))?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn import_profile(be: Be, json: String) -> Result<Profile, String> {
    be.import_profile(&json)
}

#[tauri::command]
pub fn set_showing(be: Be, on: bool) -> Result<(), String> {
    be.set_showing(on)
}

#[tauri::command]
pub fn validate(be: Be, presence: Presence) -> Result<Vec<Issue>, String> {
    be.validate(&presence)
}

#[tauri::command]
pub fn search_games(games: State<'_, Arc<Games>>, query: String, limit: usize) -> Vec<GameHit> {
    games.search(&query, limit)
}

#[tauri::command]
pub fn refresh_games(games: State<'_, Arc<Games>>) {
    games.inner().refresh();
}

#[tauri::command]
pub async fn lookup_app(app_id: String) -> Result<AppLookup, String> {
    tauri::async_runtime::spawn_blocking(move || shell::lookup_app(&app_id))
        .await
        .map_err(|e| e.to_string())?
}

/// A newer release, if there is one and the user did not switch the check off.
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    be: Be<'_>,
) -> Result<Option<decoy_core::Update>, String> {
    if be.state()?.settings.skip_update_check {
        return Ok(None);
    }
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        shell::check_update(&cache_dir, env!("CARGO_PKG_VERSION"))
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_assets(app_id: String) -> Result<Vec<AssetInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || shell::list_assets(&app_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn set_settings(app: AppHandle, be: Be, settings: Settings) -> Result<(), String> {
    // OS autostart first: if it fails the settings stay as they were.
    if settings.autostart != be.state()?.settings.autostart {
        set_os_autostart(&app, settings.autostart)?;
    }
    be.set_settings(settings)
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    check_external_url(&url)?;
    tauri_plugin_opener::open_url(url.trim(), None::<&str>).map_err(|e| e.to_string())
}

pub fn set_os_autostart(app: &AppHandle, on: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let result = if on {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| format!("Could not change the launch at login setting: {e}"))
}
