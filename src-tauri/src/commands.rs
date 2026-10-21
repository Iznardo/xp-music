//! Comandos Tauri expuestos al frontend. Convención: `area_accion`.

use std::sync::Arc;

use tauri::State;
use tauri_specta::Event;

use crate::audio::output::list_devices;
use crate::domain::{
    AlbumDetail, AlbumSummary, ArtistDetail, ArtistSummary, BrowseId, LikeStatus, MediaItem, Page,
    PlaylistDetail, PlaylistId, PlaylistSummary, Privacy, SearchFilter, Section, Track, TrackId,
};
use crate::error::AppResult;
use crate::player::service::PlayerService;
use crate::player::types::{
    AudioDevice, InsertPosition, PlayerState, QueueSnapshot, QueueSource, RepeatMode,
};
use crate::settings::Settings;
use crate::state::AppState;
use crate::yt::service::{AuthStatus, SessionAccount, YtService};
use crate::yt::session::BrowserProfile;

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
    crate::os::apply_settings(&app, &saved);
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

/// La UI avisa de las pistas que probablemente se van a pedir (ratón encima,
/// selección) para resolverlas antes del doble clic.
#[tauri::command]
#[specta::specta]
pub fn player_hint(player: Player<'_>, ids: Vec<TrackId>) {
    player.hint(ids);
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

// ---------------------------------------------------------------- sesión de YouTube Music

type YtState<'a> = State<'a, Arc<YtService>>;

#[tauri::command]
#[specta::specta]
pub fn auth_status(yt: YtState<'_>) -> AuthStatus {
    yt.status()
}

/// Abre la ventana de inicio de sesión de Google y devuelve las cuentas de esa sesión.
#[tauri::command]
#[specta::specta]
pub async fn auth_sign_in(yt: YtState<'_>) -> AppResult<Vec<SessionAccount>> {
    yt.sign_in_app().await
}

/// Perfiles de navegadores basados en Firefox (Zen, Firefox, LibreWolf…).
#[tauri::command]
#[specta::specta]
pub fn auth_detect_browsers(yt: YtState<'_>) -> Vec<BrowserProfile> {
    yt.detect_browsers()
}

/// Cuentas de YouTube con sesión en ese perfil (todas sus sesiones/contenedores).
#[tauri::command]
#[specta::specta]
pub async fn auth_scan_browser(yt: YtState<'_>, profile: String) -> AppResult<Vec<SessionAccount>> {
    yt.scan_browser(profile).await
}

/// Cuentas en una cabecera `Cookie` pegada a mano.
#[tauri::command]
#[specta::specta]
pub async fn auth_scan_cookie(yt: YtState<'_>, cookie: String) -> AppResult<Vec<SessionAccount>> {
    yt.scan_pasted(cookie).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_select_account(
    yt: YtState<'_>,
    account: SessionAccount,
) -> AppResult<AuthStatus> {
    yt.select_account(account).await
}

#[tauri::command]
#[specta::specta]
pub async fn auth_logout(yt: YtState<'_>) -> AppResult<()> {
    yt.logout().await;
    Ok(())
}

// ---------------------------------------------------------------- YouTube Music: lectura

#[tauri::command]
#[specta::specta]
pub async fn yt_home(yt: YtState<'_>, continuation: Option<String>) -> AppResult<Page<Section>> {
    yt.inner().home(continuation).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_library_playlists(yt: YtState<'_>) -> AppResult<Vec<PlaylistSummary>> {
    yt.inner().library_playlists().await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_library_albums(yt: YtState<'_>) -> AppResult<Vec<AlbumSummary>> {
    yt.inner().library_albums().await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_library_artists(yt: YtState<'_>) -> AppResult<Vec<ArtistSummary>> {
    yt.inner().library_artists().await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_library_songs(yt: YtState<'_>) -> AppResult<Page<Track>> {
    yt.inner().library_songs().await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_liked_songs(yt: YtState<'_>) -> AppResult<Page<Track>> {
    yt.inner().liked_songs().await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_history(yt: YtState<'_>) -> AppResult<Vec<Section>> {
    yt.inner().history().await
}

/// Siguiente página de canciones (playlist, me gusta, canciones de la biblioteca).
#[tauri::command]
#[specta::specta]
pub async fn yt_more_tracks(yt: YtState<'_>, continuation: String) -> AppResult<Page<Track>> {
    yt.more_tracks(continuation).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_playlist(yt: YtState<'_>, id: PlaylistId) -> AppResult<PlaylistDetail> {
    yt.inner().playlist(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_album(yt: YtState<'_>, id: BrowseId) -> AppResult<AlbumDetail> {
    yt.inner().album(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_artist(yt: YtState<'_>, id: BrowseId) -> AppResult<ArtistDetail> {
    yt.inner().artist(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_search(
    yt: YtState<'_>,
    query: String,
    filter: SearchFilter,
) -> AppResult<Page<MediaItem>> {
    yt.search(query, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_search_more(
    yt: YtState<'_>,
    continuation: String,
    filter: SearchFilter,
) -> AppResult<Page<MediaItem>> {
    yt.more_search(continuation, filter).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_search_suggestions(yt: YtState<'_>, query: String) -> AppResult<Vec<String>> {
    yt.suggestions(query).await
}

/// Canciones de la radio de `seed` (la semilla primero).
#[tauri::command]
#[specta::specta]
pub async fn yt_radio(yt: YtState<'_>, seed: TrackId) -> AppResult<Vec<Track>> {
    yt.radio(seed).await
}

// ---------------------------------------------------------------- YouTube Music: escritura

#[tauri::command]
#[specta::specta]
pub async fn yt_rate(
    yt: YtState<'_>,
    player: Player<'_>,
    id: TrackId,
    status: LikeStatus,
) -> AppResult<()> {
    yt.rate(id.clone(), status).await?;
    player.set_like(&id, status);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn yt_add_to_playlist(
    yt: YtState<'_>,
    playlist: PlaylistId,
    tracks: Vec<TrackId>,
) -> AppResult<()> {
    yt.add_to_playlist(playlist, tracks).await
}

#[tauri::command]
#[specta::specta]
pub async fn yt_create_playlist(
    yt: YtState<'_>,
    title: String,
    description: Option<String>,
    privacy: Privacy,
) -> AppResult<PlaylistId> {
    yt.create_playlist(title, description, privacy).await
}
