//! Orchestrates queue + resolver + engine and publishes the state (specs/002).
//!
//! The public methods are called from Tauri commands and return right away: the
//! slow parts (yt-dlp, network, radio) go to separate threads or tasks, and a
//! sequence number discards results that arrive after the user already asked for
//! something else.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender, unbounded};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_specta::Event;

use super::queue::{Previous, Queue};
use super::types::{
    InsertPosition, PlaybackStatus, PlayerError, PlayerState, PositionUpdate, QueueSnapshot,
    QueueSource, RepeatMode,
};
use crate::audio::decoder::MediaStream;
use crate::audio::engine::{EngineCmd, EngineEvent, EngineHandle, EngineState};
use crate::audio::resolver::{FormatPreference, ResolvedStream, StreamResolver, Upgrade};
use crate::domain::{LikeStatus, Track, TrackId};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::settings::Settings;
use crate::storage::Db;

/// The next track is preloaded when fewer than this many ms remain.
const PRELOAD_BEFORE_END_MS: u32 = 20_000;
/// Retries per track when the download fails midway.
const MAX_STREAM_RETRIES: u32 = 2;
const SAVE_DEBOUNCE: Duration = Duration::from_secs(2);
/// How often the position is flagged for saving while playing.
const POSITION_SAVE_EVERY_MS: u32 = 10_000;
/// Position saved separately from the queue (`settings` table).
const POSITION_KEY: &str = "player.position_ms";
/// Upcoming tracks that are resolved ahead of time (yt-dlp takes seconds).
const PREFETCH_AHEAD: usize = 3;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type StateObserver = Box<dyn Fn(&PlayerState) + Send + Sync>;

/// More songs when the queue runs out (implemented in `yt::playback`).
pub trait RadioProvider: Send + Sync {
    fn continue_radio(
        &self,
        seed: TrackId,
        exclude: Vec<TrackId>,
    ) -> BoxFuture<AppResult<Vec<Track>>>;
}

/// Recording of plays in the history (implemented in `yt::playback`).
pub trait PlaybackReporter: Send + Sync {
    fn on_track_started(&self, id: TrackId);
}

#[derive(Serialize, Deserialize)]
struct Persisted {
    queue: Queue,
    position_ms: u32,
}

struct Inner {
    queue: Queue,
    status: PlaybackStatus,
    position_ms: u32,
    duration_ms: Option<u32>,
    volume: f32,
    muted: bool,
    error: Option<AppError>,
    /// Track loaded (or loading) in the engine.
    loaded: Option<TrackId>,
    load_seq: u64,
    preload_for: Option<TrackId>,
    retries: u32,
    last_saved_position: u32,
}

pub struct PlayerService {
    app: AppHandle,
    engine: EngineHandle,
    resolver: Arc<dyn StreamResolver>,
    db: Arc<Db>,
    radio: Arc<dyn RadioProvider>,
    reporter: Arc<dyn PlaybackReporter>,
    inner: Mutex<Inner>,
    dirty: AtomicBool,
    /// The queue changed since the last save. If only the position changed, it is
    /// saved on its own (the whole queue used to be rewritten: MBs every 10 s).
    queue_dirty: AtomicBool,
    /// Anyone who wants to learn about every state change without depending on the
    /// WebView (tray, Windows media controls, notifications).
    observers: Mutex<Vec<StateObserver>>,
    prefetch_tx: Sender<Vec<TrackId>>,
    /// Tracks the user is likely to request (mouse hover, selected row).
    /// Separate lane so it doesn't wait on the queue prefetch.
    hint_tx: Sender<Vec<TrackId>>,
    /// Fast start (008): URL of the start of the audio in ~0.3 s.
    fast_start: Mutex<Option<FastStart>>,
}

pub type FastStart = Arc<dyn Fn(&TrackId) -> AppResult<ResolvedStream> + Send + Sync>;

impl PlayerService {
    pub fn start(
        app: AppHandle,
        db: Arc<Db>,
        resolver: Arc<dyn StreamResolver>,
        radio: Arc<dyn RadioProvider>,
        reporter: Arc<dyn PlaybackReporter>,
    ) -> Arc<Self> {
        let settings = Settings::load(&db).unwrap_or_default();
        let (queue, position_ms) = restore(&db);
        let status = if queue.current().is_some() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Idle
        };
        let duration_ms = queue.current().and_then(|t| t.duration_ms);

        let (ev_tx, ev_rx) = unbounded();
        let (prefetch_tx, prefetch_rx) = unbounded();
        let (hint_tx, hint_rx) = unbounded();
        let engine = EngineHandle::spawn(ev_tx);
        engine.send(EngineCmd::SetVolume {
            volume: settings.volume,
            muted: false,
        });
        if let Some(dev) = settings.output_device.clone() {
            engine.send(EngineCmd::SetDevice(Some(dev)));
        }

        let svc = Arc::new(Self {
            app,
            engine,
            resolver,
            db,
            radio,
            reporter,
            inner: Mutex::new(Inner {
                queue,
                status,
                position_ms,
                duration_ms,
                volume: settings.volume,
                muted: false,
                error: None,
                loaded: None,
                load_seq: 0,
                preload_for: None,
                retries: 0,
                last_saved_position: position_ms,
            }),
            dirty: AtomicBool::new(false),
            queue_dirty: AtomicBool::new(false),
            observers: Mutex::new(Vec::new()),
            prefetch_tx,
            hint_tx,
            fast_start: Mutex::new(None),
        });

        spawn_prefetcher("player-prefetch", Arc::clone(&svc.resolver), prefetch_rx);
        spawn_prefetcher("player-hint", Arc::clone(&svc.resolver), hint_rx);
        let pump = Arc::clone(&svc);
        thread::Builder::new()
            .name("player-events".into())
            .spawn(move || pump.event_loop(ev_rx))
            .expect("player events thread");
        let saver = Arc::downgrade(&svc);
        thread::Builder::new()
            .name("player-saver".into())
            .spawn(move || {
                loop {
                    thread::sleep(SAVE_DEBOUNCE);
                    let Some(s) = saver.upgrade() else { break };
                    if s.dirty.swap(false, Ordering::AcqRel) {
                        s.save_now();
                    }
                }
            })
            .expect("queue saver thread");
        svc
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|p| p.into_inner())
    }

    // ------------------------------------------------------------ state

    pub fn state(&self) -> PlayerState {
        let i = self.lock();
        PlayerState {
            status: i.status,
            track: i.queue.current().cloned(),
            position_ms: i.position_ms,
            duration_ms: i.duration_ms,
            volume: i.volume,
            muted: i.muted,
            shuffle: i.queue.shuffle(),
            repeat: i.queue.repeat(),
            error: i.error.clone(),
        }
    }

    pub fn queue_snapshot(&self) -> QueueSnapshot {
        self.lock().queue.snapshot()
    }

    fn emit_state(&self) {
        let state = self.state();
        for f in self
            .observers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
        {
            f(&state);
        }
        if let Err(e) = state.emit(&self.app) {
            tracing::warn!(error = %e, "could not emit player://state");
        }
    }

    fn emit_queue(&self) {
        let _ = self.queue_snapshot().emit(&self.app);
        self.queue_dirty.store(true, Ordering::Release);
        self.dirty.store(true, Ordering::Release);
    }

    fn fail(&self, err: AppError) {
        {
            let mut i = self.lock();
            i.status = PlaybackStatus::Error;
            i.error = Some(err.clone());
        }
        self.emit_state();
        let _ = PlayerError(err).emit(&self.app);
    }

    // ------------------------------------------------------------ loading

    /// Resolves the queue's current track and loads it in the engine.
    fn play_current(self: &Arc<Self>, start_ms: u32, autoplay: bool) {
        let (track, seq) = {
            let mut i = self.lock();
            let Some(track) = i.queue.current().cloned() else {
                drop(i);
                self.stop();
                return;
            };
            i.load_seq += 1;
            i.loaded = Some(track.id.clone());
            i.status = PlaybackStatus::Loading;
            i.position_ms = start_ms;
            i.duration_ms = track.duration_ms;
            i.error = None;
            i.preload_for = None;
            (track, i.load_seq)
        };
        // Whatever was playing (or opening) is cut right away: otherwise, with quick ⏭
        // the intermediate song would keep playing while the new one resolves.
        self.engine.send(EngineCmd::Stop);
        self.emit_state();
        // Without the current track in cache, the next ones wait for its yt-dlp to
        // finish (`resolve_for_play`): several at once when cold slow everything down.
        if track.id.is_local()
            || self
                .resolver
                .cached(&track.id, FormatPreference::Aac128)
                .is_some()
        {
            self.prefetch_upcoming();
        }

        let svc = Arc::clone(self);
        thread::spawn(move || {
            let result = svc.resolve_for_play(&track);
            {
                let mut i = svc.lock();
                if i.load_seq != seq {
                    return; // The user already asked for another song.
                }
                if let Ok(s) = &result {
                    i.duration_ms = i.duration_ms.or(s.duration_ms());
                }
            }
            match result {
                Ok(stream) => svc.engine.send(EngineCmd::Load {
                    stream,
                    start_ms,
                    autoplay,
                }),
                Err(e) => svc.fail(e),
            }
        });
    }

    pub fn set_fast_start(&self, f: FastStart) {
        *self.fast_start.lock().unwrap() = Some(f);
    }

    /// What gets loaded into the engine: a local file as is; a YouTube one from the
    /// cache, with fast start (and the full resolution in parallel) or, failing
    /// that, waiting for yt-dlp.
    fn resolve_for_play(self: &Arc<Self>, track: &Track) -> AppResult<MediaStream> {
        if track.id.is_local() {
            return local_stream(track);
        }
        self.resolve_remote(&track.id).map(MediaStream::Remote)
    }

    fn resolve_remote(self: &Arc<Self>, id: &TrackId) -> AppResult<ResolvedStream> {
        let pref = FormatPreference::Aac128;
        if let Some(s) = self.resolver.cached(id, pref) {
            return Ok(s);
        }
        let Some(fast) = self.fast_start.lock().unwrap().clone() else {
            let r = self.resolver.resolve(id, pref);
            self.prefetch_upcoming();
            return r;
        };
        let upgrade = Upgrade::default();
        {
            let (svc, id, up) = (Arc::clone(self), id.clone(), upgrade.clone());
            thread::spawn(move || {
                up.fill(svc.resolver.resolve(&id, pref));
                svc.prefetch_upcoming();
            });
        }
        let t0 = std::time::Instant::now();
        match fast(id) {
            Ok(mut head) => {
                tracing::info!(track = %id.0, ms = t0.elapsed().as_millis() as u64, "fast start");
                head.upgrade = Some(upgrade);
                Ok(head)
            }
            // No fast start: the usual way (the resolution is already underway).
            Err(_) => loop {
                if let Some(r) = upgrade.wait(Duration::from_secs(1)) {
                    break r;
                }
            },
        }
    }

    /// Resolves right now the tracks the user is about to pick.
    pub fn hint(&self, ids: Vec<TrackId>) {
        let ids: Vec<TrackId> = ids.into_iter().filter(|id| !id.is_local()).collect();
        if !ids.is_empty() {
            let _ = self.hint_tx.send(ids);
        }
    }

    /// Asks to resolve the upcoming tracks in the background so that ⏭ and
    /// transitions don't wait on yt-dlp.
    fn prefetch_upcoming(&self) {
        let ids: Vec<TrackId> = {
            let i = self.lock();
            let (Some(p), len) = (i.queue.current_index(), i.queue.len()) else {
                return;
            };
            let wrap = i.queue.repeat() == RepeatMode::All;
            (1..=PREFETCH_AHEAD.min(len.saturating_sub(1)))
                .map(|k| p + k)
                .filter_map(|n| {
                    if n < len {
                        Some(n)
                    } else if wrap {
                        Some(n % len)
                    } else {
                        None
                    }
                })
                .filter_map(|n| i.queue.track_at(n).map(|t| t.id.clone()))
                // Local files need no resolution.
                .filter(|id| !id.is_local())
                .collect()
        };
        if !ids.is_empty() {
            let _ = self.prefetch_tx.send(ids);
        }
    }

    fn preload_next(self: &Arc<Self>) {
        let next = {
            let mut i = self.lock();
            let Some(next) = i
                .queue
                .peek_next(true)
                .and_then(|n| i.queue.track_at(n))
                .cloned()
            else {
                return;
            };
            if i.preload_for.as_ref() == Some(&next.id) {
                return;
            }
            i.preload_for = Some(next.id.clone());
            next
        };
        let svc = Arc::clone(self);
        thread::spawn(move || {
            let stream = if next.id.is_local() {
                local_stream(&next)
            } else {
                svc.resolver
                    .resolve(&next.id, FormatPreference::Aac128)
                    .map(MediaStream::Remote)
            };
            match stream {
                Ok(stream) => svc.engine.send(EngineCmd::Preload { stream }),
                Err(e) => tracing::warn!(track = %next.id.0, error = %e, "could not preload"),
            }
        });
    }

    // ------------------------------------------------------------ transport

    pub fn play(self: &Arc<Self>) {
        let (loaded, has_track, position) = {
            let i = self.lock();
            (
                i.loaded.is_some(),
                i.queue.current().is_some(),
                i.position_ms,
            )
        };
        if loaded {
            self.engine.send(EngineCmd::Play);
        } else if has_track {
            // Queue restored at startup: load it at the saved position.
            self.play_current(position, true);
        }
    }

    pub fn pause(&self) {
        self.engine.send(EngineCmd::Pause);
    }

    pub fn toggle(self: &Arc<Self>) {
        let status = self.lock().status;
        match status {
            PlaybackStatus::Playing | PlaybackStatus::Buffering | PlaybackStatus::Loading => {
                self.pause()
            }
            _ => self.play(),
        }
    }

    pub fn stop(&self) {
        self.engine.send(EngineCmd::Stop);
        {
            let mut i = self.lock();
            i.load_seq += 1;
            i.loaded = None;
            i.status = PlaybackStatus::Idle;
            i.position_ms = 0;
            i.preload_for = None;
        }
        self.emit_state();
        self.dirty.store(true, Ordering::Release);
    }

    pub fn next(self: &Arc<Self>) {
        let advanced = self.lock().queue.advance(false);
        if advanced {
            self.emit_queue();
            self.play_current(0, true);
        } else {
            self.on_queue_end();
        }
    }

    pub fn previous(self: &Arc<Self>) {
        let (result, position) = {
            let mut i = self.lock();
            let pos = i.position_ms;
            (i.queue.previous(pos), pos)
        };
        match result {
            Previous::Restart if position > 0 => self.seek(0),
            Previous::Restart => {}
            Previous::Moved => {
                self.emit_queue();
                self.play_current(0, true);
            }
        }
    }

    pub fn seek(&self, ms: u32) {
        let loaded = {
            let mut i = self.lock();
            let ms = i.duration_ms.map_or(ms, |d| ms.min(d.saturating_sub(500)));
            i.position_ms = ms;
            i.loaded.is_some()
        };
        if loaded {
            self.engine.send(EngineCmd::Seek(self.lock().position_ms));
        }
        self.emit_state();
    }

    pub fn set_volume(&self, volume: f32) {
        let volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.8
        };
        let muted = {
            let mut i = self.lock();
            i.volume = volume;
            i.muted
        };
        self.engine.send(EngineCmd::SetVolume { volume, muted });
        if let Ok(s) = Settings::load(&self.db) {
            let saved = Settings { volume, ..s }.save(&self.db);
            if let Ok(saved) = saved {
                let _ = saved.emit(&self.app);
            }
        }
        self.emit_state();
    }

    pub fn set_muted(&self, muted: bool) {
        let volume = {
            let mut i = self.lock();
            i.muted = muted;
            i.volume
        };
        self.engine.send(EngineCmd::SetVolume { volume, muted });
        self.emit_state();
    }

    pub fn set_shuffle(&self, enabled: bool) {
        {
            let mut i = self.lock();
            i.queue.set_shuffle(enabled);
            i.preload_for = None;
        }
        self.emit_queue();
        self.prefetch_upcoming();
        self.emit_state();
    }

    pub fn set_repeat(&self, mode: RepeatMode) {
        {
            let mut i = self.lock();
            i.queue.set_repeat(mode);
            i.preload_for = None;
        }
        self.emit_queue();
        self.prefetch_upcoming();
        self.emit_state();
    }

    /// Registers a state observer (006) and passes it the current state.
    pub fn observe(&self, f: StateObserver) {
        f(&self.state());
        self.observers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(f);
    }

    /// Turns the PCM copy for the analyzer on or off (005).
    pub fn set_tap(&self, on: bool) {
        self.engine.send(EngineCmd::SetTap(on));
    }

    /// New PCM tap (if the output was opened or changed since the last time).
    pub fn take_tap(&self) -> Option<crate::audio::output::PcmTap> {
        self.engine.take_tap()
    }

    /// UI sound (004), mixed over the output.
    pub fn play_sound(&self, s: crate::audio::sounds::UiSound) {
        self.engine.send(EngineCmd::PlaySound(s));
    }

    /// Normalization gain of the current track (003, `loudnessDb`).
    pub fn set_gain_db(&self, db: f32) {
        self.engine.send(EngineCmd::SetGainDb(db));
    }

    pub fn set_device(&self, id: Option<String>) -> AppResult<()> {
        self.engine.send(EngineCmd::SetDevice(id.clone()));
        let s = Settings::load(&self.db)?;
        let saved = Settings {
            output_device: id,
            ..s
        }
        .save(&self.db)?;
        let _ = saved.emit(&self.app);
        Ok(())
    }

    // ------------------------------------------------------------ queue

    pub fn queue_set(
        self: &Arc<Self>,
        tracks: Vec<Track>,
        start: u32,
        source: QueueSource,
    ) -> AppResult<()> {
        if tracks.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                tr!("The queue is empty.", "La lista está vacía."),
            ));
        }
        self.lock().queue.set(tracks, start as usize, source);
        self.emit_queue();
        self.play_current(0, true);
        Ok(())
    }

    pub fn queue_play_index(self: &Arc<Self>, index: u32) -> AppResult<()> {
        if !self.lock().queue.play_index(index as usize) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                tr!(
                    "That song is not in the queue.",
                    "Esa canción no está en la cola."
                ),
            ));
        }
        self.emit_queue();
        self.play_current(0, true);
        Ok(())
    }

    pub fn queue_add(&self, tracks: Vec<Track>, at: InsertPosition) {
        {
            let mut i = self.lock();
            i.queue.add(tracks, at);
            i.preload_for = None;
        }
        self.emit_queue();
        self.prefetch_upcoming();
        self.emit_state();
    }

    pub fn queue_remove(self: &Arc<Self>, index: u32) {
        let (was_current, playing) = {
            let mut i = self.lock();
            let was = i.queue.remove(index as usize);
            i.preload_for = None;
            (
                was,
                matches!(
                    i.status,
                    PlaybackStatus::Playing | PlaybackStatus::Buffering | PlaybackStatus::Loading
                ),
            )
        };
        self.emit_queue();
        if was_current {
            if self.lock().queue.current().is_some() {
                self.play_current(0, playing);
            } else {
                self.stop();
            }
        }
    }

    pub fn queue_move(&self, from: u32, to: u32) {
        {
            let mut i = self.lock();
            i.queue.move_item(from as usize, to as usize);
            i.preload_for = None;
        }
        self.emit_queue();
        self.prefetch_upcoming();
    }

    pub fn queue_clear(&self) {
        self.lock().queue.clear();
        self.stop();
        self.emit_queue();
    }

    /// "Like" given or removed: the song in the queue (and the playing one) reflects it.
    pub fn set_like(&self, id: &TrackId, status: LikeStatus) {
        if self.lock().queue.set_like(id, status) {
            self.emit_queue();
            self.emit_state();
        }
    }

    pub fn queue_set_autoplay(&self, enabled: bool) {
        self.lock().queue.set_autoplay(enabled);
        self.emit_queue();
    }

    /// End of the queue: autoplay with radio or stop.
    fn on_queue_end(self: &Arc<Self>) {
        let (autoplay, seed, exclude) = {
            let i = self.lock();
            let seed = radio_seed(&i.queue);
            let exclude: Vec<TrackId> = i
                .queue
                .tracks_in_order()
                .map(|t| t.id.clone())
                .filter(|id| !id.is_local())
                .collect();
            (i.queue.autoplay(), seed, exclude)
        };
        let Some(seed) = seed.filter(|_| autoplay) else {
            self.stop();
            return;
        };
        let svc = Arc::clone(self);
        let radio = Arc::clone(&self.radio);
        tauri::async_runtime::spawn(async move {
            match radio.continue_radio(seed, exclude).await {
                Ok(more) if !more.is_empty() => {
                    {
                        let mut i = svc.lock();
                        i.queue.add(more, InsertPosition::End);
                        i.queue.advance(false);
                    }
                    svc.emit_queue();
                    svc.play_current(0, true);
                }
                Ok(_) => svc.stop(),
                Err(e) => {
                    tracing::warn!(error = %e, "autoplay returned no results");
                    svc.stop();
                }
            }
        });
    }

    // ------------------------------------------------------------ engine events

    fn event_loop(self: Arc<Self>, rx: Receiver<EngineEvent>) {
        for ev in rx {
            self.on_engine_event(ev);
        }
    }

    fn on_engine_event(self: &Arc<Self>, ev: EngineEvent) {
        match ev {
            EngineEvent::State(s) => {
                let status = match s {
                    EngineState::Loading => PlaybackStatus::Loading,
                    EngineState::Buffering => PlaybackStatus::Buffering,
                    EngineState::Playing => PlaybackStatus::Playing,
                    EngineState::Paused => PlaybackStatus::Paused,
                    // End of track arrives as `Ended`; stopping is handled by `stop`.
                    EngineState::Idle => return,
                };
                {
                    let mut i = self.lock();
                    if i.loaded.is_none() {
                        return;
                    }
                    i.status = status;
                }
                self.emit_state();
            }
            EngineEvent::TrackStarted {
                track_id,
                duration_ms,
                gapless,
            } => {
                {
                    let mut i = self.lock();
                    if gapless {
                        // Only valid if it is exactly the next one in the queue; otherwise it is
                        // a preload that became stale after a user change.
                        let expected = i
                            .queue
                            .peek_next(true)
                            .and_then(|n| i.queue.track_at(n))
                            .map(|t| t.id.clone());
                        if expected.as_ref() != Some(&track_id) {
                            tracing::warn!(track = %track_id.0, "stale gapless transition; ignoring");
                            return;
                        }
                        i.queue.advance(true);
                        i.loaded = Some(track_id.clone());
                        i.position_ms = 0;
                        i.preload_for = None;
                    } else if i.loaded.as_ref() != Some(&track_id) {
                        return; // Arrived late: another track was already requested.
                    }
                    i.duration_ms = duration_ms.or(i.duration_ms);
                    i.retries = 0;
                }
                if gapless {
                    self.emit_queue();
                    self.prefetch_upcoming();
                }
                self.emit_state();
                self.reporter.on_track_started(track_id);
            }
            EngineEvent::Position {
                track_id,
                position_ms,
                buffered_ms,
            } => {
                let (should_preload, save) = {
                    let mut i = self.lock();
                    if i.loaded.as_ref() != Some(&track_id) {
                        return;
                    }
                    i.position_ms = position_ms;
                    let near_end = i
                        .duration_ms
                        .is_some_and(|d| d.saturating_sub(position_ms) < PRELOAD_BEFORE_END_MS);
                    let save =
                        position_ms.abs_diff(i.last_saved_position) >= POSITION_SAVE_EVERY_MS;
                    if save {
                        i.last_saved_position = position_ms;
                    }
                    (near_end, save)
                };
                let _ = PositionUpdate {
                    position_ms,
                    buffered_ms,
                }
                .emit(&self.app);
                if save {
                    self.dirty.store(true, Ordering::Release);
                }
                if should_preload {
                    self.preload_next();
                }
            }
            EngineEvent::Ended { track_id } => {
                let advanced = {
                    let mut i = self.lock();
                    if i.loaded.as_ref() != Some(&track_id) {
                        return;
                    }
                    i.loaded = None;
                    i.queue.advance(true)
                };
                if advanced {
                    self.emit_queue();
                    self.play_current(0, true);
                } else {
                    self.on_queue_end();
                }
            }
            EngineEvent::StreamFailed {
                track_id,
                position_ms,
                expired,
            } => {
                let retry = {
                    let mut i = self.lock();
                    if i.loaded.as_ref() != Some(&track_id) {
                        return;
                    }
                    i.retries += 1;
                    i.retries <= MAX_STREAM_RETRIES
                };
                tracing::warn!(track = %track_id.0, position_ms, expired, retry, "stream failure");
                if retry {
                    if expired && !track_id.is_local() {
                        self.resolver.invalidate(&track_id);
                    }
                    self.play_current(position_ms, true);
                } else {
                    self.fail(AppError::new(
                        ErrorCode::Network,
                        tr!(
                            "Could not keep playing this song.",
                            "No se pudo seguir reproduciendo esta canción."
                        ),
                    ));
                }
            }
            EngineEvent::PreloadFailed { track_id } => {
                tracing::warn!(track = %track_id.0, "preload failed; will load on arrival");
            }
            EngineEvent::Error(e) => self.fail(e),
        }
    }

    // ------------------------------------------------------------ persistence

    pub fn save_now(&self) {
        let (position, queue_json) = {
            let i = self.lock();
            let json = self.queue_dirty.swap(false, Ordering::AcqRel).then(|| {
                serde_json::to_string(&Persisted {
                    queue: i.queue.clone(),
                    position_ms: i.position_ms,
                })
            });
            (i.position_ms, json)
        };
        // The position, on its own and small: it is what changes every few seconds.
        if let Err(e) = self.db.kv_set(POSITION_KEY, &position.to_string()) {
            tracing::warn!(error = %e, "could not save the position");
        }
        let Some(Ok(json)) = queue_json else { return };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let res = self.db.conn().execute(
            "INSERT INTO queue_state (id, json, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
            (json, now),
        );
        if let Err(e) = res {
            tracing::warn!(error = %e, "could not save the queue");
        }
    }

    pub fn shutdown(&self) {
        self.save_now();
        self.engine.shutdown();
    }
}

/// A local track goes to the engine as is; it only has to still be on disk.
fn local_stream(track: &Track) -> AppResult<MediaStream> {
    let path = track
        .id
        .local_path()
        .filter(|p| p.is_file())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                tr!(
                    "This file is no longer on disk.",
                    "Este archivo ya no está en el disco."
                ),
            )
        })?;
    Ok(MediaStream::Local {
        track_id: track.id.clone(),
        path,
        duration_ms: track.duration_ms,
    })
}

/// Song the autoplay radio starts from: the current one or, if it is a local file
/// (YouTube does not know it), the last YouTube song before it. A queue of only
/// local files has none: playback stops at the end.
fn radio_seed(queue: &Queue) -> Option<TrackId> {
    let upto = queue.current_index()?;
    queue
        .tracks_in_order()
        .take(upto + 1)
        .filter(|t| !t.id.is_local())
        .last()
        .map(|t| t.id.clone())
}

/// Thread that resolves (and caches) the tracks it receives, one at a
/// time. If a new list arrives, it abandons the previous one.
fn spawn_prefetcher(name: &str, resolver: Arc<dyn StreamResolver>, rx: Receiver<Vec<TrackId>>) {
    thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            while let Ok(mut ids) = rx.recv() {
                'list: loop {
                    for id in &ids {
                        if let Ok(newer) = rx.try_recv() {
                            ids = newer;
                            continue 'list;
                        }
                        if let Err(e) = resolver.resolve(id, FormatPreference::Aac128) {
                            tracing::debug!(track = %id.0, error = %e, "could not resolve ahead of time");
                        }
                    }
                    break;
                }
            }
        })
        .expect("prefetch thread");
}

fn restore(db: &Db) -> (Queue, u32) {
    let json: Option<String> = db
        .conn()
        .query_row("SELECT json FROM queue_state WHERE id = 1", [], |r| {
            r.get(0)
        })
        .ok();
    match json.map(|j| serde_json::from_str::<Persisted>(&j)) {
        Some(Ok(p)) => {
            // The most recent position is saved separately.
            let position_ms = db
                .kv_get(POSITION_KEY)
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(p.position_ms);
            tracing::info!(tracks = p.queue.len(), position_ms, "queue restored");
            (p.queue, position_ms)
        }
        Some(Err(e)) => {
            tracing::warn!(error = %e, "saved queue is invalid; starting empty");
            (Queue::new(), 0)
        }
        None => (Queue::new(), 0),
    }
}

#[cfg(test)]
mod bench {
    //! Cost measurements (not correctness tests):
    //! `cargo test --lib bench_ -- --ignored --nocapture`.
    use super::*;
    use crate::domain::{AlbumRef, ArtistRef, BrowseId, Thumbnail};
    use std::time::Instant;

    pub(super) fn realistic_track(i: usize) -> Track {
        let thumb = |w: u32| Thumbnail {
            url: format!(
                "https://lh3.googleusercontent.com/{}AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcdefghijklmnopqrstuvwxyz=w{w}-h{w}-l90-rj",
                i
            ),
            width: w,
            height: w,
        };
        Track {
            id: TrackId(format!("vid{i:08}")),
            title: format!("Canción número {i} (feat. Alguien)"),
            artists: vec![ArtistRef {
                id: Some(BrowseId(format!("UCartist{i}"))),
                name: "Artista de prueba".into(),
            }],
            album: Some(AlbumRef {
                id: Some(BrowseId(format!("MPREalbum{i}"))),
                name: "Álbum de prueba".into(),
            }),
            duration_ms: Some(200_000),
            thumbnails: vec![thumb(60), thumb(120), thumb(226), thumb(544)],
            explicit: false,
            like_status: None,
            set_video_id: Some(format!("SV{i}")),
        }
    }

    pub(super) fn time<F: FnMut()>(n: u32, mut f: F) -> f64 {
        let t = Instant::now();
        for _ in 0..n {
            f();
        }
        t.elapsed().as_secs_f64() * 1000.0 / n as f64
    }

    #[test]
    fn separately_saved_position_wins_on_restore() {
        let db = Db::open_in_memory().unwrap();
        let mut q = Queue::new();
        q.set(
            (0..3).map(realistic_track).collect(),
            1,
            QueueSource::Manual,
        );
        let json = serde_json::to_string(&Persisted {
            queue: q,
            position_ms: 100,
        })
        .unwrap();
        db.conn()
            .execute(
                "INSERT INTO queue_state (id, json, updated_at) VALUES (1, ?1, 0)",
                [json],
            )
            .unwrap();
        assert_eq!(restore(&db).1, 100);
        db.kv_set(POSITION_KEY, "54321").unwrap();
        let (queue, pos) = restore(&db);
        assert_eq!((queue.len(), pos), (3, 54_321));
    }

    fn local_track(path: &std::path::Path) -> Track {
        Track {
            id: TrackId::local(path),
            duration_ms: Some(1234),
            ..realistic_track(0)
        }
    }

    #[test]
    fn radio_starts_from_the_last_youtube_song() {
        let dir = std::env::temp_dir();
        let mut q = Queue::new();
        q.set(
            vec![
                realistic_track(1),
                local_track(&dir.join("a.mp3")),
                realistic_track(2),
                local_track(&dir.join("b.mp3")),
            ],
            3,
            QueueSource::Manual,
        );
        assert_eq!(radio_seed(&q), Some(realistic_track(2).id));
        q.play_index(0);
        assert_eq!(radio_seed(&q), Some(realistic_track(1).id));

        let mut only_local = Queue::new();
        only_local.set(vec![local_track(&dir.join("a.mp3"))], 0, QueueSource::Local);
        assert_eq!(
            radio_seed(&only_local),
            None,
            "nothing for YouTube to go on"
        );
    }

    #[test]
    fn local_tracks_skip_resolution() {
        let dir = std::env::temp_dir().join(format!("xp-music-svc-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("song.flac");
        std::fs::write(&path, b"x").unwrap();
        let stream = local_stream(&local_track(&path)).unwrap();
        assert_eq!(
            stream,
            MediaStream::Local {
                track_id: TrackId::local(&path),
                path: path.clone(),
                duration_ms: Some(1234),
            }
        );
        std::fs::remove_file(&path).unwrap();
        let err = local_stream(&local_track(&path)).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[test]
    #[ignore]
    fn bench_position_and_settings_save() {
        let db = Db::open_in_memory().unwrap();
        let mut q = Queue::new();
        q.set(
            (0..3000).map(realistic_track).collect(),
            0,
            QueueSource::Manual,
        );
        let json = serde_json::to_string(&Persisted {
            queue: q.clone(),
            position_ms: 1,
        })
        .unwrap();
        let ms = time(20, || {
            let json = serde_json::to_string(&Persisted {
                queue: q.clone(),
                position_ms: 1,
            })
            .unwrap();
            db.conn()
                .execute(
                    "INSERT INTO queue_state (id, json, updated_at) VALUES (1, ?1, 0)
                     ON CONFLICT(id) DO UPDATE SET json = excluded.json",
                    [json],
                )
                .unwrap();
        });
        println!(
            "BEFORE saving position (3000-track queue): {ms:.2} ms and {} KB per save",
            json.len() / 1024
        );
        let ms = time(2000, || {
            db.kv_set(POSITION_KEY, "123456").unwrap();
        });
        println!("NOW saving position: {ms:.4} ms and 6 bytes per save");
        let ms = time(2000, || {
            let _ = Settings::load(&db).unwrap();
        });
        println!("Settings::load: {:.4} ms per read", ms);
    }
}
