//! Linux only: this process is just the window. The tray, the profiles and the
//! Discord connection live in `decoy-daemon`; here we find it (or start it) and
//! talk to it over the local socket.

use std::path::PathBuf;

use decoy_app::rpc::{self, Client};
use gtk::prelude::GtkSettingsExt;
use serde_json::Value;

/// The AppImage file this process was started from, if any.
fn appimage() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
}

/// The daemon to start: `decoy-daemon` next to this executable (same folder in
/// `target/` and in a .deb or .rpm).
///
/// An AppImage is different: its files live in a mount that goes away when this
/// process exits, which would pull the binary from under the running daemon. So
/// from an AppImage the daemon runs as a copy in the user's data directory.
fn daemon_exe() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let bundled = exe
        .parent()
        .map(|d| d.join("decoy-daemon"))
        .ok_or_else(|| std::io::Error::other("cannot locate the Decoy background service"))?;
    if appimage().is_none() {
        return Ok(bundled);
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .ok_or_else(|| std::io::Error::other("HOME is not set"))?
        .join("dev.decoy.app");
    std::fs::create_dir_all(&data)?;
    let copy = data.join("decoy-daemon");
    // Copy aside and rename: a daemon still running from the old file keeps it.
    let fresh = data.join(format!(".decoy-daemon.{}", std::process::id()));
    std::fs::copy(&bundled, &fresh)?;
    std::fs::rename(&fresh, &copy)?;
    Ok(copy)
}

/// Start the daemon and tell it how to open the window later: through the
/// AppImage file when there is one, else through this executable.
fn spawn_daemon() -> std::io::Result<()> {
    let daemon = daemon_exe()?;
    let ui = match appimage() {
        Some(file) => file,
        None => std::env::current_exe()?,
    };
    rpc::spawn_detached_with(&daemon, &[(rpc::ENV_UI, ui.as_os_str())])
}

/// Connect to the daemon, starting it if needed (detached, so it outlives this process).
pub fn connect(
    on_event: impl Fn(&str, Value) + Send + 'static,
    on_close: impl FnOnce() + Send + 'static,
) -> Result<Client, String> {
    rpc::ensure_daemon(
        &rpc::socket_path(),
        decoy_app::VERSION,
        &spawn_daemon,
        on_event,
        on_close,
    )
}

/// `decoy --hidden` (autostart): make sure the daemon runs, create no window.
pub fn ensure_daemon_only() {
    if let Err(e) = connect(|_, _| {}, || {}) {
        eprintln!("decoy: {e}");
    }
}

/// Without GSettings schemas (a bare window manager, a shell without
/// gsettings-desktop-schemas) GDK reports `gtk-xft-dpi` as -1. WebKitGTK then
/// derives devicePixelRatio = -1/96 and a negative viewport, and the whole UI
/// collapses into a sliver. Fall back to the usual 96 dpi (GTK stores dpi * 1024).
/// Must run on the main thread, after GTK is initialised and before the window is built.
pub fn fix_broken_dpi() {
    if let Some(settings) = gtk::Settings::default() {
        if settings.gtk_xft_dpi() <= 0 {
            settings.set_gtk_xft_dpi(96 * 1024);
        }
    }
}

/// WebKitGTK paints on the CPU unless a page needs compositing, and scrolls the
/// mouse wheel in jumps. Ask for GPU compositing and animated wheel scrolling.
/// `DECOY_GPU=auto|off` restores the old behaviour if a driver misbehaves.
pub fn tune_webview(window: &tauri::WebviewWindow) {
    use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
    let policy = match std::env::var("DECOY_GPU").as_deref() {
        Ok("off") => HardwareAccelerationPolicy::Never,
        Ok("auto") => HardwareAccelerationPolicy::OnDemand,
        _ => HardwareAccelerationPolicy::Always,
    };
    let _ = window.with_webview(move |webview| {
        if let Some(settings) = WebViewExt::settings(&webview.inner()) {
            settings.set_hardware_acceleration_policy(policy);
            settings.set_enable_smooth_scrolling(true);
            // WebKit holds pages "near 60 fps" even on faster displays (82 on a 165 Hz one).
            set_feature(&settings, "PreferPageRenderingUpdatesNear60FPS", false);
            // Development knob: DECOY_DEV_FEATURES="+Name,-Other" switches WebKit features.
            if let Ok(list) = std::env::var("DECOY_DEV_FEATURES") {
                for item in list.split(',').filter(|i| i.len() > 1) {
                    set_feature(&settings, &item[1..], item.starts_with('+'));
                }
            }
        }
    });
}

/// Switch a WebKit feature by its identifier. The feature API exists since
/// WebKitGTK 2.42, so the functions are looked up at run time and an older
/// library (or an unknown identifier) is simply left alone.
fn set_feature(settings: &webkit2gtk::Settings, identifier: &str, enabled: bool) {
    use gtk::glib::object::ObjectType;
    use std::ffi::{c_char, c_int, c_void, CStr};
    type AllFeatures = unsafe extern "C" fn() -> *mut c_void;
    type Length = unsafe extern "C" fn(*mut c_void) -> usize;
    type Get = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
    type Identifier = unsafe extern "C" fn(*mut c_void) -> *const c_char;
    type SetEnabled = unsafe extern "C" fn(*mut c_void, *mut c_void, c_int);
    type Unref = unsafe extern "C" fn(*mut c_void);
    unsafe {
        let sym = |name: &CStr| libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr());
        let symbols = [
            sym(c"webkit_settings_get_all_features"),
            sym(c"webkit_feature_list_get_length"),
            sym(c"webkit_feature_list_get"),
            sym(c"webkit_feature_get_identifier"),
            sym(c"webkit_settings_set_feature_enabled"),
            sym(c"webkit_feature_list_unref"),
        ];
        if symbols.iter().any(|s| s.is_null()) {
            return;
        }
        let all: AllFeatures = std::mem::transmute(symbols[0]);
        let length: Length = std::mem::transmute(symbols[1]);
        let get: Get = std::mem::transmute(symbols[2]);
        let id: Identifier = std::mem::transmute(symbols[3]);
        let set: SetEnabled = std::mem::transmute(symbols[4]);
        let unref: Unref = std::mem::transmute(symbols[5]);

        let list = all();
        if list.is_null() {
            return;
        }
        let dump = std::env::var_os("DECOY_FEATURES").is_some();
        for i in 0..length(list) {
            let feature = get(list, i);
            if feature.is_null() {
                continue;
            }
            let name = id(feature);
            if name.is_null() {
                continue;
            }
            let name = CStr::from_ptr(name).to_string_lossy();
            if dump {
                println!("feature {name}");
            }
            if name == identifier {
                set(settings.as_ptr() as *mut c_void, feature, enabled as c_int);
            }
        }
        unref(list);
    }
}
