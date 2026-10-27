//! "My music": songs in folders on the user's disk, playable without a YouTube
//! session. The folders live in the settings; what is in them, in `local_tracks`.

pub mod scan;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri::http::{Request, Response, StatusCode};
use tauri_specta::Event;

use crate::domain::{AlbumRef, ArtistRef, BrowseId, Thumbnail, Track, TrackId};
use crate::error::AppResult;
use crate::settings::Settings;
use crate::storage::Db;

/// How often the scan progress is sent to the UI.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// Largest cover served (some files embed huge scans).
const MAX_COVER: usize = 8 * 1024 * 1024;
/// Prefix of the thumbnail "URL" of a local song; the UI turns it into `cover://`.
const COVER_PREFIX: &str = "cover:";
/// Prefix of the album and artist ids of local songs (YouTube's never have it).
const LOCAL_REF: &str = "local:";

#[derive(Serialize, Type, Event, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "local://scan")]
pub struct LocalScanStatus {
    pub scanning: bool,
    pub done: u32,
    pub total: u32,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalAlbum {
    /// Empty = untagged.
    pub title: String,
    pub artist: String,
    pub year: Option<u32>,
    pub track_count: u32,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalArtist {
    /// Empty = untagged.
    pub name: String,
    pub album_count: u32,
    pub track_count: u32,
    pub thumbnails: Vec<Thumbnail>,
}

/// Which songs to list.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
pub enum LocalFilter {
    All,
    Artist(String),
    Album { artist: String, title: String },
}

/// Opaque id of a song's cover in the `cover` protocol. The URL reaches the WMP skins,
/// whose scripts come from third parties: it must not carry the file's path.
fn cover_key(path: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(path.as_bytes())[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn cover_thumbs(path: Option<String>) -> Vec<Thumbnail> {
    path.map(|p| Thumbnail {
        url: format!("{COVER_PREFIX}{}", cover_key(&p)),
        width: 300,
        height: 300,
    })
    .into_iter()
    .collect()
}

fn row_to_track(r: &rusqlite::Row<'_>) -> rusqlite::Result<Track> {
    let path: String = r.get("path")?;
    let artist: String = r.get("artist")?;
    let album: String = r.get("album")?;
    let album_artist: String = r.get("album_artist")?;
    let has_cover = r.get::<_, Option<String>>("cover")?.is_some();
    // The ids lead to "My music" pages: `local:<album artist>` for the album (its
    // title is the name) and `local:<main artist>` for the artist.
    Ok(Track {
        id: TrackId::local(Path::new(&path)),
        title: r.get("title")?,
        artists: (!artist.is_empty())
            .then(|| ArtistRef {
                id: Some(BrowseId(format!(
                    "{LOCAL_REF}{}",
                    scan::primary_artist(&artist)
                ))),
                name: artist,
            })
            .into_iter()
            .collect(),
        album: (!album.is_empty()).then_some(AlbumRef {
            id: Some(BrowseId(format!("{LOCAL_REF}{album_artist}"))),
            name: album,
        }),
        duration_ms: r.get("duration_ms")?,
        thumbnails: cover_thumbs(has_cover.then_some(path)),
        explicit: false,
        like_status: None,
        set_video_id: None,
    })
}

/// Songs in album order (artist, album, disc, track).
pub fn tracks(db: &Db, filter: &LocalFilter) -> AppResult<Vec<Track>> {
    const ORDER: &str = "ORDER BY album_artist COLLATE NOCASE, album COLLATE NOCASE,
                         disc_no, track_no, title COLLATE NOCASE";
    let conn = db.conn();
    let tracks = match filter {
        LocalFilter::All => {
            let mut stmt = conn.prepare(&format!("SELECT * FROM local_tracks {ORDER}"))?;
            stmt.query_map([], row_to_track)?
                .collect::<Result<Vec<_>, _>>()?
        }
        LocalFilter::Artist(name) => {
            let mut stmt = conn.prepare(&format!(
                "SELECT * FROM local_tracks WHERE artist = ?1 OR album_artist = ?1 {ORDER}"
            ))?;
            stmt.query_map([name], row_to_track)?
                .collect::<Result<Vec<_>, _>>()?
        }
        LocalFilter::Album { artist, title } => {
            let mut stmt = conn.prepare(&format!(
                "SELECT * FROM local_tracks WHERE album_artist = ?1 AND album = ?2 {ORDER}"
            ))?;
            stmt.query_map([artist, title], row_to_track)?
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    Ok(tracks)
}

pub fn albums(db: &Db) -> AppResult<Vec<LocalAlbum>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT album, album_artist, MAX(year), COUNT(*),
                MAX(CASE WHEN cover IS NOT NULL THEN path END)
         FROM local_tracks GROUP BY album_artist, album
         ORDER BY album_artist COLLATE NOCASE, album COLLATE NOCASE",
    )?;
    let albums = stmt
        .query_map([], |r| {
            Ok(LocalAlbum {
                title: r.get(0)?,
                artist: r.get(1)?,
                year: r.get(2)?,
                track_count: r.get(3)?,
                thumbnails: cover_thumbs(r.get(4)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(albums)
}

pub fn artists(db: &Db) -> AppResult<Vec<LocalArtist>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT artist, COUNT(DISTINCT album), COUNT(*),
                MAX(CASE WHEN cover IS NOT NULL THEN path END)
         FROM local_tracks GROUP BY artist ORDER BY artist COLLATE NOCASE",
    )?;
    let artists = stmt
        .query_map([], |r| {
            Ok(LocalArtist {
                name: r.get(0)?,
                album_count: r.get(1)?,
                track_count: r.get(2)?,
                thumbnails: cover_thumbs(r.get(3)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(artists)
}

// ---------------------------------------------------------------- covers

/// Cover of a library song, by its `cover_key`: the image embedded in the file or the
/// one in its folder. Only songs in the library are served (never an arbitrary file).
fn cover(db: &Db, key: &str) -> Option<(String, Vec<u8>)> {
    if key.len() != 32 || !key.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    // The key is a hash: find the song it belongs to (hashing a path takes well under a
    // microsecond, so even large libraries answer in a few milliseconds).
    let (path, source) = {
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT path, cover FROM local_tracks WHERE cover IS NOT NULL")
            .ok()?;
        let mut rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .ok()?;
        rows.find_map(|row| row.ok().filter(|(p, _)| cover_key(p) == key))?
    };
    if source != scan::EMBEDDED {
        let bytes = std::fs::read(&source)
            .ok()
            .filter(|b| b.len() <= MAX_COVER)?;
        let mime = if source.to_lowercase().ends_with(".png") {
            "image/png"
        } else {
            "image/jpeg"
        };
        return Some((mime.to_owned(), bytes));
    }
    scan::guarded(Path::new(&path), || embedded_cover(Path::new(&path)))
}

/// The image embedded in an audio file, preferring the front cover.
fn embedded_cover(path: &Path) -> Option<(String, Vec<u8>)> {
    let mut format = scan::probe(path)?;
    let mut metadata = format.metadata();
    let mut best: Option<symphonia::core::meta::Visual> = None;
    loop {
        if let Some(rev) = metadata.current() {
            let visuals = rev
                .media
                .visuals
                .iter()
                .chain(rev.per_track.iter().flat_map(|t| &t.metadata.visuals));
            for v in visuals {
                let front = matches!(
                    v.usage,
                    Some(symphonia::core::meta::StandardVisualKey::FrontCover)
                );
                if best.is_none() || front {
                    best = Some(v.clone());
                }
            }
        }
        if metadata.pop().is_none() {
            break;
        }
    }
    let v = best.filter(|v| v.data.len() <= MAX_COVER)?;
    let mime = v
        .media_type
        .filter(|m| m.starts_with("image/"))
        .unwrap_or_else(|| "image/jpeg".to_owned());
    Some((mime, v.data.into_vec()))
}

/// `cover` protocol: `http://cover.localhost/<cover_key>`. No CORS header: `<img>`
/// does not need one, and without it no other page can read the covers.
pub fn handle_cover(db: &Db, req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let key = req.uri().path().trim_start_matches('/');
    let found = cover(db, key);
    let builder = Response::builder();
    match found {
        Some((mime, bytes)) => builder
            .header("Content-Type", mime)
            .header("Cache-Control", "max-age=3600")
            .body(bytes),
        None => builder.status(StatusCode::NOT_FOUND).body(Vec::new()),
    }
    .expect("cover response")
}

// ---------------------------------------------------------------- service

#[derive(Default)]
struct ScanState {
    running: bool,
    /// Asked again while scanning: one more pass when it finishes.
    again: bool,
    status: LocalScanStatus,
}

/// Runs the scans (one at a time, in the background) and reports their progress.
pub struct LocalLibrary {
    app: Option<AppHandle>,
    db: Arc<Db>,
    state: Mutex<ScanState>,
}

impl LocalLibrary {
    pub fn new(app: Option<AppHandle>, db: Arc<Db>) -> Arc<Self> {
        Arc::new(Self {
            app,
            db,
            state: Mutex::new(ScanState::default()),
        })
    }

    pub fn folders(&self) -> Vec<String> {
        Settings::load(&self.db)
            .map(|s| s.local_folders)
            .unwrap_or_default()
    }

    pub fn status(&self) -> LocalScanStatus {
        self.state.lock().unwrap().status
    }

    fn emit(&self, status: LocalScanStatus) {
        self.state.lock().unwrap().status = status;
        if let Some(app) = &self.app {
            let _ = status.emit(app);
        }
    }

    /// Saves the new folder list, notifies the settings and rescans.
    pub fn set_folders(self: &Arc<Self>, folders: Vec<String>) -> AppResult<Vec<String>> {
        let s = Settings::load(&self.db)?;
        let saved = Settings {
            local_folders: folders,
            ..s
        }
        .save(&self.db)?;
        if let Some(app) = &self.app {
            let _ = saved.clone().emit(app);
        }
        self.rescan();
        Ok(saved.local_folders)
    }

    /// Scans in the background. Without folders it only clears what was left over.
    pub fn rescan(self: &Arc<Self>) {
        {
            let mut st = self.state.lock().unwrap();
            if st.running {
                st.again = true;
                return;
            }
            st.running = true;
        }
        let me = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("local-scan".into())
            .spawn(move || me.scan_loop());
        if spawned.is_err() {
            self.state.lock().unwrap().running = false;
        }
    }

    fn scan_loop(&self) {
        loop {
            let folders: Vec<PathBuf> = self.folders().into_iter().map(PathBuf::from).collect();
            let t0 = Instant::now();
            let mut last = Instant::now() - PROGRESS_EVERY;
            self.emit(LocalScanStatus {
                scanning: true,
                done: 0,
                total: 0,
            });
            // A panic must not leave `running` set: no scan would ever start again.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                scan::scan(&self.db, &folders, |done, total| {
                    if last.elapsed() >= PROGRESS_EVERY {
                        last = Instant::now();
                        self.emit(LocalScanStatus {
                            scanning: true,
                            done,
                            total,
                        });
                    }
                })
            }))
            .unwrap_or_else(|_| Err(crate::error::AppError::internal("the scan panicked")));
            let total = match result {
                Ok(s) => {
                    tracing::info!(
                        files = s.total,
                        read = s.read,
                        removed = s.removed,
                        ms = t0.elapsed().as_millis() as u64,
                        "local library scanned"
                    );
                    s.total
                }
                Err(e) => {
                    tracing::warn!(error = %e, "local library scan failed");
                    0
                }
            };
            let mut st = self.state.lock().unwrap();
            if st.again {
                st.again = false;
                continue;
            }
            st.running = false;
            drop(st);
            self.emit(LocalScanStatus {
                scanning: false,
                done: total,
                total,
            });
            return;
        }
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    //! Tiny audio files built in memory, so no binary fixture is committed.
    use std::path::PathBuf;

    pub fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xp-music-{name}-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// 16-bit PCM WAV with a 440 Hz tone.
    pub fn wav(rate: u32, channels: u16, frames: u32) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend(1u16.to_le_bytes());
        fmt.extend(channels.to_le_bytes());
        fmt.extend(rate.to_le_bytes());
        fmt.extend((rate * channels as u32 * 2).to_le_bytes());
        fmt.extend((channels * 2).to_le_bytes());
        fmt.extend(16u16.to_le_bytes());
        let mut data = Vec::with_capacity(frames as usize * channels as usize * 2);
        for i in 0..frames {
            let v =
                ((i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 8000.0) as i16;
            for _ in 0..channels {
                data.extend(v.to_le_bytes());
            }
        }
        let mut body = b"WAVE".to_vec();
        body.extend(chunk(b"fmt ", &fmt));
        body.extend(chunk(b"data", &data));
        chunk(b"RIFF", &body)
    }

    fn id3_frame(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend((body.len() as u32).to_be_bytes());
        out.extend([0, 0]);
        out.extend(body);
        out
    }

    fn id3_text(id: &[u8; 4], text: &str) -> Vec<u8> {
        let mut body = vec![0u8]; // ISO-8859-1
        body.extend(text.as_bytes());
        id3_frame(id, &body)
    }

    /// MP3 of silent MPEG-1 Layer III frames behind an ID3v2.3 tag (with an
    /// `APIC` cover if `cover`).
    pub fn mp3_with_id3(
        title: &str,
        artist: &str,
        album: &str,
        track: u32,
        cover: bool,
    ) -> Vec<u8> {
        let mut frames = Vec::new();
        frames.extend(id3_text(b"TIT2", title));
        frames.extend(id3_text(b"TPE1", artist));
        frames.extend(id3_text(b"TALB", album));
        frames.extend(id3_text(b"TRCK", &track.to_string()));
        if cover {
            let mut apic = vec![0u8];
            apic.extend(b"image/jpeg\0");
            apic.push(3); // front cover
            apic.push(0); // no description
            apic.extend([0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3, 4]);
            frames.extend(id3_frame(b"APIC", &apic));
        }
        let size = frames.len() as u32;
        let syncsafe = [
            (size >> 21) as u8 & 0x7F,
            (size >> 14) as u8 & 0x7F,
            (size >> 7) as u8 & 0x7F,
            size as u8 & 0x7F,
        ];
        let mut out = b"ID3\x03\x00\x00".to_vec();
        out.extend(syncsafe);
        out.extend(frames);
        // 128 kbps, 44.1 kHz, mono: 417 bytes per frame; all-zero side info = silence.
        for _ in 0..40 {
            let mut f = vec![0u8; 417];
            f[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0xC4]);
            out.extend(f);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{mp3_with_id3, temp_dir, wav};
    use super::*;

    fn library() -> (Arc<Db>, PathBuf) {
        let db = Arc::new(Db::open_in_memory().unwrap());
        let dir = temp_dir("library");
        let a = dir.join("Moby - Play");
        let b = dir.join("misc");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(
            a.join("03.mp3"),
            mp3_with_id3("Porcelain", "Moby", "Play", 3, true),
        )
        .unwrap();
        std::fs::write(
            a.join("01.mp3"),
            mp3_with_id3("Honey", "Moby", "Play", 1, false),
        )
        .unwrap();
        std::fs::write(b.join("Tone.wav"), wav(8_000, 1, 800)).unwrap();
        scan::scan(&db, std::slice::from_ref(&dir), |_, _| {}).unwrap();
        (db, dir)
    }

    #[test]
    fn lists_songs_albums_and_artists() {
        let (db, _dir) = library();
        let all = tracks(&db, &LocalFilter::All).unwrap();
        let titles: Vec<_> = all.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Tone", "Honey", "Porcelain"],
            "by artist, then track number"
        );
        assert!(all.iter().all(|t| t.id.is_local()));
        let album = all[1].album.as_ref().unwrap();
        assert_eq!(album.name, "Play");
        // Links to the "My music" pages, never mistaken for YouTube ids.
        assert_eq!(album.id.as_ref().unwrap().0, "local:Moby");
        assert_eq!(all[1].artists[0].id.as_ref().unwrap().0, "local:Moby");

        let play = tracks(
            &db,
            &LocalFilter::Album {
                artist: "Moby".into(),
                title: "Play".into(),
            },
        )
        .unwrap();
        assert_eq!(play.len(), 2);

        let albums = albums(&db).unwrap();
        let moby = albums.iter().find(|a| a.title == "Play").unwrap();
        assert_eq!(moby.track_count, 2);
        assert_eq!(moby.thumbnails.len(), 1, "one of its songs has a cover");

        let artists = artists(&db).unwrap();
        let names: Vec<_> = artists.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["", "Moby"], "untagged songs form their own group");
        assert!(all[0].artists.is_empty());
        assert_eq!(
            tracks(&db, &LocalFilter::Artist("Moby".into()))
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn serves_embedded_covers_only_for_library_songs() {
        let (db, dir) = library();
        let song = dir.join("Moby - Play").join("03.mp3").display().to_string();
        let (mime, bytes) = cover(&db, &cover_key(&song)).expect("embedded cover");
        assert_eq!(mime, "image/jpeg");
        assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF]);
        let no_cover = dir.join("Moby - Play").join("01.mp3").display().to_string();
        assert!(cover(&db, &cover_key(&no_cover)).is_none());
        assert!(cover(&db, &cover_key(r"C:\Windows\win.ini")).is_none());
        // Only keys: a path (or anything that is not 32 hex digits) finds nothing.
        assert!(cover(&db, &song).is_none());
        assert!(cover(&db, "../../etc/passwd").is_none());
    }

    /// The cover URL reaches the WMP skins (third-party scripts): it carries an opaque
    /// key, never the song's path.
    #[test]
    fn cover_urls_do_not_carry_the_path() {
        let (db, dir) = library();
        let folder = dir.display().to_string();
        let urls: Vec<String> = tracks(&db, &LocalFilter::All)
            .unwrap()
            .into_iter()
            .flat_map(|t| t.thumbnails)
            .chain(albums(&db).unwrap().into_iter().flat_map(|a| a.thumbnails))
            .map(|t| t.url)
            .collect();
        assert!(!urls.is_empty());
        for url in urls {
            let key = url.strip_prefix(COVER_PREFIX).expect("cover URL");
            assert_eq!(key.len(), 32, "{url}");
            assert!(key.bytes().all(|b| b.is_ascii_hexdigit()), "{url}");
            assert!(!url.contains(&folder) && !url.contains("Moby"), "{url}");
        }
    }

    #[test]
    fn cover_protocol_has_no_cors_header() {
        let (db, dir) = library();
        let song = dir.join("Moby - Play").join("03.mp3").display().to_string();
        let get = |path: String| {
            let req = Request::builder()
                .uri(format!("http://cover.localhost/{path}"))
                .body(Vec::new())
                .unwrap();
            handle_cover(&db, &req)
        };
        let ok = get(cover_key(&song));
        assert_eq!(ok.status(), StatusCode::OK);
        assert!(ok.headers().get("Access-Control-Allow-Origin").is_none());
        assert_eq!(get("not-a-key".into()).status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_parser_panic_counts_as_unreadable() {
        let hostile = std::path::Path::new("hostile.mp3");
        assert_eq!(
            scan::guarded(hostile, || -> Option<u8> { panic!("parser bug") }),
            None
        );
        assert_eq!(scan::guarded(hostile, || Some(7)), Some(7));
    }
}
