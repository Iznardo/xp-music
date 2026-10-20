//! Estado de la interfaz que debe sobrevivir a la WebView (principio II) y
//! operaciones de ventana: sonidos, última vista y modo compacto (004).

use std::sync::{Arc, Mutex};

use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, State, WebviewWindow};

use crate::audio::sounds::UiSound;
use crate::error::{AppError, AppResult};
use crate::player::service::PlayerService;
use crate::settings::Settings;
use crate::state::AppState;

/// Tamaño del modo compacto (píxeles lógicos).
const COMPACT: (f64, f64) = (420.0, 118.0);
const MIN_FULL: (f64, f64) = (640.0, 420.0);

/// Tamaño, posición y si estaba maximizada antes de entrar en modo compacto.
type Restore = (PhysicalSize<u32>, PhysicalPosition<i32>, bool);

#[derive(Default)]
pub struct UiState {
    last_route: Mutex<Option<String>>,
    /// Tamaño y posición en modo completo, para volver a ellos.
    restore: Mutex<Option<Restore>>,
    compact: Mutex<bool>,
}

#[tauri::command]
#[specta::specta]
pub fn ui_play_sound(
    player: State<'_, Arc<PlayerService>>,
    app: State<'_, AppState>,
    sound: UiSound,
) {
    if Settings::load(&app.db).map(|s| s.xp_sounds).unwrap_or(true) {
        player.play_sound(sound);
    }
}

#[tauri::command]
#[specta::specta]
pub fn ui_get_last_route(ui: State<'_, UiState>) -> Option<String> {
    ui.last_route.lock().unwrap().clone()
}

#[tauri::command]
#[specta::specta]
pub fn ui_set_last_route(ui: State<'_, UiState>, route: String) {
    *ui.last_route.lock().unwrap() = Some(route);
}

#[tauri::command]
#[specta::specta]
pub fn window_is_compact(ui: State<'_, UiState>) -> bool {
    *ui.compact.lock().unwrap()
}

/// Cambia entre modo completo y compacto (FR-004-13).
#[tauri::command]
#[specta::specta]
pub fn window_set_compact(
    window: WebviewWindow,
    ui: State<'_, UiState>,
    enabled: bool,
) -> AppResult<()> {
    let err = |e: tauri::Error| {
        tracing::warn!(error = %e, "no se pudo cambiar el modo de ventana");
        AppError::internal("No se pudo cambiar el tamaño de la ventana.")
    };
    let mut compact = ui.compact.lock().unwrap();
    if *compact == enabled {
        return Ok(());
    }
    if enabled {
        let maximized = window.is_maximized().map_err(err)?;
        if maximized {
            window.unmaximize().map_err(err)?;
        }
        *ui.restore.lock().unwrap() = Some((
            window.outer_size().map_err(err)?,
            window.outer_position().map_err(err)?,
            maximized,
        ));
        window.set_min_size(None::<LogicalSize<f64>>).map_err(err)?;
        window
            .set_size(LogicalSize::new(COMPACT.0, COMPACT.1))
            .map_err(err)?;
        window.set_resizable(false).map_err(err)?;
        window.set_always_on_top(true).map_err(err)?;
    } else {
        window.set_always_on_top(false).map_err(err)?;
        window.set_resizable(true).map_err(err)?;
        window
            .set_min_size(Some(LogicalSize::new(MIN_FULL.0, MIN_FULL.1)))
            .map_err(err)?;
        if let Some((size, pos, maximized)) = ui.restore.lock().unwrap().take() {
            window.set_size(size).map_err(err)?;
            window.set_position(pos).map_err(err)?;
            if maximized {
                window.maximize().map_err(err)?;
            }
        }
    }
    *compact = enabled;
    Ok(())
}

/// Errores y avisos del frontend al log de la app (la WebView no tiene consola visible).
#[tauri::command]
#[specta::specta]
pub fn ui_log(level: String, message: String) {
    let message = crate::logging::redact(&message);
    match level.as_str() {
        "error" => tracing::error!(target: "ui", "{message}"),
        "warn" => tracing::warn!(target: "ui", "{message}"),
        _ => tracing::info!(target: "ui", "{message}"),
    }
}
