//! Folder scanning: walks the folders, reads tags with symphonia (the same library
//! that plays the files) and keeps the `local_tracks` table in sync.
//!
//! Files whose size and modification date have not changed are not opened again,
//! so a rescan of a library that is already known only costs one `stat` per file.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTag, Tag};

use crate::audio::decoder::track_duration_ms;
use crate::error::AppResult;
use crate::storage::Db;

/// Extensions symphonia can decode with the enabled features. `.ogg` may also hold
/// Opus, which it cannot: those files are rejected when probed.
pub const EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "m4b", "aac", "flac", "ogg", "oga", "wav", "aif", "aiff",
];
/// Images next to the songs that are used as the album cover.
const FOLDER_COVERS: &[&str] = &[
    "cover.jpg",
    "cover.jpeg",
    "cover.png",
    "folder.jpg",
    "folder.jpeg",
    "folder.png",
    "front.jpg",
    "front.png",
];
/// `cover` value for art embedded in the file itself.
pub const EMBEDDED: &str = "embedded";
/// Rows written per database lock (the connection is shared with the whole app).
const BATCH: usize = 100;

/// What is read from a file.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Tags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub year: Option<u32>,
    pub duration_ms: Option<u32>,
    pub embedded_cover: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScanSummary {
    pub total: u32,
    pub read: u32,
    pub removed: u32,
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Audio files under `folder`, recursively. Symbolic links are not followed (no loops).
pub fn walk(folder: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else { continue };
            let path = entry.path();
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file() && is_audio(&path) {
                out.push(path);
            }
        }
    }
    out
}

/// Opens `path` with symphonia. `None` if it is not audio it can play.
pub fn probe(path: &Path) -> Option<Box<dyn FormatReader>> {
    let file = File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(&ext.to_ascii_lowercase());
    }
    symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .ok()
}

/// Runs `f`, which parses `path` with symphonia, so that a parser panic on a malformed or
/// hostile file is logged and treated as "nothing read" instead of killing the thread.
pub fn guarded<T>(path: &Path, f: impl FnOnce() -> Option<T>) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
        tracing::warn!(path = %path.display(), "the audio parser panicked on this file; skipped");
        None
    })
}

/// Reads the tags of a file; `None` if it is not playable.
pub fn read_tags(path: &Path) -> Option<Tags> {
    let mut format = probe(path)?;
    let track = format.default_track(TrackType::Audio)?;
    let params = track.codec_params.as_ref()?.audio()?;
    // Only what can be decoded is listed (an Opus `.ogg` cannot).
    symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .ok()?;
    let track_id = track.id as u64;
    let mut tags = Tags {
        duration_ms: track_duration_ms(track),
        ..Default::default()
    };
    // The probe may have found several revisions (an ID3v2 before the stream, the
    // container's own…): they are all applied in order, the latest wins.
    let mut metadata = format.metadata();
    loop {
        if let Some(rev) = metadata.current() {
            let per_track = rev
                .per_track
                .iter()
                .filter(|t| t.track_id == track_id)
                .map(|t| &t.metadata);
            for container in std::iter::once(&rev.media).chain(per_track) {
                container.tags.iter().for_each(|t| apply(&mut tags, t));
                tags.embedded_cover |= !container.visuals.is_empty();
            }
        }
        if metadata.pop().is_none() {
            break;
        }
    }
    Some(tags)
}

fn text(s: &str) -> Option<String> {
    let s = s.trim().trim_matches('\0').trim();
    (!s.is_empty()).then(|| s.to_owned())
}

/// Main artist of a credit: "Romeo Santos, Drake" or "Bad Bunny feat. Drake" →
/// the first one. Used to group an album whose files have no album artist tag
/// (otherwise every song with a guest would make an album of its own). "&" is
/// kept: it is usually part of the name ("Simon & Garfunkel").
pub(crate) fn primary_artist(artist: &str) -> &str {
    const SEPARATORS: &[&str] = &[", ", "; ", " feat. ", " feat ", " ft. ", " featuring "];
    let lower = artist.to_lowercase();
    let cut = SEPARATORS
        .iter()
        .filter_map(|s| lower.find(s))
        .min()
        .unwrap_or(artist.len());
    // `to_lowercase` may change byte lengths; fall back to the whole credit then.
    artist.get(..cut).map(str::trim).unwrap_or(artist)
}

/// "2003-05-12" → 2003.
fn year_of(s: &str) -> Option<u32> {
    s.trim()
        .get(..4)
        .and_then(|y| y.parse().ok())
        .filter(|y| *y > 0)
}

fn apply(tags: &mut Tags, tag: &Tag) {
    let Some(std) = &tag.std else { return };
    match std {
        StandardTag::TrackTitle(v) => tags.title = text(v).or(tags.title.take()),
        StandardTag::Artist(v) => tags.artist = text(v).or(tags.artist.take()),
        StandardTag::Album(v) => tags.album = text(v).or(tags.album.take()),
        StandardTag::AlbumArtist(v) => tags.album_artist = text(v).or(tags.album_artist.take()),
        StandardTag::TrackNumber(n) => tags.track_no = u32::try_from(*n).ok(),
        StandardTag::DiscNumber(n) => tags.disc_no = u32::try_from(*n).ok(),
        StandardTag::ReleaseYear(y) | StandardTag::RecordingYear(y) => {
            tags.year = tags.year.or(Some(u32::from(*y)))
        }
        StandardTag::ReleaseDate(d) | StandardTag::RecordingDate(d) => {
            tags.year = tags.year.or(year_of(d))
        }
        _ => {}
    }
}

/// A cover image in the song's folder (cached per folder during a scan).
fn folder_cover(dir: &Path, cache: &mut HashMap<PathBuf, Option<String>>) -> Option<String> {
    cache
        .entry(dir.to_path_buf())
        .or_insert_with(|| {
            let names: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
                .ok()?
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                .map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.path()))
                .collect();
            FOLDER_COVERS.iter().find_map(|want| {
                names
                    .iter()
                    .find(|(n, _)| n == want)
                    .map(|(_, p)| p.display().to_string())
            })
        })
        .clone()
}

fn stat(path: &Path) -> Option<(i64, i64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some((mtime, meta.len() as i64))
}

struct Row {
    path: String,
    folder: String,
    tags: Tags,
    cover: Option<String>,
    mtime: i64,
    size: i64,
}

fn write_rows(db: &Db, rows: &mut Vec<Row>) -> AppResult<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut conn = db.conn();
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO local_tracks (path, folder, title, artist, album, album_artist, track_no,
                                       disc_no, year, duration_ms, cover, mtime, size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(path) DO UPDATE SET folder = excluded.folder, title = excluded.title,
                 artist = excluded.artist, album = excluded.album, album_artist = excluded.album_artist,
                 track_no = excluded.track_no, disc_no = excluded.disc_no, year = excluded.year,
                 duration_ms = excluded.duration_ms, cover = excluded.cover,
                 mtime = excluded.mtime, size = excluded.size",
        )?;
        for r in rows.drain(..) {
            let t = r.tags;
            let title = t.title.unwrap_or_else(|| {
                Path::new(&r.path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let artist = t.artist.unwrap_or_default();
            let album_artist = t
                .album_artist
                .unwrap_or_else(|| primary_artist(&artist).to_owned());
            stmt.execute(rusqlite::params![
                r.path,
                r.folder,
                title,
                artist,
                t.album.unwrap_or_default(),
                album_artist,
                t.track_no,
                t.disc_no,
                t.year,
                t.duration_ms,
                r.cover,
                r.mtime,
                r.size,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Brings `local_tracks` in line with `folders`: new or changed files are read,
/// unchanged ones are skipped and the ones no longer there (or in a folder that
/// was removed) are deleted. `progress(done, total)` is called as it goes.
pub fn scan(
    db: &Db,
    folders: &[PathBuf],
    mut progress: impl FnMut(u32, u32),
) -> AppResult<ScanSummary> {
    let known: HashMap<String, (i64, i64)> = {
        let conn = db.conn();
        let mut stmt = conn.prepare("SELECT path, mtime, size FROM local_tracks")?;
        stmt.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
            .collect::<Result<_, _>>()?
    };
    let files: Vec<(PathBuf, String)> = folders
        .iter()
        .flat_map(|f| {
            let folder = f.display().to_string();
            walk(f).into_iter().map(move |p| (p, folder.clone()))
        })
        .collect();
    let total = files.len() as u32;
    let mut summary = ScanSummary {
        total,
        ..Default::default()
    };
    let mut seen = HashSet::with_capacity(files.len());
    let mut covers = HashMap::new();
    let mut rows = Vec::with_capacity(BATCH);
    for (i, (path, folder)) in files.into_iter().enumerate() {
        progress(i as u32, total);
        let key = path.display().to_string();
        let Some((mtime, size)) = stat(&path) else {
            continue;
        };
        if known.get(&key) == Some(&(mtime, size)) {
            seen.insert(key);
            continue;
        }
        let Some(tags) = guarded(&path, || read_tags(&path)) else {
            tracing::debug!(path = %key, "not playable audio; skipped");
            continue;
        };
        let cover = if tags.embedded_cover {
            Some(EMBEDDED.to_owned())
        } else {
            path.parent().and_then(|d| folder_cover(d, &mut covers))
        };
        seen.insert(key.clone());
        summary.read += 1;
        rows.push(Row {
            path: key,
            folder,
            tags,
            cover,
            mtime,
            size,
        });
        if rows.len() >= BATCH {
            write_rows(db, &mut rows)?;
        }
    }
    write_rows(db, &mut rows)?;

    let gone: Vec<&String> = known.keys().filter(|k| !seen.contains(*k)).collect();
    if !gone.is_empty() {
        let mut conn = db.conn();
        let tx = conn.transaction()?;
        for path in &gone {
            tx.execute("DELETE FROM local_tracks WHERE path = ?1", [path])?;
        }
        tx.commit()?;
    }
    summary.removed = gone.len() as u32;
    progress(total, total);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::fixtures::{mp3_with_id3, temp_dir, wav};

    fn count(db: &Db) -> i64 {
        db.conn()
            .query_row("SELECT COUNT(*) FROM local_tracks", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn untagged_wav_gives_its_duration() {
        // symphonia 0.6 reads a WAV's `INFO` list but does not hand it over: WAVs
        // are listed by file name.
        let dir = temp_dir("scan-wav");
        let path = dir.join("song.wav");
        std::fs::write(&path, wav(44_100, 2, 44_100)).unwrap();
        let tags = read_tags(&path).expect("playable");
        assert_eq!(tags.title, None);
        assert_eq!(tags.duration_ms, Some(1000));
        assert!(!tags.embedded_cover);
    }

    #[test]
    fn reads_id3v2_tags_and_embedded_cover() {
        let dir = temp_dir("scan-id3");
        let path = dir.join("song.mp3");
        std::fs::write(&path, mp3_with_id3("Intro", "Moby", "Play", 3, true)).unwrap();
        let tags = read_tags(&path).expect("playable");
        assert_eq!(tags.title.as_deref(), Some("Intro"));
        assert_eq!(tags.artist.as_deref(), Some("Moby"));
        assert_eq!(tags.album.as_deref(), Some("Play"));
        assert_eq!(tags.track_no, Some(3));
        assert!(tags.embedded_cover);
    }

    #[test]
    fn untagged_file_uses_its_name_and_folder_cover() {
        let db = Db::open_in_memory().unwrap();
        let dir = temp_dir("scan-untagged");
        let album = dir.join("Album");
        std::fs::create_dir_all(&album).unwrap();
        std::fs::write(album.join("01 - Untitled.wav"), wav(8_000, 1, 800)).unwrap();
        std::fs::write(album.join("Folder.JPG"), b"\xFF\xD8\xFF").unwrap();
        std::fs::write(album.join("notes.txt"), b"not audio").unwrap();
        std::fs::write(album.join("broken.mp3"), b"not audio either").unwrap();

        let s = scan(&db, std::slice::from_ref(&dir), |_, _| {}).unwrap();
        assert_eq!((s.total, s.read), (2, 1), "the broken mp3 is skipped");
        let (title, cover): (String, Option<String>) = db
            .conn()
            .query_row("SELECT title, cover FROM local_tracks", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(title, "01 - Untitled");
        assert!(cover.unwrap().to_lowercase().ends_with("folder.jpg"));
    }

    #[test]
    fn rescan_skips_unchanged_and_removes_gone_files() {
        let db = Db::open_in_memory().unwrap();
        let dir = temp_dir("scan-rescan");
        let sub = dir.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(dir.join("a.wav"), wav(8_000, 1, 800)).unwrap();
        std::fs::write(sub.join("b.wav"), wav(8_000, 1, 800)).unwrap();
        let folders = vec![dir.clone()];

        let first = scan(&db, &folders, |_, _| {}).unwrap();
        assert_eq!((first.total, first.read, first.removed), (2, 2, 0));
        let again = scan(&db, &folders, |_, _| {}).unwrap();
        assert_eq!(again.read, 0, "unchanged files are not opened again");

        std::fs::remove_file(sub.join("b.wav")).unwrap();
        let after = scan(&db, &folders, |_, _| {}).unwrap();
        assert_eq!((after.total, after.removed), (1, 1));
        assert_eq!(count(&db), 1);

        // Without the folder in the list, its songs leave the library.
        scan(&db, &[], |_, _| {}).unwrap();
        assert_eq!(count(&db), 0);
    }

    #[test]
    fn albums_without_album_artist_group_by_the_main_artist() {
        assert_eq!(primary_artist("Romeo Santos, Drake"), "Romeo Santos");
        assert_eq!(primary_artist("Bad Bunny feat. Drake"), "Bad Bunny");
        assert_eq!(primary_artist("Ozuna Ft. Anuel AA"), "Ozuna");
        assert_eq!(primary_artist("Simon & Garfunkel"), "Simon & Garfunkel");
        assert_eq!(primary_artist("Aventura"), "Aventura");
        assert_eq!(primary_artist(""), "");

        let db = Db::open_in_memory().unwrap();
        let dir = temp_dir("scan-album-artist");
        std::fs::write(
            dir.join("1.mp3"),
            mp3_with_id3("A", "Romeo Santos", "Golden", 1, false),
        )
        .unwrap();
        std::fs::write(
            dir.join("2.mp3"),
            mp3_with_id3("B", "Romeo Santos, Ozuna", "Golden", 2, false),
        )
        .unwrap();
        scan(&db, std::slice::from_ref(&dir), |_, _| {}).unwrap();
        let groups: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(DISTINCT album_artist || '|' || album) FROM local_tracks",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(groups, 1, "one album, not one per guest");
    }

    #[test]
    fn year_from_dates() {
        assert_eq!(year_of("2003-05-12"), Some(2003));
        assert_eq!(year_of("1999"), Some(1999));
        assert_eq!(year_of("abc"), None);
        assert_eq!(year_of("0000"), None);
    }
}
