//! Comandos Tauri expuestos al frontend. Convención: `area_accion`.

use std::sync::Arc;

use tauri::State;
use tauri_specta::Event;

use crate::audio::output::list_devices;
use crate::domain::Track;
use crate::error::AppResult;
use crate::player::service::PlayerService;
use crate::player::types::{
    AudioDevice, InsertPosition, PlayerState, QueueSnapshot, QueueSource, RepeatMode,
};
use crate::settings::Settings;
use crate::state::AppState;

type Player<'a> = State<'a, Arc<PlayerService>>;

// ---------------------------------------------------------------- ajustes

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

// ---------------------------------------------------------------- reproductor

#[tauri::command]
#[specta::specta]
pub fn player_get_state(player: Player<'_>) -> PlayerState {
    player.state()
}

#[tauri::command]
#[specta::specta]
pub fn player_play(player: Player<'_>) {
    player.play();
}

#[tauri::command]
#[specta::specta]
pub fn player_pause(player: Player<'_>) {
    player.pause();
}

#[tauri::command]
#[specta::specta]
pub fn player_toggle(player: Player<'_>) {
    player.toggle();
}

#[tauri::command]
#[specta::specta]
pub fn player_stop(player: Player<'_>) {
    player.stop();
}

#[tauri::command]
#[specta::specta]
pub fn player_next(player: Player<'_>) {
    player.next();
}

#[tauri::command]
#[specta::specta]
pub fn player_previous(player: Player<'_>) {
    player.previous();
}

#[tauri::command]
#[specta::specta]
pub fn player_seek(player: Player<'_>, position_ms: u32) {
    player.seek(position_ms);
}

/// `volume` entre 0 y 1.
#[tauri::command]
#[specta::specta]
pub fn player_set_volume(player: Player<'_>, volume: f32) {
    player.set_volume(volume);
}

#[tauri::command]
#[specta::specta]
pub fn player_set_muted(player: Player<'_>, muted: bool) {
    player.set_muted(muted);
}

#[tauri::command]
#[specta::specta]
pub fn player_set_shuffle(player: Player<'_>, enabled: bool) {
    player.set_shuffle(enabled);
}

#[tauri::command]
#[specta::specta]
pub fn player_set_repeat(player: Player<'_>, mode: RepeatMode) {
    player.set_repeat(mode);
}

#[tauri::command]
#[specta::specta]
pub fn player_list_devices() -> Vec<AudioDevice> {
    list_devices()
}

/// `None` = dispositivo predeterminado.
#[tauri::command]
#[specta::specta]
pub fn player_set_device(player: Player<'_>, id: Option<String>) -> AppResult<()> {
    player.set_device(id)
}

// ---------------------------------------------------------------- cola

#[tauri::command]
#[specta::specta]
pub fn queue_get(player: Player<'_>) -> QueueSnapshot {
    player.queue_snapshot()
}

/// Sustituye la cola y empieza a sonar `start_index` (índice en `tracks`).
#[tauri::command]
#[specta::specta]
pub fn queue_set(
    player: Player<'_>,
    tracks: Vec<Track>,
    start_index: u32,
    source: QueueSource,
) -> AppResult<()> {
    player.queue_set(tracks, start_index, source)
}

#[tauri::command]
#[specta::specta]
pub fn queue_play_index(player: Player<'_>, index: u32) -> AppResult<()> {
    player.queue_play_index(index)
}

#[tauri::command]
#[specta::specta]
pub fn queue_add(player: Player<'_>, tracks: Vec<Track>, position: InsertPosition) {
    player.queue_add(tracks, position);
}

#[tauri::command]
#[specta::specta]
pub fn queue_remove(player: Player<'_>, index: u32) {
    player.queue_remove(index);
}

#[tauri::command]
#[specta::specta]
pub fn queue_move(player: Player<'_>, from: u32, to: u32) {
    player.queue_move(from, to);
}

#[tauri::command]
#[specta::specta]
pub fn queue_clear(player: Player<'_>) {
    player.queue_clear();
}

#[tauri::command]
#[specta::specta]
pub fn queue_set_autoplay(player: Player<'_>, enabled: bool) {
    player.queue_set_autoplay(enabled);
}
