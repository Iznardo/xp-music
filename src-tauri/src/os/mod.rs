//! Integración con Windows (specs/006): bandeja, ventana desechable, controles
//! multimedia del sistema, notificaciones, autoarranque y atajos globales.

pub mod media;
pub mod taskbar;
pub mod tray;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::player::service::PlayerService;
use crate::settings::Settings;

/// Si se está saliendo de verdad (no solo cerrando la ventana a la bandeja).
#[derive(Default)]
pub struct OsState {
    pub quitting: AtomicBool,
}

/// Muestra la ventana principal; si se destruyó al cerrar, la vuelve a crear (FR-006-03).
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
        tracing::error!("falta la configuración de la ventana principal");
        return;
    };
    match WebviewWindowBuilder::from_config(app, &cfg).and_then(|b| b.build()) {
        Ok(w) => {
            let _ = w.set_focus();
            // Botones de la miniatura: la ventana es nueva, hay que volver a engancharlos.
            if let Ok(h) = w.hwnd() {
                let raw = h.0 as isize;
                let _ = app.run_on_main_thread(move || taskbar::attach(raw));
            }
            tracing::info!("ventana principal creada");
        }
        Err(e) => tracing::error!(error = %e, "no se pudo crear la ventana principal"),
    }
}

/// Cierra la ventana principal: con ella se destruye la WebView y su memoria.
pub fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.close();
    }
}

pub fn main_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false))
}

/// Salir de verdad: guarda la cola y termina el proceso.
pub fn quit(app: &AppHandle) {
    if let Some(os) = app.try_state::<OsState>() {
        os.quitting.store(true, Ordering::Release);
    }
    app.exit(0);
}

/// Aplica los ajustes que tocan el sistema: autoarranque y atajos globales.
pub fn apply_settings(app: &AppHandle, s: &Settings) {
    let auto = app.autolaunch();
    let enabled = auto.is_enabled().unwrap_or(false);
    let res = match (s.launch_at_login, enabled) {
        (true, false) => auto.enable(),
        (false, true) => auto.disable(),
        _ => Ok(()),
    };
    if let Err(e) = res {
        tracing::warn!(error = %e, "no se pudo cambiar el inicio con Windows");
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
            tracing::warn!(atajo = accel, error = %e, "no se pudo registrar el atajo global");
        }
    };
    bind(&s.hotkeys.play_pause, |_, p| p.toggle());
    bind(&s.hotkeys.next, |_, p| p.next());
    bind(&s.hotkeys.previous, |_, p| p.previous());
    bind(&s.hotkeys.show, |app, _| show_main(app));
}
