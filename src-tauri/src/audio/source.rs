//! `MediaSource` over HTTP with Range requests (FR-002-01/03, NFR-002-03).
//!
//! A downloader thread keeps 256 KiB blocks around the read position
//! (32 ahead, 8 behind: ≤ 10 MiB). `read` blocks until the needed block
//! arrives; `seek` only moves the position.

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_RANGE, RANGE, USER_AGENT};

use super::fast::HEAD_LIMIT;
use super::resolver::Upgrade;

/// 256 KiB: the first block arrives quickly and playback starts sooner (008).
pub const CHUNK: u64 = 1 << 18;
const AHEAD: u64 = 32;
const KEEP_BEHIND: u64 = 8;
const RETRIES: u32 = 3;
/// The longest we wait for the full URL during a fast start. yt-dlp already has
/// its own limits; this only avoids waiting forever. With a short limit
/// (previously 45 s) a slow cold yt-dlp (first song, antivirus) cut the
/// track off when the head ran out (~1:04) even though the URL was about to arrive.
const UPGRADE_TIMEOUT: Duration = Duration::from_secs(180);
/// Upper bound for honouring a stream's `available_at` (YouTube asks for ~6 s).
const MAX_AVAILABILITY_WAIT: u64 = 30;
const DEFAULT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36";

/// Failures the engine tells apart: a 403 means an expired URL (re-resolve).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    Expired,
    Network,
}

impl SourceError {
    /// Recovers the type from the `io::Error` returned by `read`.
    pub fn from_io(e: &io::Error) -> Self {
        if e.kind() == io::ErrorKind::PermissionDenied {
            Self::Expired
        } else {
            Self::Network
        }
    }

    fn into_io(self) -> io::Error {
        match self {
            Self::Expired => {
                io::Error::new(io::ErrorKind::PermissionDenied, "stream expired (403)")
            }
            Self::Network => io::Error::other("network error downloading the audio"),
        }
    }
}

struct State {
    chunks: BTreeMap<u64, Arc<Vec<u8>>>,
    want: u64,
    error: Option<SourceError>,
    closed: bool,
}

struct Shared {
    state: Mutex<State>,
    cv: Condvar,
}

pub struct HttpRangeSource {
    shared: Arc<Shared>,
    pos: u64,
    len: u64,
}

/// Read-only view to measure the buffer from another thread.
#[derive(Clone)]
pub struct SourceProbe {
    shared: Arc<Shared>,
}

impl SourceProbe {
    /// Contiguous bytes already downloaded from the block being read.
    pub fn buffered_ahead(&self) -> u64 {
        let st = self.shared.state.lock().unwrap();
        let mut bytes = 0;
        let mut c = st.want;
        while let Some(chunk) = st.chunks.get(&c) {
            bytes += chunk.len() as u64;
            c += 1;
        }
        bytes
    }
}

impl HttpRangeSource {
    /// Opens the source. If the length is unknown, it requests it (blocking).
    /// Nothing is requested from `url` before `available_at` (see `wait_available`).
    /// With `upgrade`, `url` only serves the start (fast start) and the rest
    /// is downloaded with the full URL once it arrives.
    pub fn open(
        client: Client,
        url: &str,
        user_agent: Option<&str>,
        len: Option<u64>,
        available_at: Option<u64>,
        upgrade: Option<Upgrade>,
    ) -> Result<Self, SourceError> {
        let ua = user_agent.unwrap_or(DEFAULT_UA).to_owned();
        let len = match len {
            Some(l) => l,
            None => {
                wait_available(available_at, || false);
                probe_len(&client, url, &ua)?
            }
        };
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                chunks: BTreeMap::new(),
                want: 0,
                error: None,
                closed: false,
            }),
            cv: Condvar::new(),
        });
        let worker = Arc::clone(&shared);
        let url = url.to_owned();
        thread::Builder::new()
            .name("audio-download".into())
            .spawn(move || download_loop(worker, client, url, ua, len, available_at, upgrade))
            .map_err(|_| SourceError::Network)?;
        Ok(Self {
            shared,
            pos: 0,
            len,
        })
    }

    pub fn probe(&self) -> SourceProbe {
        SourceProbe {
            shared: Arc::clone(&self.shared),
        }
    }
}

fn probe_len(client: &Client, url: &str, ua: &str) -> Result<u64, SourceError> {
    let resp = client
        .get(url)
        .header(RANGE, "bytes=0-0")
        .header(USER_AGENT, ua)
        .send()
        .map_err(|_| SourceError::Network)?;
    if resp.status() == StatusCode::FORBIDDEN {
        return Err(SourceError::Expired);
    }
    resp.headers()
        .get(CONTENT_RANGE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cr| cr.rsplit('/').next())
        .and_then(|t| t.parse().ok())
        .ok_or(SourceError::Network)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Some YouTube clients' URLs answer 403 for a few seconds after being issued;
/// yt-dlp reports when they start working (`available_at`, Unix seconds). Waits
/// until then (at most `MAX_AVAILABILITY_WAIT`). `false` if `cancelled` first.
fn wait_available(available_at: Option<u64>, cancelled: impl Fn() -> bool) -> bool {
    let now = unix_now();
    let Some(wait) = available_at.filter(|&at| at > now).map(|at| at - now) else {
        return true;
    };
    tracing::debug!(seconds = wait, "waiting until the stream URL is available");
    let deadline = Instant::now() + Duration::from_secs(wait.min(MAX_AVAILABILITY_WAIT));
    while Instant::now() < deadline {
        if cancelled() {
            return false;
        }
        thread::sleep(Duration::from_millis(100));
    }
    true
}

/// Waits for the full URL while still watching for the source being closed.
/// Returns its URL, user agent and `available_at`.
fn wait_upgrade(sh: &Shared, up: &Upgrade, len: u64) -> Option<(String, String, Option<u64>)> {
    let deadline = std::time::Instant::now() + UPGRADE_TIMEOUT;
    while std::time::Instant::now() < deadline {
        if sh.state.lock().unwrap().closed {
            return None;
        }
        match up.wait(Duration::from_millis(200)) {
            // Different size = different file: bytes from two versions are not mixed.
            Some(Ok(full)) if full.content_length.is_some_and(|l| l != len) => {
                tracing::warn!("fast start: the full URL is for a different file");
                return None;
            }
            Some(Ok(full)) => {
                let ua = full.user_agent.unwrap_or_else(|| DEFAULT_UA.to_owned());
                return Some((full.url, ua, full.available_at));
            }
            Some(Err(_)) => return None,
            None => {}
        }
    }
    None
}

fn download_loop(
    sh: Arc<Shared>,
    client: Client,
    mut url: String,
    mut ua: String,
    len: u64,
    mut available_at: Option<u64>,
    mut upgrade: Option<Upgrade>,
) {
    let n_chunks = len.div_ceil(CHUNK);
    loop {
        let target = {
            let mut st = sh.state.lock().unwrap();
            loop {
                if st.closed || st.error.is_some() {
                    return;
                }
                let want = st.want;
                let lo = want.saturating_sub(KEEP_BEHIND);
                let hi = want + AHEAD;
                st.chunks.retain(|&c, _| c >= lo && c <= hi);
                if let Some(c) = (want..hi.min(n_chunks)).find(|c| !st.chunks.contains_key(c)) {
                    break c;
                }
                st = sh
                    .cv
                    .wait_timeout(st, Duration::from_millis(500))
                    .unwrap()
                    .0;
            }
        };

        let start = target * CHUNK;
        let end = (start + CHUNK).min(len) - 1;
        // Fast start: beyond the head the full URL is needed.
        if end >= HEAD_LIMIT
            && let Some(up) = upgrade.take()
        {
            match wait_upgrade(&sh, &up, len) {
                Some((full, full_ua, full_available_at)) => {
                    tracing::debug!("fast start: switching to the full URL");
                    url = full;
                    ua = full_ua;
                    // It usually arrives a moment after yt-dlp issued it, and the
                    // download runs ahead of playback: requesting it right away got
                    // a 403 that only surfaced when playback reached ~1:04.
                    available_at = full_available_at;
                }
                None => {
                    // No full URL: network failure, the engine retries. Not as expired:
                    // that would evict from the cache the full URL yt-dlp may have left.
                    let mut st = sh.state.lock().unwrap();
                    st.error = Some(SourceError::Network);
                    sh.cv.notify_all();
                    return;
                }
            }
        }
        if !wait_available(available_at.take(), || sh.state.lock().unwrap().closed) {
            return;
        }
        let result = fetch(&client, &url, &ua, start, end);
        let mut st = sh.state.lock().unwrap();
        match result {
            Ok(bytes) => {
                st.chunks.insert(target, Arc::new(bytes));
            }
            Err(e) => {
                tracing::warn!(chunk = target, error = ?e, "failed to download audio");
                st.error = Some(e);
            }
        }
        sh.cv.notify_all();
    }
}

fn fetch(
    client: &Client,
    url: &str,
    ua: &str,
    start: u64,
    end: u64,
) -> Result<Vec<u8>, SourceError> {
    for attempt in 0..RETRIES {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(250 << (attempt - 1)));
        }
        let resp = client
            .get(url)
            .header(RANGE, format!("bytes={start}-{end}"))
            .header(USER_AGENT, ua)
            .send();
        match resp {
            Ok(r) if r.status() == StatusCode::FORBIDDEN => return Err(SourceError::Expired),
            Ok(r) if r.status() == StatusCode::PARTIAL_CONTENT => match r.bytes() {
                Ok(b) if b.len() as u64 == end - start + 1 => return Ok(b.to_vec()),
                _ => continue,
            },
            _ => continue,
        }
    }
    Err(SourceError::Network)
}

impl Read for HttpRangeSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let idx = self.pos / CHUNK;
        let chunk = {
            let mut st = self.shared.state.lock().unwrap();
            if st.want != idx {
                st.want = idx;
                self.shared.cv.notify_all();
            }
            loop {
                if let Some(c) = st.chunks.get(&idx) {
                    break Arc::clone(c);
                }
                if let Some(e) = st.error {
                    return Err(e.into_io());
                }
                if st.closed {
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "source closed"));
                }
                st = self.shared.cv.wait(st).unwrap();
            }
        };
        let off = (self.pos - idx * CHUNK) as usize;
        let n = buf.len().min(chunk.len() - off);
        buf[..n].copy_from_slice(&chunk[off..off + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for HttpRangeSource {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let new = match to {
            SeekFrom::Start(p) => p as i128,
            SeekFrom::End(d) => self.len as i128 + d as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if new < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before the start",
            ));
        }
        self.pos = new as u64;
        Ok(self.pos)
    }
}

impl symphonia::core::io::MediaSource for HttpRangeSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

impl Drop for HttpRangeSource {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().closed = true;
        self.shared.cv.notify_all();
    }
}

/// `MediaSource` over a file on disk (local music). `MediaSourceStream` already
/// buffers the reads, so there is nothing to download or keep around here.
pub struct FileSource {
    file: std::fs::File,
    len: u64,
}

impl FileSource {
    pub fn open(path: &std::path::Path) -> io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}

impl Read for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for FileSource {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.file.seek(to)
    }
}

impl symphonia::core::io::MediaSource for FileSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

#[cfg(test)]
pub(crate) mod test_server {
    //! Minimal HTTP server with Range support for tests.
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    /// Serves `data`. With `fail_after = Some(n)` it answers 403 from request n on.
    pub fn serve(data: Arc<Vec<u8>>, fail_after: Option<usize>) -> String {
        serve_with(data, move |i| fail_after.is_some_and(|n| i >= n))
    }

    /// Serves `data` but answers 403 until `from`, like a YouTube URL that is
    /// not available yet.
    pub fn serve_from(data: Arc<Vec<u8>>, from: Instant) -> String {
        serve_with(data, move |_| Instant::now() < from)
    }

    /// `forbidden(i)`: whether request number `i` gets a 403.
    fn serve_with(
        data: Arc<Vec<u8>>,
        forbidden: impl Fn(usize) -> bool + Send + Sync + 'static,
    ) -> String {
        let forbidden = Arc::new(forbidden);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for (i, stream) in listener.incoming().enumerate() {
                let Ok(mut stream) = stream else { continue };
                let data = Arc::clone(&data);
                let forbidden = Arc::clone(&forbidden);
                thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut range = None;
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                            break;
                        }
                        if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                            let (a, b) = v.trim().split_once('-').unwrap();
                            range =
                                Some((a.parse::<usize>().unwrap(), b.parse::<usize>().unwrap()));
                        }
                    }
                    if forbidden(i) {
                        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                        return;
                    }
                    let (a, b) = range.unwrap_or((0, data.len() - 1));
                    let b = b.min(data.len() - 1);
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {a}-{b}/{}\r\nConnection: close\r\n\r\n",
                        b - a + 1,
                        data.len()
                    );
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(&data[a..=b]);
                });
            }
        });
        format!("http://{addr}/audio.m4a")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 31 % 251) as u8).collect()
    }

    fn client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap()
    }

    #[test]
    fn reads_in_order_and_after_seek() {
        let data = Arc::new(pattern(5 * CHUNK as usize + 12345));
        let url = test_server::serve(Arc::clone(&data), None);
        let mut src = HttpRangeSource::open(client(), &url, None, None, None, None).unwrap();
        assert_eq!(src.len, data.len() as u64);

        let mut buf = vec![0u8; 3000];
        src.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], &data[..3000]);

        let at = 4 * CHUNK as usize - 1000;
        src.seek(SeekFrom::Start(at as u64)).unwrap();
        src.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], &data[at..at + 3000]);

        src.seek(SeekFrom::End(-10)).unwrap();
        let mut tail = Vec::new();
        src.read_to_end(&mut tail).unwrap();
        assert_eq!(&tail[..], &data[data.len() - 10..]);
    }

    #[test]
    fn file_source_reads_and_seeks() {
        use symphonia::core::io::MediaSource;
        let data = pattern(70_000);
        let dir = crate::local::fixtures::temp_dir("file-source");
        let path = dir.join("audio.bin");
        std::fs::write(&path, &data).unwrap();

        let mut src = FileSource::open(&path).unwrap();
        assert_eq!(src.len(), data.len() as u64);
        assert_eq!(src.byte_len(), Some(data.len() as u64));
        assert!(src.is_seekable());

        let mut buf = vec![0u8; 1000];
        src.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], &data[..1000]);

        src.seek(SeekFrom::Start(65_000)).unwrap();
        src.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], &data[65_000..66_000]);

        src.seek(SeekFrom::Current(-2000)).unwrap();
        src.read_exact(&mut buf).unwrap();
        assert_eq!(&buf[..], &data[64_000..65_000]);

        assert_eq!(
            src.seek(SeekFrom::End(-10)).unwrap(),
            data.len() as u64 - 10
        );
        let mut tail = Vec::new();
        src.read_to_end(&mut tail).unwrap();
        assert_eq!(&tail[..], &data[data.len() - 10..]);
        assert_eq!(
            src.read(&mut buf).unwrap(),
            0,
            "past the end there is nothing"
        );

        assert!(FileSource::open(&dir.join("missing.mp3")).is_err());
    }

    #[test]
    fn measures_what_is_buffered_ahead() {
        let data = Arc::new(pattern(3 * CHUNK as usize));
        let url = test_server::serve(Arc::clone(&data), None);
        let mut src =
            HttpRangeSource::open(client(), &url, None, Some(data.len() as u64), None, None)
                .unwrap();
        let probe = src.probe();
        let mut b = [0u8; 10];
        src.read_exact(&mut b).unwrap();
        for _ in 0..100 {
            if probe.buffered_ahead() == data.len() as u64 {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("the buffer never completed: {}", probe.buffered_ahead());
    }

    #[test]
    fn a_403_is_reported_as_expired() {
        let data = Arc::new(pattern(4 * CHUNK as usize));
        // Request 0 = length, 1 = first block; then 403.
        let url = test_server::serve(Arc::clone(&data), Some(2));
        let mut src = HttpRangeSource::open(client(), &url, None, None, None, None).unwrap();
        let mut buf = vec![0u8; CHUNK as usize];
        src.read_exact(&mut buf).unwrap();
        src.seek(SeekFrom::Start(3 * CHUNK)).unwrap();
        let err = src.read(&mut buf).unwrap_err();
        assert_eq!(SourceError::from_io(&err), SourceError::Expired);
    }

    #[test]
    fn fast_start_switches_to_the_full_url() {
        let data = Arc::new(pattern(3 * CHUNK as usize + 777));
        // The "fast" URL only serves the first MiB (the blocks that fit in it).
        let head = test_server::serve(Arc::clone(&data), Some((HEAD_LIMIT / CHUNK) as usize));
        let full = test_server::serve(Arc::clone(&data), None);
        let up = Upgrade::default();
        let mut src = HttpRangeSource::open(
            client(),
            &head,
            None,
            Some(data.len() as u64),
            None,
            Some(up.clone()),
        )
        .unwrap();
        let filler = up.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            filler.fill(Ok(crate::audio::resolver::ResolvedStream {
                track_id: crate::domain::TrackId("t".into()),
                url: full,
                itag: 140,
                content_length: None,
                duration_ms: None,
                expires_at: u64::MAX,
                available_at: None,
                user_agent: None,
                upgrade: None,
            }));
        });
        let mut all = Vec::new();
        src.read_to_end(&mut all).unwrap();
        assert_eq!(all.len(), data.len());
        assert!(all == *data, "content differs after the URL switch");
    }

    #[test]
    fn waits_until_the_url_is_available() {
        let data = Arc::new(pattern(2 * CHUNK as usize));
        let url = test_server::serve_from(
            Arc::clone(&data),
            Instant::now() + Duration::from_millis(1500),
        );
        let mut src =
            HttpRangeSource::open(client(), &url, None, None, Some(unix_now() + 2), None).unwrap();
        let mut all = Vec::new();
        src.read_to_end(&mut all).unwrap();
        assert!(all == *data);
    }

    /// The bug behind the stall at ~1:04 on the first song of a list: the full URL
    /// arrives a moment after yt-dlp issued it and the download, which runs ahead
    /// of playback, requested it before YouTube allowed it (403).
    #[test]
    fn fast_start_waits_until_the_full_url_is_available() {
        let data = Arc::new(pattern(HEAD_LIMIT as usize + 2 * CHUNK as usize));
        let head = test_server::serve(Arc::clone(&data), Some((HEAD_LIMIT / CHUNK) as usize));
        let full = test_server::serve_from(
            Arc::clone(&data),
            Instant::now() + Duration::from_millis(1500),
        );
        let up = Upgrade::default();
        let mut src = HttpRangeSource::open(
            client(),
            &head,
            None,
            Some(data.len() as u64),
            None,
            Some(up.clone()),
        )
        .unwrap();
        let filler = up.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            filler.fill(Ok(crate::audio::resolver::ResolvedStream {
                track_id: crate::domain::TrackId("t".into()),
                url: full,
                itag: 140,
                content_length: None,
                duration_ms: None,
                expires_at: u64::MAX,
                available_at: Some(unix_now() + 2),
                user_agent: None,
                upgrade: None,
            }));
        });
        let mut all = Vec::new();
        src.read_to_end(&mut all).unwrap();
        assert!(all == *data, "content differs after the URL switch");
    }

    #[test]
    fn missing_full_url_is_reported_as_network_failure() {
        let data = Arc::new(pattern(2 * HEAD_LIMIT as usize));
        let head = test_server::serve(Arc::clone(&data), Some((HEAD_LIMIT / CHUNK) as usize));
        let up = Upgrade::default();
        up.fill(Err(crate::error::AppError::internal("yt-dlp failed")));
        let mut src = HttpRangeSource::open(
            client(),
            &head,
            None,
            Some(data.len() as u64),
            None,
            Some(up),
        )
        .unwrap();
        // The start is audible; beyond it there is no URL to continue with.
        let mut buf = vec![0u8; HEAD_LIMIT as usize];
        src.read_exact(&mut buf).unwrap();
        let err = src.read(&mut buf).unwrap_err();
        assert_eq!(SourceError::from_io(&err), SourceError::Network);
    }
}
