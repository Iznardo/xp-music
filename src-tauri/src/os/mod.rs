//! System integration (specs/006): tray, disposable window, system media
//! controls, notifications, autostart and global shortcuts.

pub mod discord;
pub mod media;
#[cfg(windows)]
pub mod taskbar;
pub mod tray;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::error::{AppError, AppResult};
use crate::player::service::PlayerService;
use crate::settings::Settings;

/// Opens a folder in the system file manager.
pub fn open_dir(dir: &std::path::Path) -> AppResult<()> {
    let opener = if cfg!(windows) {
        "explorer.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::internal(e.to_string()))
}

/// Whether the app is really quitting (not just closing the window to the tray).
#[derive(Default)]
pub struct OsState {
    pub quitting: AtomicBool,
}

/// Shows the main window; if it was destroyed on close, recreates it (FR-006-03).
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let Some(cfg) = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .cloned()
    else {
        tracing::error!("main window configuration is missing");
        return;
    };
    match WebviewWindowBuilder::from_config(app, &cfg).and_then(|b| b.build()) {
        Ok(w) => {
            let _ = w.set_focus();
            // Thumbnail buttons: the window is new, they must be attached again.
            #[cfg(windows)]
            if let Ok(h) = w.hwnd() {
                let raw = h.0 as isize;
                let _ = app.run_on_main_thread(move || taskbar::attach(raw));
            }
            tracing::info!("main window created");
        }
        Err(e) => tracing::error!(error = %e, "could not create the main window"),
    }
}

/// Closes the main window: the WebView and its memory are destroyed with it.
pub fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.close();
    }
}

pub fn main_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// Really quit: saves the queue and ends the process.
pub fn quit(app: &AppHandle) {
    if let Some(os) = app.try_state::<OsState>() {
        os.quitting.store(true, Ordering::Release);
    }
    app.exit(0);
}

/// Rebuilds the OS-level texts (tray menu, taskbar buttons) after a language change.
pub fn relocalize(app: &AppHandle) {
    tray::relocalize(app);
    #[cfg(windows)]
    {
        let _ = app.run_on_main_thread(taskbar::relocalize);
    }
}

/// Applies the settings that touch the system: autostart and global shortcuts.
pub fn apply_settings(app: &AppHandle, s: &Settings) {
    if crate::i18n::set(s.language) {
        relocalize(app);
    }
    let auto = app.autolaunch();
    let enabled = auto.is_enabled().unwrap_or(false);
    let res = match (s.launch_at_login, enabled) {
        (true, false) => auto.enable(),
        (false, true) => auto.disable(),
        _ => Ok(()),
    };
    if let Err(e) = res {
        tracing::warn!(error = %e, "could not change launch at Windows login");
    }

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let Some(player) = app
        .try_state::<Arc<PlayerService>>()
        .map(|p| Arc::clone(&p))
    else {
        return;
    };
    let bind = |accel: &Option<String>, action: fn(&AppHandle, &Arc<PlayerService>)| {
        let Some(accel) = accel.as_deref().filter(|a| !a.trim().is_empty()) else {
            return;
        };
        let p = Arc::clone(&player);
        let res = gs.on_shortcut(accel, move |app, _, ev| {
            if ev.state == ShortcutState::Pressed {
                action(app, &p);
            }
        });
        if let Err(e) = res {
            tracing::warn!(shortcut = accel, error = %e, "could not register the global shortcut");
        }
    };
    bind(&s.hotkeys.play_pause, |_, p| p.toggle());
    bind(&s.hotkeys.next, |_, p| p.next());
    bind(&s.hotkeys.previous, |_, p| p.previous());
    bind(&s.hotkeys.show, |app, _| show_main(app));
}
