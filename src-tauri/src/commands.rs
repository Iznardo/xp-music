//! Tauri commands exposed to the frontend. Convention: `area_action`.

use std::sync::Arc;

use tauri::State;
use tauri_specta::Event;

use crate::audio::output::list_devices;
use crate::domain::{
    AlbumDetail, AlbumSummary, ArtistDetail, ArtistSummary, BrowseId, LikeStatus, MediaItem, Page,
    PlaylistDetail, PlaylistId, PlaylistSummary, Privacy, SearchFilter, Section, Track, TrackId,
};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::local::{LocalAlbum, LocalArtist, LocalFilter, LocalLibrary, LocalScanStatus};
use crate::player::service::PlayerService;
use crate::player::types::{
    AudioDevice, InsertPosition, PlayerState, QueueSnapshot, QueueSource, RepeatMode,
};
use crate::settings::Settings;
use crate::state::AppState;
use crate::yt::service::{AuthStatus, SessionAccount, YtService};
use crate::yt::session::BrowserProfile;

type Player<'a> = State<'a, Arc<PlayerService>>;

// ---------------------------------------------------------------- settings

#[tauri::command]
#[specta::specta]
pub fn settings_get(state: State<'_, AppState>) -> AppResult<Settings> {
    Settings::load(&state.db)
}

/// Replaces the whole settings and notifies all windows.
#[tauri::command]
#[specta::specta]
pub fn settings_set(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    local: Local<'_>,
    settings: Settings,
) -> AppResult<Settings> {
    let folders_before = Settings::load(&state.db)?.local_folders;
    let saved = settings.save(&state.db)?;
    if saved.local_folders != folders_before {
        local.rescan();
    }
    crate::os::apply_settings(&app, &saved);
    if let Err(e) = saved.clone().emit(&app) {
        tracing::warn!(error = %e, "could not emit settings://changed");
    }
    Ok(saved)
}

// ---------------------------------------------------------------- player

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

/// `volume` between 0 and 1.
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

/// `None` = default device.
#[tauri::command]
#[specta::specta]
pub fn player_set_device(player: Player<'_>, id: Option<String>) -> AppResult<()> {
    player.set_device(id)
}

// ---------------------------------------------------------------- queue

#[tauri::command]
#[specta::specta]
pub fn queue_get(player: Player<'_>) -> QueueSnapshot {
    player.queue_snapshot()
}

/// Replaces the queue and starts playing `start_index` (index into `tracks`).
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

/// The UI reports tracks that are likely to be requested (mouse hover,
/// selection) so they can be resolved before the double click.
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

// ---------------------------------------------------------------- my music (local files)

type Local<'a> = State<'a, Arc<LocalLibrary>>;

#[tauri::command]
#[specta::specta]
pub fn local_folders(local: Local<'_>) -> Vec<String> {
    local.folders()
}

/// Asks for one or more folders with the system dialog and adds them to the library.
/// Returns the resulting list (unchanged if cancelled).
#[tauri::command]
#[specta::specta]
pub async fn local_add_folder(app: tauri::AppHandle, local: Local<'_>) -> AppResult<Vec<String>> {
    use tauri_plugin_dialog::DialogExt;
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(tr!("Add a music folder", "Agregar una carpeta de música"))
            .blocking_pick_folders()
    })
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;
    let mut folders = local.folders();
    let new: Vec<String> = picked
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| f.into_path().ok())
        .map(|p| p.display().to_string())
        .filter(|p| !folders.contains(p))
        .collect();
    if new.is_empty() {
        return Ok(folders);
    }
    folders.extend(new);
    local.inner().set_folders(folders)
}

#[tauri::command]
#[specta::specta]
pub fn local_remove_folder(local: Local<'_>, path: String) -> AppResult<Vec<String>> {
    let folders = local.folders().into_iter().filter(|f| *f != path).collect();
    local.inner().set_folders(folders)
}

#[tauri::command]
#[specta::specta]
pub fn local_rescan(local: Local<'_>) {
    local.inner().rescan();
}

#[tauri::command]
#[specta::specta]
pub fn local_scan_status(local: Local<'_>) -> LocalScanStatus {
    local.status()
}

#[tauri::command]
#[specta::specta]
pub fn local_tracks(state: State<'_, AppState>, filter: LocalFilter) -> AppResult<Vec<Track>> {
    crate::local::tracks(&state.db, &filter)
}

#[tauri::command]
#[specta::specta]
pub fn local_albums(state: State<'_, AppState>) -> AppResult<Vec<LocalAlbum>> {
    crate::local::albums(&state.db)
}

#[tauri::command]
#[specta::specta]
pub fn local_artists(state: State<'_, AppState>) -> AppResult<Vec<LocalArtist>> {
    crate::local::artists(&state.db)
}

/// Likes and playlists are YouTube's: a local file cannot go there.
fn youtube_only(ids: &[TrackId]) -> AppResult<()> {
    if ids.iter().any(TrackId::is_local) {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            tr!(
                "Songs from your computer cannot be sent to YouTube Music.",
                "Las canciones de tu equipo no se pueden enviar a YouTube Music."
            ),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- YouTube Music session

type YtState<'a> = State<'a, Arc<YtService>>;

#[tauri::command]
#[specta::specta]
pub fn auth_status(yt: YtState<'_>) -> AuthStatus {
    yt.status()
}

/// Opens the Google sign-in window and returns the accounts of that session.
#[tauri::command]
#[specta::specta]
pub async fn auth_sign_in(yt: YtState<'_>) -> AppResult<Vec<SessionAccount>> {
    yt.sign_in_app().await
}

/// Firefox-based browser profiles (Zen, Firefox, LibreWolf…).
#[tauri::command]
#[specta::specta]
pub fn auth_detect_browsers(yt: YtState<'_>) -> Vec<BrowserProfile> {
    yt.detect_browsers()
}

/// YouTube accounts signed in on that profile (all its sessions/containers).
#[tauri::command]
#[specta::specta]
pub async fn auth_scan_browser(yt: YtState<'_>, profile: String) -> AppResult<Vec<SessionAccount>> {
    yt.scan_browser(profile).await
}

/// Accounts in a `Cookie` header pasted by hand.
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
    yt.logout().await
}

// ---------------------------------------------------------------- YouTube Music: read

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

/// Next page of songs (playlist, liked songs, library songs).
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

/// Songs of the radio for `seed` (the seed first).
#[tauri::command]
#[specta::specta]
pub async fn yt_radio(yt: YtState<'_>, seed: TrackId) -> AppResult<Vec<Track>> {
    youtube_only(std::slice::from_ref(&seed))?;
    yt.radio(seed).await
}

// ---------------------------------------------------------------- YouTube Music: write

#[tauri::command]
#[specta::specta]
pub async fn yt_rate(
    yt: YtState<'_>,
    player: Player<'_>,
    id: TrackId,
    status: LikeStatus,
) -> AppResult<()> {
    youtube_only(std::slice::from_ref(&id))?;
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
    youtube_only(&tracks)?;
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
