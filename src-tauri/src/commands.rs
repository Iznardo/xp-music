//! Comandos Tauri expuestos al frontend. Convención: `area_accion`.

use tauri::State;
use tauri_specta::Event;

use crate::error::AppResult;
use crate::settings::Settings;
use crate::state::AppState;

#[tauri::command]
#[specta::specta]
pub fn settings_get(state: State<'_, AppState>) -> AppResult<Settings> {
    Settings::load(&state.db)
}

/// Sustituye los ajustes completos y avisa a todas las ventanas.
#[tauri::command]
#[specta::specta]
pub fn settings_set(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let saved = settings.save(&state.db)?;
    if let Err(e) = saved.clone().emit(&app) {
        tracing::warn!(error = %e, "no se pudo emitir settings://changed");
    }
    Ok(saved)
}
