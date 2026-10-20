//! `MediaSource` sobre HTTP con peticiones Range (FR-002-01/03, NFR-002-03).
//!
//! Un hilo descargador mantiene bloques de 256 KiB alrededor de la posición de
//! lectura (32 por delante, 8 por detrás: ≤ 10 MiB). `read` bloquea hasta que
//! llega el bloque necesario; `seek` solo mueve la posición.

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{CONTENT_RANGE, RANGE, USER_AGENT};

use super::fast::HEAD_LIMIT;
use super::resolver::Upgrade;

/// 256 KiB: el primer bloque llega rápido y suena antes (008).
pub const CHUNK: u64 = 1 << 18;
const AHEAD: u64 = 32;
const KEEP_BEHIND: u64 = 8;
const RETRIES: u32 = 3;
/// Lo más que se espera a la URL completa en un arranque rápido.
const UPGRADE_TIMEOUT: Duration = Duration::from_secs(45);
const DEFAULT_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Safari/537.36";

/// Fallos que el motor distingue: un 403 significa URL caducada (re-resolver).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    Expired,
    Network,
}

impl SourceError {
    /// Recupera el tipo a partir del `io::Error` que devuelve `read`.
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
                io::Error::new(io::ErrorKind::PermissionDenied, "stream caducado (403)")
            }
            Self::Network => io::Error::other("error de red descargando el audio"),
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

/// Vista de solo lectura para medir el búfer desde otro hilo.
#[derive(Clone)]
pub struct SourceProbe {
    shared: Arc<Shared>,
}

impl SourceProbe {
    /// Bytes contiguos ya descargados desde el bloque que se está leyendo.
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
    /// Abre la fuente. Si no se conoce la longitud, la pide (bloqueante).
    /// Con `upgrade`, `url` solo sirve el principio (arranque rápido) y el resto
    /// se descarga con la URL completa cuando llegue.
    pub fn open(
        client: Client,
        url: &str,
        user_agent: Option<&str>,
        len: Option<u64>,
        upgrade: Option<Upgrade>,
    ) -> Result<Self, SourceError> {
        let ua = user_agent.unwrap_or(DEFAULT_UA).to_owned();
        let len = match len {
            Some(l) => l,
            None => probe_len(&client, url, &ua)?,
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
            .spawn(move || download_loop(worker, client, url, ua, len, upgrade))
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

/// Espera la URL completa sin dejar de atender al cierre de la fuente.
fn wait_upgrade(sh: &Shared, up: &Upgrade, len: u64) -> Option<(String, String)> {
    let deadline = std::time::Instant::now() + UPGRADE_TIMEOUT;
    while std::time::Instant::now() < deadline {
        if sh.state.lock().unwrap().closed {
            return None;
        }
        match up.wait(Duration::from_millis(200)) {
            // Otro tamaño = otro archivo: no se mezclan bytes de dos versiones.
            Some(Ok(full)) if full.content_length.is_some_and(|l| l != len) => {
                tracing::warn!("arranque rápido: la URL completa es de otro archivo");
                return None;
            }
            Some(Ok(full)) => {
                let ua = full.user_agent.unwrap_or_else(|| DEFAULT_UA.to_owned());
                return Some((full.url, ua));
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
        // Arranque rápido: más allá del principio hace falta la URL completa.
        if end >= HEAD_LIMIT
            && let Some(up) = upgrade.take()
        {
            match wait_upgrade(&sh, &up, len) {
                Some((full, full_ua)) => {
                    tracing::debug!("arranque rápido: se pasa a la URL completa");
                    url = full;
                    ua = full_ua;
                }
                None => {
                    // Sin URL completa: el motor lo trata como caducada y re-resuelve.
                    let mut st = sh.state.lock().unwrap();
                    st.error = Some(SourceError::Expired);
                    sh.cv.notify_all();
                    return;
                }
            }
        }
        let result = fetch(&client, &url, &ua, start, end);
        let mut st = sh.state.lock().unwrap();
        match result {
            Ok(bytes) => {
                st.chunks.insert(target, Arc::new(bytes));
            }
            Err(e) => {
                tracing::warn!(chunk = target, error = ?e, "fallo descargando audio");
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
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "fuente cerrada"));
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
                "seek antes del inicio",
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

#[cfg(test)]
pub(crate) mod test_server {
    //! Servidor HTTP mínimo con soporte Range para tests.
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;

    /// Sirve `data`. Con `fail_after = Some(n)` responde 403 desde la petición n.
    pub fn serve(data: Arc<Vec<u8>>, fail_after: Option<usize>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            for (i, stream) in listener.incoming().enumerate() {
                let Ok(mut stream) = stream else { continue };
                let data = Arc::clone(&data);
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
                    if fail_after.is_some_and(|n| i >= n) {
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
    fn lee_en_orden_y_tras_seek() {
        let data = Arc::new(pattern(5 * CHUNK as usize + 12345));
        let url = test_server::serve(Arc::clone(&data), None);
        let mut src = HttpRangeSource::open(client(), &url, None, None, None).unwrap();
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
    fn mide_lo_almacenado_por_delante() {
        let data = Arc::new(pattern(3 * CHUNK as usize));
        let url = test_server::serve(Arc::clone(&data), None);
        let mut src =
            HttpRangeSource::open(client(), &url, None, Some(data.len() as u64), None).unwrap();
        let probe = src.probe();
        let mut b = [0u8; 10];
        src.read_exact(&mut b).unwrap();
        for _ in 0..100 {
            if probe.buffered_ahead() == data.len() as u64 {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "el búfer no llegó a completarse: {}",
            probe.buffered_ahead()
        );
    }

    #[test]
    fn un_403_se_reporta_como_caducado() {
        let data = Arc::new(pattern(4 * CHUNK as usize));
        // Petición 0 = longitud, 1 = primer bloque; después 403.
        let url = test_server::serve(Arc::clone(&data), Some(2));
        let mut src = HttpRangeSource::open(client(), &url, None, None, None).unwrap();
        let mut buf = vec![0u8; CHUNK as usize];
        src.read_exact(&mut buf).unwrap();
        src.seek(SeekFrom::Start(3 * CHUNK)).unwrap();
        let err = src.read(&mut buf).unwrap_err();
        assert_eq!(SourceError::from_io(&err), SourceError::Expired);
    }

    #[test]
    fn arranque_rapido_pasa_a_la_url_completa() {
        let data = Arc::new(pattern(3 * CHUNK as usize + 777));
        // La URL "rápida" solo sirve el primer MiB (los bloques que caben en él).
        let head = test_server::serve(Arc::clone(&data), Some((HEAD_LIMIT / CHUNK) as usize));
        let full = test_server::serve(Arc::clone(&data), None);
        let up = Upgrade::default();
        let mut src = HttpRangeSource::open(
            client(),
            &head,
            None,
            Some(data.len() as u64),
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
                user_agent: None,
                upgrade: None,
            }));
        });
        let mut all = Vec::new();
        src.read_to_end(&mut all).unwrap();
        assert_eq!(all.len(), data.len());
        assert!(all == *data, "contenido distinto tras el cambio de URL");
    }

    #[test]
    fn sin_url_completa_se_reporta_como_caducado() {
        let data = Arc::new(pattern(2 * HEAD_LIMIT as usize));
        let head = test_server::serve(Arc::clone(&data), Some((HEAD_LIMIT / CHUNK) as usize));
        let up = Upgrade::default();
        up.fill(Err(crate::error::AppError::internal("yt-dlp falló")));
        let mut src =
            HttpRangeSource::open(client(), &head, None, Some(data.len() as u64), Some(up))
                .unwrap();
        // El principio se oye; más allá no hay URL con la que seguir.
        let mut buf = vec![0u8; HEAD_LIMIT as usize];
        src.read_exact(&mut buf).unwrap();
        let err = src.read(&mut buf).unwrap_err();
        assert_eq!(SourceError::from_io(&err), SourceError::Expired);
    }
}
