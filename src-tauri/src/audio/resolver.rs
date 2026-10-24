//! Resolución de `TrackId` → URL de audio (specs/002, principio III).
//!
//! `YtDlpResolver` invoca el sidecar yt-dlp; `CachedResolver` evita repetirlo
//! mientras la URL siga vigente (yt-dlp tarda ~3,5 s, ver spike/FINDINGS.md).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::domain::TrackId;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::storage::Db;

/// Margen antes de `expire` a partir del cual una URL ya no se reutiliza.
const EXPIRY_MARGIN: Duration = Duration::from_secs(10 * 60);
const YT_DLP_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FormatPreference {
    /// itag 140
    Aac128,
}

impl FormatPreference {
    pub fn itag(self) -> u32 {
        match self {
            Self::Aac128 => 140,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolvedStream {
    pub track_id: TrackId,
    pub url: String,
    pub itag: u32,
    pub content_length: Option<u64>,
    pub duration_ms: Option<u32>,
    /// Segundos Unix (parámetro `expire` de la URL).
    pub expires_at: u64,
    pub user_agent: Option<String>,
    /// Arranque rápido: esta URL solo sirve el principio y la completa llega
    /// aquí más tarde (ver `fast.rs`). Nunca se guarda en caché.
    #[serde(skip)]
    pub upgrade: Option<Upgrade>,
}

/// Hueco donde llega la resolución completa de una pista que ya está sonando
/// con la URL del arranque rápido.
#[derive(Clone, Default)]
pub struct Upgrade(Arc<(Mutex<Option<AppResult<ResolvedStream>>>, std::sync::Condvar)>);

impl Upgrade {
    pub fn fill(&self, r: AppResult<ResolvedStream>) {
        *self.0.0.lock().unwrap() = Some(r);
        self.0.1.notify_all();
    }

    /// Espera hasta `timeout`; `None` si aún no ha llegado.
    pub fn wait(&self, timeout: Duration) -> Option<AppResult<ResolvedStream>> {
        let guard = self.0.0.lock().unwrap();
        let (guard, _) = self
            .0
            .1
            .wait_timeout_while(guard, timeout, |r| r.is_none())
            .unwrap();
        guard.clone()
    }
}

impl std::fmt::Debug for Upgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Upgrade")
    }
}

impl PartialEq for Upgrade {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl ResolvedStream {
    fn still_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        now + EXPIRY_MARGIN < Duration::from_secs(self.expires_at)
    }
}

/// Bloqueante: se llama desde `spawn_blocking` o hilos propios.
pub trait StreamResolver: Send + Sync {
    fn resolve(&self, id: &TrackId, pref: FormatPreference) -> AppResult<ResolvedStream>;
    /// Olvida lo que se tenga en caché de `id` (URL caducada o rechazada).
    fn invalidate(&self, _id: &TrackId) {}
    /// Resolución ya disponible sin esperar (caché válida).
    fn cached(&self, _id: &TrackId, _pref: FormatPreference) -> Option<ResolvedStream> {
        None
    }
}

// ---------------------------------------------------------------- yt-dlp

#[derive(Clone, Debug)]
pub struct ToolPaths {
    pub yt_dlp: PathBuf,
    pub deno: PathBuf,
}

impl ToolPaths {
    /// Busca en `XP_MUSIC_TOOLS`, los recursos del paquete, junto al ejecutable
    /// o `tools/` del repositorio en desarrollo.
    pub fn locate() -> Option<Self> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(dir) = std::env::var_os("XP_MUSIC_TOOLS") {
            candidates.push(dir.into());
        }
        if let Some(dir) = crate::tools::RESOURCE_DIR.get() {
            candidates.push(dir.join("tools"));
        }
        if let Some(dir) = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
        {
            candidates.push(dir.join("tools"));
            candidates.push(dir);
        }
        #[cfg(debug_assertions)]
        candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools"));

        let deno = candidates
            .iter()
            .map(|d| d.join(format!("deno{}", std::env::consts::EXE_SUFFIX)))
            .find(|p| p.is_file())?;
        // La copia que actualiza la propia app (007) tiene prioridad; si no, la incluida.
        // Se prefiere la versión "onedir" (`yt-dlp/yt-dlp.exe`): no se descomprime en cada
        // uso y arranca ~0,7 s antes que el ejecutable de un solo archivo.
        let yt_dlp = crate::tools::updated_yt_dlp()
            .filter(|p| p.is_file())
            .or_else(|| {
                candidates.iter().find_map(|dir| {
                    [
                        dir.join("yt-dlp").join(crate::tools::YT_DLP_EXE),
                        dir.join(crate::tools::YT_DLP_EXE),
                    ]
                    .into_iter()
                    .find(|p| p.is_file())
                })
            })?;
        Some(Self { yt_dlp, deno })
    }
}

pub struct YtDlpResolver {
    tools: ToolPaths,
    /// Da las cookies de la sesión (formato Netscape), solo para el reintento (NFR-002-05).
    cookies: Option<CookieProvider>,
    failures: std::sync::atomic::AtomicU32,
}

pub type CookieProvider = Box<dyn Fn() -> Option<String> + Send + Sync>;

#[derive(Deserialize)]
struct YtDlpInfo {
    url: String,
    format_id: String,
    filesize: Option<u64>,
    duration: Option<f64>,
    #[serde(default)]
    http_headers: HashMap<String, String>,
}

impl YtDlpResolver {
    pub fn new(tools: ToolPaths, cookies: Option<CookieProvider>) -> Self {
        Self {
            tools,
            cookies,
            failures: Default::default(),
        }
    }

    fn run(
        &self,
        id: &TrackId,
        pref: FormatPreference,
        cookies: Option<&Path>,
    ) -> AppResult<ResolvedStream> {
        // Se vuelven a buscar en cada uso: tras una actualización se usa la copia nueva.
        let tools = ToolPaths::locate().unwrap_or_else(|| self.tools.clone());
        let mut cmd = Command::new(&tools.yt_dlp);
        cmd.args([
            "-f",
            &pref.itag().to_string(),
            "-j",
            "--no-playlist",
            "--no-warnings",
            // Solo hace falta el formato progresivo: sin manifiestos HLS/DASH (~0,3 s menos).
            "--extractor-args",
            "youtube:skip=hls,dash,translated_subs",
        ])
        .arg("--js-runtimes")
        .arg(format!("deno:{}", tools.deno.display()));
        if let Some(c) = cookies {
            cmd.arg("--cookies").arg(c);
        }
        cmd.arg(format!("https://music.youtube.com/watch?v={}", id.0));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let out = run_with_timeout(cmd, YT_DLP_TIMEOUT)?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let last = stderr
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("sin salida");
            tracing::warn!(track = %id.0, error = %last, "yt-dlp falló");
            return Err(AppError::new(
                ErrorCode::ResolveFailed,
                "No se pudo obtener el audio de esta canción.",
            ));
        }
        parse_info(id, &out.stdout)
    }
}

fn parse_info(id: &TrackId, stdout: &[u8]) -> AppResult<ResolvedStream> {
    let info: YtDlpInfo = serde_json::from_slice(stdout).map_err(|e| {
        tracing::warn!(error = %e, "JSON de yt-dlp no válido");
        AppError::new(
            ErrorCode::ResolveFailed,
            "Respuesta inesperada al obtener el audio.",
        )
    })?;
    let expires_at = info
        .url
        .split(['?', '&'])
        .find_map(|kv| kv.strip_prefix("expire="))
        .and_then(|v| v.parse().ok())
        // Sin `expire` se asume la vida típica de 6 h.
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + 6 * 3600
        });
    Ok(ResolvedStream {
        track_id: id.clone(),
        itag: info.format_id.parse().unwrap_or(0),
        content_length: info.filesize,
        duration_ms: info.duration.map(|d| (d * 1000.0) as u32),
        expires_at,
        user_agent: info.http_headers.get("User-Agent").cloned(),
        url: info.url,
        upgrade: None,
    })
}

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> AppResult<std::process::Output> {
    use std::process::Stdio;
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            tracing::error!(error = %e, "no se pudo lanzar yt-dlp");
            AppError::new(
                ErrorCode::ResolveFailed,
                "Falta la herramienta de extracción de audio (yt-dlp).",
            )
        })?;
    // Leer stdout/stderr en hilos para que el proceso no se bloquee con la tubería llena.
    let mut out = child.stdout.take().expect("stdout");
    let mut err = child.stderr.take().expect("stderr");
    let out_h = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = std::io::Read::read_to_end(&mut out, &mut b);
        b
    });
    let err_h = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = std::io::Read::read_to_end(&mut err, &mut b);
        b
    });
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|e| AppError::internal(e.to_string()))?
        {
            break s;
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            tracing::warn!("yt-dlp superó el tiempo límite");
            return Err(AppError::new(
                ErrorCode::ResolveFailed,
                "La extracción del audio tardó demasiado.",
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    Ok(std::process::Output {
        status,
        stdout: out_h.join().unwrap_or_default(),
        stderr: err_h.join().unwrap_or_default(),
    })
}

/// Fallos seguidos a partir de los cuales se sospecha de un yt-dlp desfasado (FR-007-09).
const FAILS_BEFORE_UPDATE: u32 = 2;

impl StreamResolver for YtDlpResolver {
    fn resolve(&self, id: &TrackId, pref: FormatPreference) -> AppResult<ResolvedStream> {
        use std::sync::atomic::Ordering;
        let res = self.resolve_once(id, pref);
        if res.is_ok() {
            self.failures.store(0, Ordering::Relaxed);
            return res;
        }
        if self.failures.fetch_add(1, Ordering::Relaxed) + 1 < FAILS_BEFORE_UPDATE {
            return res;
        }
        // YouTube cambia a menudo y una versión nueva de yt-dlp suele arreglarlo.
        match crate::tools::update_yt_dlp(true) {
            Ok(Some(v)) => {
                tracing::info!(version = %v, track = %id.0, "reintentando con yt-dlp actualizado");
                self.failures.store(0, Ordering::Relaxed);
                self.resolve_once(id, pref)
            }
            _ => res,
        }
    }
}

impl YtDlpResolver {
    fn resolve_once(&self, id: &TrackId, pref: FormatPreference) -> AppResult<ResolvedStream> {
        match self.run(id, pref, None) {
            Ok(s) => Ok(s),
            Err(first) => {
                let Some(netscape) = self.cookies.as_ref().and_then(|f| f()) else {
                    return Err(first);
                };
                // Fichero temporal (yt-dlp lo reescribe); se borra al terminar.
                let tmp = std::env::temp_dir().join(format!(
                    "xp-music-cookies-{}-{}.txt",
                    std::process::id(),
                    rand::random::<u32>()
                ));
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                {
                    use std::io::Write;
                    let mut file = options
                        .open(&tmp)
                        .map_err(|e| AppError::internal(e.to_string()))?;
                    if let Err(e) = file.write_all(netscape.as_bytes()) {
                        let _ = std::fs::remove_file(&tmp);
                        return Err(AppError::internal(e.to_string()));
                    }
                }
                tracing::info!(track = %id.0, "reintentando yt-dlp con sesión");
                let res = self.run(id, pref, Some(&tmp));
                let _ = std::fs::remove_file(&tmp);
                res
            }
        }
    }
}

/// Sustituto cuando no se encuentran yt-dlp/deno: cada intento da un error claro.
pub struct MissingTools;

impl StreamResolver for MissingTools {
    fn resolve(&self, _: &TrackId, _: FormatPreference) -> AppResult<ResolvedStream> {
        Err(AppError::new(
            ErrorCode::ResolveFailed,
            "No se encuentran yt-dlp y deno (carpeta tools/).",
        ))
    }
}

// ---------------------------------------------------------------- caché

pub struct CachedResolver<R> {
    inner: R,
    db: Arc<Db>,
    mem: Mutex<HashMap<(String, u32), ResolvedStream>>,
    /// Resoluciones en marcha: quien pida la misma pista espera a esa en vez de
    /// lanzar otro yt-dlp (p. ej. doble clic mientras se resolvía al pasar el ratón).
    inflight: Mutex<HashMap<(String, u32), Arc<Flight>>>,
}

#[derive(Default)]
struct Flight {
    result: Mutex<Option<AppResult<ResolvedStream>>>,
    done: std::sync::Condvar,
}

impl<R: StreamResolver> CachedResolver<R> {
    pub fn new(inner: R, db: Arc<Db>) -> Self {
        Self {
            inner,
            db,
            mem: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
        }
    }

    fn lookup(&self, key: &(String, u32)) -> Option<ResolvedStream> {
        if let Some(s) = self
            .mem
            .lock()
            .unwrap()
            .get(key)
            .filter(|s| s.still_valid())
        {
            return Some(s.clone());
        }
        let json: Option<String> = self
            .db
            .conn()
            .query_row(
                "SELECT json FROM resolve_cache WHERE track_id = ?1 AND itag = ?2",
                (&key.0, key.1),
                |r| r.get(0),
            )
            .ok();
        let s: ResolvedStream = serde_json::from_str(&json?).ok()?;
        s.still_valid().then(|| {
            self.mem.lock().unwrap().insert(key.clone(), s.clone());
            s
        })
    }

    fn store(&self, key: (String, u32), s: &ResolvedStream) {
        if let Ok(json) = serde_json::to_string(s) {
            let res = self.db.conn().execute(
                "INSERT INTO resolve_cache (track_id, itag, json, expires_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(track_id, itag) DO UPDATE SET json = excluded.json, expires_at = excluded.expires_at",
                (&key.0, key.1, json, s.expires_at as i64),
            );
            if let Err(e) = res {
                tracing::warn!(error = %e, "no se pudo guardar la resolución en caché");
            }
        }
        self.mem.lock().unwrap().insert(key, s.clone());
    }

    /// Borra entradas caducadas (se llama al arrancar).
    pub fn purge_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let _ = self
            .db
            .conn()
            .execute("DELETE FROM resolve_cache WHERE expires_at < ?1", [now]);
    }
}

impl<R: StreamResolver> StreamResolver for CachedResolver<R> {
    fn resolve(&self, id: &TrackId, pref: FormatPreference) -> AppResult<ResolvedStream> {
        let key = (id.0.clone(), pref.itag());
        if let Some(s) = self.lookup(&key) {
            tracing::debug!(track = %id.0, "resolución desde caché");
            return Ok(s);
        }
        let (flight, leader) = {
            let mut map = self.inflight.lock().unwrap();
            match map.get(&key) {
                Some(f) => (Arc::clone(f), false),
                None => {
                    let f = Arc::new(Flight::default());
                    map.insert(key.clone(), Arc::clone(&f));
                    (f, true)
                }
            }
        };
        if !leader {
            tracing::debug!(track = %id.0, "esperando a la resolución en marcha");
            let mut r = flight.result.lock().unwrap();
            while r.is_none() {
                r = flight.done.wait(r).unwrap();
            }
            return r.clone().expect("resultado");
        }
        let t0 = std::time::Instant::now();
        let res = self.inner.resolve(id, pref);
        tracing::info!(track = %id.0, ms = t0.elapsed().as_millis() as u64, ok = res.is_ok(), "yt-dlp");
        if let Ok(s) = &res {
            self.store(key.clone(), s);
        }
        *flight.result.lock().unwrap() = Some(res.clone());
        flight.done.notify_all();
        self.inflight.lock().unwrap().remove(&key);
        res
    }

    fn cached(&self, id: &TrackId, pref: FormatPreference) -> Option<ResolvedStream> {
        self.lookup(&(id.0.clone(), pref.itag()))
    }

    fn invalidate(&self, id: &TrackId) {
        self.mem.lock().unwrap().retain(|k, _| k.0 != id.0);
        let _ = self
            .db
            .conn()
            .execute("DELETE FROM resolve_cache WHERE track_id = ?1", [&id.0]);
        self.inner.invalidate(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct Fake {
        calls: AtomicU32,
        expires_in: u64,
    }

    impl StreamResolver for Fake {
        fn resolve(&self, id: &TrackId, _: FormatPreference) -> AppResult<ResolvedStream> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            // Como yt-dlp: tarda un poco.
            std::thread::sleep(std::time::Duration::from_millis(50));
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            Ok(ResolvedStream {
                track_id: id.clone(),
                url: format!("https://x/{}?expire={}", id.0, now + self.expires_in),
                itag: 140,
                content_length: Some(10),
                duration_ms: Some(1000),
                expires_at: now + self.expires_in,
                user_agent: None,
                upgrade: None,
            })
        }
    }

    #[test]
    fn peticiones_simultaneas_comparten_una_resolucion() {
        let (r, _db) = cached(3600);
        let r = Arc::new(r);
        let hilos: Vec<_> = (0..4)
            .map(|_| {
                let r = Arc::clone(&r);
                std::thread::spawn(move || {
                    r.resolve(&TrackId("abc".into()), FormatPreference::Aac128)
                })
            })
            .collect();
        for h in hilos {
            assert!(h.join().unwrap().is_ok());
        }
        assert_eq!(r.inner.calls.load(Ordering::SeqCst), 1);
    }

    fn cached(expires_in: u64) -> (CachedResolver<Fake>, Arc<Db>) {
        let db = Arc::new(Db::open_in_memory().unwrap());
        (
            CachedResolver::new(
                Fake {
                    calls: AtomicU32::new(0),
                    expires_in,
                },
                Arc::clone(&db),
            ),
            db,
        )
    }

    #[test]
    fn reutiliza_mientras_es_valida() {
        let (r, _) = cached(6 * 3600);
        let id = TrackId("abc".into());
        let a = r.resolve(&id, FormatPreference::Aac128).unwrap();
        let b = r.resolve(&id, FormatPreference::Aac128).unwrap();
        assert_eq!(a, b);
        assert_eq!(r.inner.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn no_reutiliza_si_caduca_pronto() {
        let (r, _) = cached(60); // caduca dentro del margen de 10 min
        let id = TrackId("abc".into());
        r.resolve(&id, FormatPreference::Aac128).unwrap();
        r.resolve(&id, FormatPreference::Aac128).unwrap();
        assert_eq!(r.inner.calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn sobrevive_a_reinicio_via_sqlite_e_invalida() {
        let (r, db) = cached(6 * 3600);
        let id = TrackId("abc".into());
        r.resolve(&id, FormatPreference::Aac128).unwrap();
        let r2 = CachedResolver::new(
            Fake {
                calls: AtomicU32::new(0),
                expires_in: 6 * 3600,
            },
            db,
        );
        r2.resolve(&id, FormatPreference::Aac128).unwrap();
        assert_eq!(
            r2.inner.calls.load(Ordering::SeqCst),
            0,
            "debe venir de SQLite"
        );
        r2.invalidate(&id);
        r2.resolve(&id, FormatPreference::Aac128).unwrap();
        assert_eq!(r2.inner.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn parsea_salida_de_yt_dlp() {
        let json = br#"{"url":"https://rr3.googlevideo.com/videoplayback?expire=1792466577&itag=140","format_id":"140","filesize":3449447,"duration":213.0,"http_headers":{"User-Agent":"Mozilla/5.0"}}"#;
        let s = parse_info(&TrackId("dQw4w9WgXcQ".into()), json).unwrap();
        assert_eq!(s.itag, 140);
        assert_eq!(s.expires_at, 1_792_466_577);
        assert_eq!(s.duration_ms, Some(213_000));
        assert_eq!(s.content_length, Some(3_449_447));
        assert_eq!(s.user_agent.as_deref(), Some("Mozilla/5.0"));
    }

    /// Prueba real con yt-dlp: `cargo test -- --ignored resuelve_de_verdad`.
    #[test]
    #[ignore]
    fn resuelve_de_verdad() {
        let tools = ToolPaths::locate().expect("tools/ con yt-dlp y deno");
        let s = YtDlpResolver::new(tools, None)
            .resolve(&TrackId("dQw4w9WgXcQ".into()), FormatPreference::Aac128)
            .unwrap();
        assert_eq!(s.itag, 140);
        assert!(s.url.contains("googlevideo.com"));
    }
}
