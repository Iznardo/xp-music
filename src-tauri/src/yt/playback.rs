//! Conexión YouTube Music ↔ reproductor: radio automática, normalización de
//! volumen y registro de escuchas (FR-003-11/14/15).

use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use super::service::YtService;
use crate::domain::{Track, TrackId};
use crate::error::AppResult;
use crate::player::service::{BoxFuture, PlaybackReporter, PlayerService, RadioProvider};
use crate::settings::Settings;
use crate::storage::Db;

/// Segundos de escucha antes de registrarla en el historial.
const REPORT_AFTER: Duration = Duration::from_secs(10);
/// Pistas que se añaden a la cola cuando se acaba (autoplay).
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

    /// El reproductor se crea después del reportero: se conecta aquí.
    pub fn attach(&self, player: &Arc<PlayerService>) {
        let _ = self.0.player.set(Arc::downgrade(player));
    }
}

impl PlaybackReporter for YtReporter {
    fn on_track_started(&self, id: TrackId) {
        *self.0.current.lock().unwrap() = Some(id.clone());
        let inner = Arc::clone(&self.0);
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let settings = Settings::load(&inner.db).unwrap_or_default();
            let is_current = || inner.current.lock().unwrap().as_ref() == Some(&id);
            let player = inner.player.get().and_then(Weak::upgrade);

            let info = match inner.yt.player_info(&id).await {
                Ok(i) => i,
                Err(e) => {
                    tracing::debug!(track = %id.0, error = %e, "sin datos del reproductor web");
                    if let Some(p) = &player {
                        p.set_gain_db(0.0);
                    }
                    return;
                }
            };
            // YouTube solo atenúa lo que suena por encima de su referencia (no sube lo flojo).
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
                return; // Se saltó antes de los 10 s.
            }
            match inner.yt.report_listen(url).await {
                Ok(()) => tracing::info!(track = %id.0, "escucha registrada en el historial"),
                Err(e) => {
                    tracing::warn!(track = %id.0, error = %e, "no se pudo registrar la escucha")
                }
            }
        });
    }
}
