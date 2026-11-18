//! Tray icon and menu. The menu is rebuilt whenever the controller reports a change.

use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuEvent, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use decoy_app::{tooltip, Controller, DaemonState};

const TRAY_ID: &str = "main";
const TRAY_PNG: &[u8] = include_bytes!("../../assets/tray.png");
const ID_OPEN: &str = "open";
const ID_SHOWING: &str = "showing";
const ID_QUIT: &str = "quit";
const ID_PROFILE_PREFIX: &str = "profile:";

fn controller(app: &AppHandle) -> Option<Arc<Controller>> {
    app.try_state::<Arc<Controller>>()
        .map(|s| s.inner().clone())
}

fn build_menu(app: &AppHandle, state: &DaemonState) -> tauri::Result<Menu<tauri::Wry>> {
    let showing = CheckMenuItem::with_id(
        app,
        ID_SHOWING,
        "Showing in Discord",
        true,
        state.showing,
        None::<&str>,
    )?;
    let mut menu = MenuBuilder::new(app)
        .text(ID_OPEN, "Open Decoy")
        .item(&showing);

    if !state.profiles.is_empty() {
        let mut sub = SubmenuBuilder::new(app, "Profiles");
        for p in &state.profiles {
            let name = p.name.trim();
            let label = if name.is_empty() { "Untitled" } else { name };
            let active = state.settings.active_profile.as_deref() == Some(p.id.as_str());
            let item = CheckMenuItem::with_id(
                app,
                format!("{ID_PROFILE_PREFIX}{}", p.id),
                label,
                true,
                active,
                None::<&str>,
            )?;
            sub = sub.item(&item);
        }
        menu = menu.item(&sub.build()?);
    }

    menu.separator().text(ID_QUIT, "Quit Decoy").build()
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let state = controller(app).map(|c| c.state());
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(TRAY_PNG)?)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main_window(tray.app_handle());
            }
        });
    if let Some(state) = &state {
        builder = builder
            .menu(&build_menu(app, state)?)
            .tooltip(tooltip(state));
    }
    builder.build(app)?;
    Ok(())
}

/// Update the tooltip, and the menu too when `rebuild_menu` is set. Safe to call
/// from any thread: the work is queued to the main thread and never waited for.
pub fn refresh(app: &AppHandle, rebuild_menu: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let (Some(tray), Some(ctrl)) = (handle.tray_by_id(TRAY_ID), controller(&handle)) else {
            return;
        };
        let state = ctrl.state();
        let _ = tray.set_tooltip(Some(tooltip(&state)));
        if rebuild_menu {
            if let Ok(menu) = build_menu(&handle, &state) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    });
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    let Some(ctrl) = controller(app) else { return };
    let id = event.id.as_ref();
    match id {
        ID_OPEN => crate::show_main_window(app),
        ID_SHOWING => {
            if ctrl.set_showing_external(!ctrl.is_showing()).is_err() {
                // Nothing valid to show yet: let the user fix it in the window.
                crate::show_main_window(app);
            }
        }
        ID_QUIT => crate::quit(app),
        _ => {
            if let Some(profile_id) = id.strip_prefix(ID_PROFILE_PREFIX) {
                let _ = ctrl.set_active_profile_external(profile_id);
            }
        }
    }
}
