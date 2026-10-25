//! Discord Rich Presence: "Listening to XP Music" with cover and progress bar.
//!
//! Own thread, so a closed or slow Discord never blocks the media controls. It only talks to
//! the local Discord client (IPC); nothing leaves the machine except what Discord itself shows.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use discord_rich_presence::activity::{Activity, ActivityType, Assets, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

use crate::domain::Track;
use crate::player::types::{PlaybackStatus, PlayerState};

/// Application ID of "XP Music" in the Discord Developer Portal (public, not a secret).
/// Its name is what Discord shows after "Listening to".
const CLIENT_ID: &str = "1563880117095366716";
/// Art asset uploaded to the Discord application (Rich Presence > Art Assets), used when a
/// track has no usable cover.
const FALLBACK_IMAGE: &str = "icon-1024";
/// Wait before trying to reach Discord again after a failure.
const RETRY: Duration = Duration::from_secs(15);
/// How much the computed start may drift before the bar is sent again.
const DRIFT_MS: i64 = 2000;

/// Starts the thread. `enabled` is queried on every update, so the setting applies live.
pub fn start(enabled: impl Fn() -> bool + Send + 'static) -> Sender<Box<PlayerState>> {
    let (tx, rx) = channel();
    thread::Builder::new()
        .name("os-discord".into())
        .spawn(move || run(rx, enabled))
        .expect("discord presence thread");
    tx
}

/// Discord only accepts texts of 2 to 128 characters.
fn fit(s: &str) -> String {
    let mut s: String = s.chars().take(128).collect();
    while s.chars().count() < 2 {
        s.push('\u{200b}');
    }
    s
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// What Discord shows for a track. `start_ms` is when it would have started at position 0.
fn build(t: &Track, start_ms: i64, duration_ms: Option<u32>) -> Activity<'_> {
    let artist = t
        .artists
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    // The largest cover, straight from YouTube (Discord proxies external URLs up to 256 chars).
    let cover = t
        .thumbnails
        .iter()
        .max_by_key(|x| x.width)
        .map(|x| x.url.as_str())
        .filter(|u| u.len() <= 256)
        .unwrap_or(FALLBACK_IMAGE);

    // Discord's IPC takes Unix seconds (it also tolerates ms, but not every bridge does).
    let mut times = Timestamps::new().start(start_ms / 1000);
    if let Some(d) = duration_ms {
        times = times.end((start_ms + i64::from(d)) / 1000);
    }
    // Discord shows `large_text` as the third line of the card, under the artist.
    let assets = Assets::new().large_image(cover).large_text(match &t.album {
        Some(a) => fit(&a.name),
        None => "XP Music".to_string(),
    });
    let mut activity = Activity::new()
        .activity_type(ActivityType::Listening)
        .details(fit(&t.title))
        .timestamps(times)
        .assets(assets);
    // Optional: without an artist the line is left out instead of showing a blank one.
    if !artist.is_empty() {
        activity = activity.state(fit(&artist));
    }
    activity
}

/// Sends the activity; `false` if Discord did not take it.
fn publish(c: &mut DiscordIpcClient, t: &Track, start_ms: i64, duration_ms: Option<u32>) -> bool {
    c.set_activity(build(t, start_ms, duration_ms))
        .map_err(|e| tracing::warn!(error = %e, "Discord rejected the activity"))
        .is_ok()
}

fn run(rx: Receiver<Box<PlayerState>>, enabled: impl Fn() -> bool) {
    // `Some` = connected.
    let mut client: Option<DiscordIpcClient> = None;
    let mut last: Option<(Box<PlayerState>, Instant)> = None;
    // What Discord is showing now: track id and start time (ms).
    let mut shown: Option<(String, i64)> = None;
    let mut retry_at = Instant::now();

    loop {
        // The timeout also re-evaluates: turning the setting on mid-song, or Discord opening later.
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(s) => last = Some((s, Instant::now())),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let on = enabled();
        let target = match &last {
            Some((s, at)) if on => match s.status {
                PlaybackStatus::Playing => s.track.as_ref().map(|t| {
                    let elapsed = i64::try_from(at.elapsed().as_millis()).unwrap_or(0);
                    (s, t, now_ms() - i64::from(s.position_ms) - elapsed)
                }),
                // Transient: keep showing what there is.
                PlaybackStatus::Loading | PlaybackStatus::Buffering => continue,
                _ => None,
            },
            _ => None,
        };

        match target {
            Some((s, t, start)) => {
                let same = shown
                    .as_ref()
                    .is_some_and(|(id, st)| *id == t.id.0 && (st - start).abs() < DRIFT_MS);
                if same || Instant::now() < retry_at {
                    continue;
                }
                let c = match client.as_mut() {
                    Some(c) => c,
                    None => {
                        let mut c = DiscordIpcClient::new(CLIENT_ID);
                        if let Err(e) = c.connect() {
                            tracing::debug!(error = %e, "Discord is not reachable");
                            retry_at = Instant::now() + RETRY;
                            continue;
                        }
                        client.insert(c)
                    }
                };
                if publish(c, t, start, s.duration_ms) {
                    shown = Some((t.id.0.clone(), start));
                } else {
                    client = None;
                    shown = None;
                    retry_at = Instant::now() + RETRY;
                }
            }
            None => {
                if shown.take().is_some()
                    && let Some(c) = client.as_mut()
                {
                    let _ = c.clear_activity();
                }
                if !on && let Some(mut c) = client.take() {
                    let _ = c.close();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build, fit};
    use crate::domain::{AlbumRef, ArtistRef, Thumbnail, Track, TrackId};
    use serde_json::{Value, json};

    fn track() -> Track {
        Track {
            id: TrackId("id".into()),
            title: "Song".into(),
            artists: vec![ArtistRef {
                id: None,
                name: "Artist".into(),
            }],
            album: Some(AlbumRef {
                id: None,
                name: "Album".into(),
            }),
            duration_ms: Some(60_000),
            thumbnails: vec![Thumbnail {
                url: "https://example.com/cover.jpg".into(),
                width: 544,
                height: 544,
            }],
            explicit: false,
            like_status: None,
            set_video_id: None,
        }
    }

    fn activity(t: &Track) -> Value {
        serde_json::to_value(build(t, 1_700_000_000_500, Some(60_000))).unwrap()
    }

    #[test]
    fn fit_pads_short_and_truncates_long() {
        assert_eq!(fit("7").chars().count(), 2);
        assert_eq!(fit("").chars().count(), 2);
        assert_eq!(fit("ab"), "ab");
        assert_eq!(fit(&"é".repeat(300)).chars().count(), 128);
    }

    #[test]
    fn full_track() {
        let a = activity(&track());
        assert_eq!(a["type"], 2);
        assert_eq!(a["details"], "Song");
        assert_eq!(a["state"], "Artist");
        assert_eq!(a["assets"]["large_image"], "https://example.com/cover.jpg");
        assert_eq!(a["assets"]["large_text"], "Album");
        // Seconds, not milliseconds.
        assert_eq!(
            a["timestamps"],
            json!({ "start": 1_700_000_000, "end": 1_700_000_060 })
        );
    }

    #[test]
    fn fallbacks_without_album_cover_or_artist() {
        let mut t = track();
        t.album = None;
        t.thumbnails.clear();
        t.artists.clear();
        let a = activity(&t);
        assert_eq!(a["assets"]["large_image"], "icon-1024");
        assert_eq!(a["assets"]["large_text"], "XP Music");
        assert!(a.get("state").is_none());

        // A cover URL Discord would reject is replaced too.
        let mut t = track();
        t.thumbnails[0].url = format!("https://example.com/{}", "x".repeat(300));
        assert_eq!(activity(&t)["assets"]["large_image"], "icon-1024");
    }
}
