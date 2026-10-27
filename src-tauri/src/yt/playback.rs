//! YouTube Music ↔ player connection: automatic radio, volume normalization
//! and listen reporting (FR-003-11/14/15).

use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use super::service::YtService;
use crate::domain::{Track, TrackId};
use crate::error::AppResult;
use crate::player::service::{BoxFuture, PlaybackReporter, PlayerService, RadioProvider};
use crate::settings::Settings;
use crate::storage::Db;

/// Seconds of listening before it is recorded in the history.
const REPORT_AFTER: Duration = Duration::from_secs(10);
/// Tracks added to the queue when it runs out (autoplay).
const RADIO_BATCH: usize = 25;

pub struct YtRadio(pub Arc<YtService>);

impl RadioProvider for YtRadio {
    fn continue_radio(
        &self,
        seed: TrackId,
        exclude: Vec<TrackId>,
    ) -> BoxFuture<AppResult<Vec<Track>>> {
        let yt = Arc::clone(&self.0);
        Box::pin(async move {
            let tracks = yt.radio(seed).await?;
            Ok(tracks
                .into_iter()
                .filter(|t| !exclude.contains(&t.id))
                .take(RADIO_BATCH)
                .collect())
        })
    }
}

struct Inner {
    yt: Arc<YtService>,
    db: Arc<Db>,
    player: OnceLock<Weak<PlayerService>>,
    current: Mutex<Option<TrackId>>,
}

#[derive(Clone)]
pub struct YtReporter(Arc<Inner>);

impl YtReporter {
    pub fn new(yt: Arc<YtService>, db: Arc<Db>) -> Self {
        Self(Arc::new(Inner {
            yt,
            db,
            player: OnceLock::new(),
            current: Mutex::new(None),
        }))
    }

    /// The player is created after the reporter: it is connected here.
    pub fn attach(&self, player: &Arc<PlayerService>) {
        let _ = self.0.player.set(Arc::downgrade(player));
    }
}

impl PlaybackReporter for YtReporter {
    fn on_track_started(&self, id: TrackId) {
        *self.0.current.lock().unwrap() = Some(id.clone());
        // A local file: YouTube has no loudness for it nor a history to record it in.
        if id.is_local() {
            if let Some(p) = self.0.player.get().and_then(Weak::upgrade) {
                p.set_gain_db(0.0);
            }
            return;
        }
        let inner = Arc::clone(&self.0);
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let settings = Settings::load(&inner.db).unwrap_or_default();
            let is_current = || inner.current.lock().unwrap().as_ref() == Some(&id);
            let player = inner.player.get().and_then(Weak::upgrade);

            let info = match inner.yt.player_info(&id).await {
                Ok(i) => i,
                Err(e) => {
                    tracing::debug!(track = %id.0, error = %e, "no web player data");
                    if let Some(p) = &player {
                        p.set_gain_db(0.0);
                    }
                    return;
                }
            };
            // YouTube only attenuates what is above its reference (it doesn't boost quiet tracks).
            if let Some(p) = &player
                && is_current()
            {
                let gain = match (settings.normalize_volume, info.loudness_db) {
                    (true, Some(l)) => -l.max(0.0),
                    _ => 0.0,
                };
                p.set_gain_db(gain);
            }

            if !settings.report_history {
                return;
            }
            let Some(url) = info.tracking_url else { return };
            if let Some(left) = REPORT_AFTER.checked_sub(started.elapsed()) {
                tokio::time::sleep(left).await;
            }
            if !is_current() {
                return; // Skipped before the 10 s mark.
            }
            match inner.yt.report_listen(url).await {
                Ok(()) => tracing::info!(track = %id.0, "listen recorded in the history"),
                Err(e) => {
                    tracing::warn!(track = %id.0, error = %e, "could not record the listen")
                }
            }
        });
    }
}
