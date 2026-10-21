//! Servicio de YouTube Music: sesión, cliente, caché y operaciones (specs/003).

use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::Semaphore;

use super::innertube::{Raw, Yt, run};
use super::parse;
use super::session::{self, CookieSource, Session, SessionStore, now_secs};
use crate::domain::{
    AlbumDetail, AlbumSummary, ArtistDetail, ArtistSummary, BrowseId, LikeStatus, MediaItem, Page,
    PlaylistDetail, PlaylistId, PlaylistSummary, Privacy, SearchFilter, Section, Thumbnail, Track,
    TrackId,
};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::storage::Db;

/// Peticiones simultáneas a YouTube como máximo (NFR-003-02).
const MAX_CONCURRENT: usize = 4;
/// Una sesión importada del navegador se refresca si tiene más de esto.
const SESSION_REFRESH_AFTER: u64 = 3600;

// ------------------------------------------------------------------ contrato

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub name: String,
    pub handle: Option<String>,
    pub photo: Option<String>,
}

#[derive(Serialize, Type, Event, Clone, Debug, PartialEq)]
#[tauri_specta(event_name = "auth://changed")]
pub enum AuthStatus {
    SignedOut,
    SignedIn(AccountInfo),
    /// La sesión caducó y no se pudo renovar sola.
    Expired(Option<String>),
}

/// De dónde viene una cuenta candidata al importar.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all_fields = "camelCase")]
pub enum AccountSource {
    /// Perfil del navegador + contenedor (`originAttributes`).
    Browser {
        profile: String,
        container: String,
        container_name: String,
    },
    /// Cookie pegada a mano (queda en memoria hasta elegir cuenta).
    Pasted,
    /// Ventana de inicio de sesión de la propia app.
    App,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionAccount {
    pub source: AccountSource,
    /// Índice dentro de la sesión (`X-Goog-AuthUser`).
    pub index: u32,
    pub name: String,
    pub handle: Option<String>,
    pub photo: Option<String>,
}

#[derive(Serialize, Type, Event, Clone, Debug, PartialEq)]
#[tauri_specta(event_name = "yt://cache-updated")]
pub struct CacheUpdated {
    pub key: String,
}

// ------------------------------------------------------------------ servicio

pub struct YtService {
    /// `None` en pruebas: no se emiten eventos.
    app: Option<AppHandle>,
    db: Arc<Db>,
    store: SessionStore,
    session: RwLock<Option<Session>>,
    expired: RwLock<bool>,
    client: tokio::sync::Mutex<Option<Arc<Yt>>>,
    limiter: Semaphore,
    pasted: Mutex<Option<String>>,
    /// Cookies de la ventana de inicio de sesión, hasta elegir cuenta.
    app_login: Mutex<Option<super::login::Cookies>>,
}

impl YtService {
    pub fn new(app: Option<AppHandle>, db: Arc<Db>, data_dir: PathBuf) -> Arc<Self> {
        let store = SessionStore::new(&data_dir);
        let session = store.load();
        let svc = Arc::new(Self {
            app,
            db,
            store,
            session: RwLock::new(session),
            expired: RwLock::new(false),
            client: tokio::sync::Mutex::new(None),
            limiter: Semaphore::new(MAX_CONCURRENT),
            pasted: Mutex::new(None),
            app_login: Mutex::new(None),
        });
        // Sesiones viejas importadas del navegador: se renuevan en segundo plano.
        let stale = svc.session.read().unwrap().as_ref().is_some_and(|s| {
            (s.source.is_some() || s.app_login)
                && now_secs().saturating_sub(s.imported_at) > SESSION_REFRESH_AFTER
        });
        if stale {
            let s = Arc::clone(&svc);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = s.refresh_session().await {
                    tracing::warn!(error = %e, "no se pudo renovar la sesión al arrancar");
                }
            });
        }
        svc
    }

    /// Cookies Netscape de la sesión, para que yt-dlp reintente con cuenta.
    pub fn netscape_cookies(&self) -> Option<String> {
        self.session
            .read()
            .unwrap()
            .as_ref()
            .map(|s| s.netscape.clone())
    }

    pub fn status(&self) -> AuthStatus {
        let s = self.session.read().unwrap();
        match s.as_ref() {
            None => AuthStatus::SignedOut,
            Some(s) if *self.expired.read().unwrap() => AuthStatus::Expired(s.account_name.clone()),
            Some(s) => AuthStatus::SignedIn(AccountInfo {
                name: s
                    .account_name
                    .clone()
                    .unwrap_or_else(|| "Cuenta de YouTube".into()),
                handle: s.account_handle.clone(),
                photo: s.account_photo.clone(),
            }),
        }
    }

    fn emit_status(&self) {
        if let Some(app) = &self.app {
            let _ = self.status().emit(app);
        }
    }

    fn require_session(&self) -> AppResult<()> {
        if self.session.read().unwrap().is_none() {
            return Err(AppError::new(
                ErrorCode::NotAuthenticated,
                "Inicia sesión en YouTube Music para ver esto.",
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------ cliente

    async fn build_client(cookie_header: &str, auth_user: u32) -> AppResult<Yt> {
        let http = ytmapi_rs::Client::new().map_err(|e| AppError::internal(e.to_string()))?;
        let token = ytmapi_rs::auth::BrowserToken::from_str(cookie_header, &http)
            .await
            .map_err(|e| super::innertube::map_error("login", &e))?
            .with_auth_user(auth_user);
        ytmapi_rs::YtMusicBuilder::new_with_client(http)
            .with_auth_token(token)
            .build()
            .map_err(|e| AppError::internal(e.to_string()))
    }

    async fn client(&self) -> AppResult<Arc<Yt>> {
        let mut guard = self.client.lock().await;
        if let Some(c) = guard.as_ref() {
            return Ok(Arc::clone(c));
        }
        let (cookie, auth_user) = {
            let s = self.session.read().unwrap();
            let s = s.as_ref().ok_or_else(|| {
                AppError::new(ErrorCode::NotAuthenticated, "No hay sesión iniciada.")
            })?;
            (s.cookie_header.clone(), s.auth_user)
        };
        let c = Arc::new(Self::build_client(&cookie, auth_user).await?);
        *guard = Some(Arc::clone(&c));
        Ok(c)
    }

    /// Ejecuta una consulta autenticada: limita la concurrencia, detecta sesión
    /// caducada (también la que responde 200 con "Sign in") y la renueva una vez.
    async fn query(&self, q: impl Fn() -> Raw) -> AppResult<Value> {
        self.require_session()?;
        let _permit = self
            .limiter
            .acquire()
            .await
            .map_err(|_| AppError::internal("limitador cerrado"))?;
        let mut renewed = false;
        let mut waited = false;
        loop {
            let yt = self.client().await?;
            let result = run(&yt, q()).await.and_then(|v| {
                if signed_out(&v) {
                    Err(AppError::new(
                        ErrorCode::AuthExpired,
                        "La sesión de YouTube Music ha caducado.",
                    ))
                } else {
                    Ok(v)
                }
            });
            match result {
                Err(e) if e.code == ErrorCode::AuthExpired && !renewed => {
                    renewed = true;
                    self.refresh_session().await?;
                }
                Err(e) if e.code == ErrorCode::RateLimited && !waited => {
                    waited = true;
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                }
                other => return other,
            }
        }
    }

    // ------------------------------------------------------------ sesión

    /// Relee las cookies del navegador de origen y comprueba la cuenta.
    pub async fn refresh_session(&self) -> AppResult<()> {
        let current = self.session.read().unwrap().clone();
        let Some(mut s) = current else {
            return Err(AppError::new(
                ErrorCode::NotAuthenticated,
                "No hay sesión iniciada.",
            ));
        };
        let (header, netscape) = if s.app_login {
            let app = self
                .app
                .as_ref()
                .ok_or_else(|| AppError::internal("sin AppHandle"))?;
            super::login::refresh(app)
                .await
                .inspect_err(|_| self.mark_expired())?
        } else {
            let Some(source) = s.source.clone() else {
                self.mark_expired();
                return Err(AppError::new(
                    ErrorCode::AuthExpired,
                    "La sesión caducó. Vuelve a iniciar sesión.",
                ));
            };
            tokio::task::spawn_blocking(move || session::read_cookies(&source))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
                .inspect_err(|_| self.mark_expired())?
        };
        // La cuenta puede haber cambiado de índice si se añadieron otras en el navegador.
        let accounts = Self::accounts_in(&header).await?;
        let found = accounts
            .iter()
            .find(|(_, name, _, _)| Some(name) == s.account_name.as_ref())
            .or_else(|| accounts.iter().find(|(i, ..)| *i == s.auth_user));
        let Some((index, name, handle, photo)) = found.cloned() else {
            self.mark_expired();
            return Err(AppError::new(
                ErrorCode::AuthExpired,
                "La cuenta ya no está en ese navegador. Vuelve a iniciar sesión.",
            ));
        };
        s.cookie_header = header;
        s.netscape = netscape;
        s.auth_user = index;
        s.account_name = Some(name);
        s.account_handle = handle;
        s.account_photo = photo;
        s.imported_at = now_secs();
        let s_app_login = s.app_login;
        self.store.save(&s)?;
        *self.session.write().unwrap() = Some(s);
        *self.expired.write().unwrap() = false;
        *self.client.lock().await = None;
        tracing::info!(app_login = s_app_login, "sesión de YouTube Music renovada");
        self.emit_status();
        Ok(())
    }

    fn mark_expired(&self) {
        *self.expired.write().unwrap() = true;
        self.emit_status();
    }

    /// Cuentas presentes en una cookie: (índice, nombre, handle, foto).
    ///
    /// Un solo token para todos los índices y las consultas de cuenta en
    /// paralelo (antes: token + consulta, una tras otra, por cada índice).
    async fn accounts_in(
        cookie: &str,
    ) -> AppResult<Vec<(u32, String, Option<String>, Option<String>)>> {
        const MAX_ACCOUNTS: u32 = 5;
        let http = ytmapi_rs::Client::new().map_err(|e| AppError::internal(e.to_string()))?;
        let token = ytmapi_rs::auth::BrowserToken::from_str(cookie, &http)
            .await
            .map_err(|e| super::innertube::map_error("login", &e))?;
        let mut set = tokio::task::JoinSet::new();
        for index in 0..MAX_ACCOUNTS {
            let yt = ytmapi_rs::YtMusicBuilder::new_with_client(http.clone())
                .with_auth_token(token.clone().with_auth_user(index))
                .build()
                .map_err(|e| AppError::internal(e.to_string()))?;
            set.spawn(async move {
                let v = run(&yt, Raw::new("account/account_menu", json!({}))).await;
                (index, v)
            });
        }
        let mut answers: Vec<(u32, AppResult<Value>)> = set.join_all().await;
        answers.sort_by_key(|(i, _)| *i);

        // Mismo criterio que antes, en orden de índice.
        let mut out = Vec::new();
        for (index, v) in answers {
            let v = match v {
                Ok(v) => v,
                Err(e) if index == 0 => return Err(e),
                Err(_) => break,
            };
            match parse::account_info(&v) {
                Some((name, handle, photos)) => {
                    // Con un índice inexistente Google devuelve la cuenta 0 otra vez.
                    if out.iter().any(
                        |(_, n, h, _): &(u32, String, Option<String>, Option<String>)| {
                            *n == name && *h == handle
                        },
                    ) {
                        break;
                    }
                    out.push((index, name, handle, photos.last().map(|t| t.url.clone())));
                }
                None => break,
            }
        }
        if out.is_empty() {
            return Err(AppError::new(
                ErrorCode::NotAuthenticated,
                "Esa sesión no tiene ninguna cuenta iniciada.",
            ));
        }
        Ok(out)
    }

    pub fn detect_browsers(&self) -> Vec<session::BrowserProfile> {
        session::detect_profiles()
    }

    /// Cuentas disponibles en un perfil de navegador (todas sus sesiones/contenedores).
    pub async fn scan_browser(&self, profile: String) -> AppResult<Vec<SessionAccount>> {
        let path = PathBuf::from(&profile);
        let containers = {
            let p = path.clone();
            tokio::task::spawn_blocking(move || session::logged_in_containers(&p))
                .await
                .map_err(|e| AppError::internal(e.to_string()))??
        };
        if containers.is_empty() {
            return Err(AppError::new(
                ErrorCode::NotAuthenticated,
                "Ese navegador no tiene ninguna sesión de YouTube iniciada.",
            ));
        }
        let mut out = Vec::new();
        for c in containers {
            let source = CookieSource {
                profile: path.clone(),
                container: c.id.clone(),
            };
            let read = tokio::task::spawn_blocking(move || session::read_cookies(&source))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?;
            let Ok((header, _)) = read else { continue };
            match Self::accounts_in(&header).await {
                Ok(accounts) => {
                    out.extend(accounts.into_iter().map(|(index, name, handle, photo)| {
                        SessionAccount {
                            source: AccountSource::Browser {
                                profile: profile.clone(),
                                container: c.id.clone(),
                                container_name: c.name.clone(),
                            },
                            index,
                            name,
                            handle,
                            photo,
                        }
                    }))
                }
                Err(e) => {
                    tracing::info!(container = %c.name, error = %e, "contenedor sin cuentas válidas")
                }
            }
        }
        if out.is_empty() {
            return Err(AppError::new(
                ErrorCode::AuthExpired,
                "La sesión de ese navegador no es válida. Abre music.youtube.com en él y vuelve a probar.",
            ));
        }
        Ok(out)
    }

    /// Abre la ventana de inicio de sesión de Google y devuelve las cuentas de esa sesión.
    pub async fn sign_in_app(&self) -> AppResult<Vec<SessionAccount>> {
        let app = self
            .app
            .as_ref()
            .ok_or_else(|| AppError::internal("sin AppHandle"))?;
        let cookies = super::login::sign_in(app).await?;
        let accounts = Self::accounts_in(&cookies.0).await?;
        *self.app_login.lock().unwrap() = Some(cookies);
        Ok(accounts
            .into_iter()
            .map(|(index, name, handle, photo)| SessionAccount {
                source: AccountSource::App,
                index,
                name,
                handle,
                photo,
            })
            .collect())
    }

    /// Cuentas en una cookie pegada a mano.
    pub async fn scan_pasted(&self, raw: String) -> AppResult<Vec<SessionAccount>> {
        let cookie = session::clean_pasted_cookie(&raw)?;
        let accounts = Self::accounts_in(&cookie).await?;
        *self.pasted.lock().unwrap() = Some(cookie);
        Ok(accounts
            .into_iter()
            .map(|(index, name, handle, photo)| SessionAccount {
                source: AccountSource::Pasted,
                index,
                name,
                handle,
                photo,
            })
            .collect())
    }

    pub async fn select_account(&self, account: SessionAccount) -> AppResult<AuthStatus> {
        let (cookie_header, netscape, source) = match &account.source {
            AccountSource::Browser {
                profile, container, ..
            } => {
                let source = CookieSource {
                    profile: PathBuf::from(profile),
                    container: container.clone(),
                };
                let s2 = source.clone();
                let (h, n) = tokio::task::spawn_blocking(move || session::read_cookies(&s2))
                    .await
                    .map_err(|e| AppError::internal(e.to_string()))??;
                (h, n, Some(source))
            }
            AccountSource::Pasted => {
                let cookie = self.pasted.lock().unwrap().take().ok_or_else(|| {
                    AppError::new(ErrorCode::InvalidInput, "Vuelve a pegar la cookie.")
                })?;
                let n = session::netscape_from_header(&cookie);
                (cookie, n, None)
            }
            AccountSource::App => {
                let (h, n) = self.app_login.lock().unwrap().take().ok_or_else(|| {
                    AppError::new(ErrorCode::InvalidInput, "Vuelve a iniciar sesión.")
                })?;
                (h, n, None)
            }
        };
        let app_login = account.source == AccountSource::App;
        let s = Session {
            cookie_header,
            netscape,
            auth_user: account.index,
            source,
            account_name: Some(account.name),
            account_handle: account.handle,
            account_photo: account.photo,
            imported_at: now_secs(),
            app_login,
        };
        self.store.save(&s)?;
        *self.session.write().unwrap() = Some(s);
        *self.expired.write().unwrap() = false;
        *self.client.lock().await = None;
        self.clear_cache();
        tracing::info!(
            index = account.index,
            "cuenta de YouTube Music seleccionada"
        );
        self.emit_status();
        Ok(self.status())
    }

    pub async fn logout(&self) {
        let was_app = self
            .session
            .read()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.app_login);
        if was_app {
            // Cerrar sesión de verdad: fuera también el perfil de la ventana de Google.
            let _ = tokio::task::spawn_blocking(super::login::forget).await;
        }
        self.store.clear();
        *self.session.write().unwrap() = None;
        *self.expired.write().unwrap() = false;
        *self.client.lock().await = None;
        self.clear_cache();
        self.emit_status();
    }

    // ------------------------------------------------------------ caché

    fn cache_prefix(&self) -> String {
        let s = self.session.read().unwrap();
        s.as_ref()
            .map(|s| {
                format!(
                    "{}#{}",
                    s.account_name.clone().unwrap_or_default(),
                    s.auth_user
                )
            })
            .unwrap_or_default()
    }

    fn clear_cache(&self) {
        let _ = self.db.conn().execute("DELETE FROM api_cache", []);
    }

    fn cache_get<T: DeserializeOwned>(&self, key: &str) -> Option<(T, u64)> {
        let (json, at): (String, i64) = self
            .db
            .conn()
            .query_row(
                "SELECT json, fetched_at FROM api_cache WHERE key = ?1",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok()?;
        Some((serde_json::from_str(&json).ok()?, at as u64))
    }

    fn cache_put<T: Serialize>(&self, key: &str, value: &T) {
        if let Ok(json) = serde_json::to_string(value) {
            let _ = self.db.conn().execute(
                "INSERT INTO api_cache (key, json, fetched_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET json = excluded.json, fetched_at = excluded.fetched_at",
                (key, json, now_secs() as i64),
            );
        }
    }

    /// Devuelve la caché al instante si existe; si está vieja, la refresca en
    /// segundo plano y avisa con `yt://cache-updated` (FR-003-16).
    async fn cached<T, F, Fut>(self: &Arc<Self>, name: &str, ttl: u64, fetch: F) -> AppResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: Fn(Arc<Self>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AppResult<T>> + Send + 'static,
    {
        let key = format!("{}|{name}", self.cache_prefix());
        if let Some((value, at)) = self.cache_get::<T>(&key) {
            if now_secs().saturating_sub(at) > ttl {
                let svc = Arc::clone(self);
                let name = name.to_owned();
                tauri::async_runtime::spawn(async move {
                    match fetch(Arc::clone(&svc)).await {
                        Ok(fresh) => {
                            svc.cache_put(&key, &fresh);
                            if let Some(app) = &svc.app {
                                let _ = CacheUpdated { key: name }.emit(app);
                            }
                        }
                        Err(e) => tracing::debug!(error = %e, "no se pudo refrescar la caché"),
                    }
                });
            }
            return Ok(value);
        }
        let fresh = fetch(Arc::clone(self)).await?;
        self.cache_put(&key, &fresh);
        Ok(fresh)
    }

    /// Invalida entradas cuyo nombre empiece por `prefix`.
    fn invalidate(&self, prefix: &str) {
        let pattern = format!("{}|{prefix}%", self.cache_prefix());
        let _ = self
            .db
            .conn()
            .execute("DELETE FROM api_cache WHERE key LIKE ?1", [pattern]);
    }

    // ------------------------------------------------------------ lectura

    pub async fn home(self: &Arc<Self>, continuation: Option<String>) -> AppResult<Page<Section>> {
        if let Some(c) = continuation {
            return Ok(parse::home_page(
                &self.query(|| Raw::continuation("browse", &c)).await?,
            ));
        }
        self.cached("home", 10 * 60, |s| async move {
            Ok(parse::home_page(
                &s.query(|| Raw::browse("FEmusic_home")).await?,
            ))
        })
        .await
    }

    pub async fn library_playlists(self: &Arc<Self>) -> AppResult<Vec<PlaylistSummary>> {
        self.cached("library:playlists", 3600, |s| async move {
            let v = s.query(|| Raw::browse("FEmusic_liked_playlists")).await?;
            Ok(parse::grid_items(&v)
                .into_iter()
                .filter_map(|i| match i {
                    MediaItem::Playlist(p) => Some(p),
                    _ => None,
                })
                .collect())
        })
        .await
    }

    pub async fn library_albums(self: &Arc<Self>) -> AppResult<Vec<AlbumSummary>> {
        self.cached("library:albums", 3600, |s| async move {
            let v = s.query(|| Raw::browse("FEmusic_liked_albums")).await?;
            Ok(parse::grid_items(&v)
                .into_iter()
                .filter_map(|i| match i {
                    MediaItem::Album(a) => Some(a),
                    _ => None,
                })
                .collect())
        })
        .await
    }

    pub async fn library_artists(self: &Arc<Self>) -> AppResult<Vec<ArtistSummary>> {
        self.cached("library:artists", 3600, |s| async move {
            let v = s
                .query(|| Raw::browse("FEmusic_library_corpus_track_artists"))
                .await?;
            Ok(parse::search_page(&v)
                .items
                .into_iter()
                .filter_map(|i| match i {
                    MediaItem::Artist(a) => Some(a),
                    _ => None,
                })
                .collect())
        })
        .await
    }

    pub async fn library_songs(self: &Arc<Self>) -> AppResult<Page<Track>> {
        self.cached("library:songs", 3600, |s| async move {
            Ok(parse::track_page(
                &s.query(|| Raw::browse("FEmusic_liked_videos")).await?,
            ))
        })
        .await
    }

    pub async fn liked_songs(self: &Arc<Self>) -> AppResult<Page<Track>> {
        self.cached("library:liked", 3600, |s| async move {
            Ok(parse::track_page(&s.query(|| Raw::browse("VLLM")).await?))
        })
        .await
    }

    pub async fn history(self: &Arc<Self>) -> AppResult<Vec<Section>> {
        // El historial cambia con cada escucha: caché corta.
        self.cached("history", 60, |s| async move {
            Ok(parse::history_page(
                &s.query(|| Raw::browse("FEmusic_history")).await?,
            ))
        })
        .await
    }

    /// Siguiente página de una lista de canciones (playlist, me gusta, biblioteca).
    pub async fn more_tracks(&self, continuation: String) -> AppResult<Page<Track>> {
        Ok(parse::track_page(
            &self
                .query(|| Raw::continuation("browse", &continuation))
                .await?,
        ))
    }

    pub async fn playlist(self: &Arc<Self>, id: PlaylistId) -> AppResult<PlaylistDetail> {
        let browse = if id.0.starts_with("VL") {
            id.0.clone()
        } else {
            format!("VL{}", id.0)
        };
        let key = format!("playlist:{browse}");
        let id2 = PlaylistId(browse.clone());
        self.cached(&key, 3600, move |s| {
            let browse = browse.clone();
            let id = id2.clone();
            async move {
                Ok(parse::playlist_page(
                    &s.query(|| Raw::browse(&browse)).await?,
                    &id,
                ))
            }
        })
        .await
    }

    pub async fn album(self: &Arc<Self>, id: BrowseId) -> AppResult<AlbumDetail> {
        let key = format!("album:{}", id.0);
        self.cached(&key, 24 * 3600, move |s| {
            let id = id.clone();
            async move {
                Ok(parse::album_page(
                    &s.query(|| Raw::browse(&id.0)).await?,
                    &id,
                ))
            }
        })
        .await
    }

    pub async fn artist(self: &Arc<Self>, id: BrowseId) -> AppResult<ArtistDetail> {
        let key = format!("artist:{}", id.0);
        self.cached(&key, 24 * 3600, move |s| {
            let id = id.clone();
            async move {
                Ok(parse::artist_page(
                    &s.query(|| Raw::browse(&id.0)).await?,
                    &id,
                ))
            }
        })
        .await
    }

    pub async fn search(&self, query: String, filter: SearchFilter) -> AppResult<Page<MediaItem>> {
        let params = search_params(filter);
        let v = self
            .query(|| {
                let mut body = json!({ "query": query });
                if let Some(p) = params {
                    body["params"] = json!(p);
                }
                Raw::new("search", body)
            })
            .await?;
        let mut page = parse::search_page(&v);
        // Con filtro, solo lo pedido (el destacado puede ser de otro tipo).
        page.items.retain(|i| filter_matches(filter, i));
        Ok(page)
    }

    pub async fn more_search(
        &self,
        continuation: String,
        filter: SearchFilter,
    ) -> AppResult<Page<MediaItem>> {
        let mut page = parse::search_page(
            &self
                .query(|| Raw::continuation("search", &continuation))
                .await?,
        );
        page.items.retain(|i| filter_matches(filter, i));
        Ok(page)
    }

    pub async fn suggestions(&self, query: String) -> AppResult<Vec<String>> {
        Ok(parse::suggestions(
            &self
                .query(|| Raw::new("music/get_search_suggestions", json!({ "input": query })))
                .await?,
        ))
    }

    /// Radio a partir de una canción (incluye la semilla primero).
    pub async fn radio(&self, seed: TrackId) -> AppResult<Vec<Track>> {
        let v = self
            .query(|| Raw::new("next", json!({ "videoId": seed.0, "playlistId": format!("RDAMVM{}", seed.0), "isAudioOnly": true, "enablePersistentPlaylistPanel": true })))
            .await?;
        Ok(parse::radio_page(&v).items)
    }

    /// Datos del reproductor web (URL de historial y sonoridad).
    pub async fn player_info(&self, id: &TrackId) -> AppResult<parse::PlayerInfo> {
        Ok(parse::player_info(
            &self
                .query(|| Raw::new("player", json!({ "videoId": id.0 })))
                .await?,
        ))
    }

    /// Registra una escucha en el historial (FR-003-14).
    pub async fn report_listen(&self, tracking_url: String) -> AppResult<()> {
        self.require_session()?;
        let yt = self.client().await?;
        use ytmapi_rs::common::YoutubeID;
        yt.add_history_item(ytmapi_rs::common::SongTrackingUrl::from_raw(tracking_url))
            .await
            .map_err(|e| super::innertube::map_error("historial", &e))?;
        self.invalidate("history");
        Ok(())
    }

    // ------------------------------------------------------------ escritura

    pub async fn rate(&self, id: TrackId, status: LikeStatus) -> AppResult<()> {
        let path = match status {
            LikeStatus::Like => "like/like",
            LikeStatus::Dislike => "like/dislike",
            LikeStatus::Indifferent => "like/removelike",
        };
        self.query(|| Raw::new(path, json!({ "target": { "videoId": id.0 } })))
            .await?;
        self.invalidate("library:liked");
        Ok(())
    }

    pub async fn add_to_playlist(
        &self,
        playlist: PlaylistId,
        tracks: Vec<TrackId>,
    ) -> AppResult<()> {
        let plain = playlist
            .0
            .strip_prefix("VL")
            .unwrap_or(&playlist.0)
            .to_owned();
        let actions: Vec<Value> = tracks
            .iter()
            .map(|t| json!({ "action": "ACTION_ADD_VIDEO", "addedVideoId": t.0, "dedupeOption": "DEDUPE_OPTION_SKIP" }))
            .collect();
        let v = self
            .query(|| {
                Raw::new(
                    "browse/edit_playlist",
                    json!({ "playlistId": plain, "actions": actions }),
                )
            })
            .await?;
        if v.get("status")
            .and_then(Value::as_str)
            .is_some_and(|s| s != "STATUS_SUCCEEDED")
        {
            return Err(AppError::new(
                ErrorCode::Internal,
                "YouTube Music no aceptó el cambio en la playlist.",
            ));
        }
        self.invalidate(&format!("playlist:VL{plain}"));
        self.invalidate("library:playlists");
        Ok(())
    }

    pub async fn create_playlist(
        &self,
        title: String,
        description: Option<String>,
        privacy: Privacy,
    ) -> AppResult<PlaylistId> {
        let privacy = match privacy {
            Privacy::Private => "PRIVATE",
            Privacy::Unlisted => "UNLISTED",
            Privacy::Public => "PUBLIC",
        };
        let v = self
            .query(|| Raw::new("playlist/create", json!({ "title": title, "description": description.clone().unwrap_or_default(), "privacyStatus": privacy })))
            .await?;
        let id = v
            .get("playlistId")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::new(ErrorCode::Internal, "No se pudo crear la playlist."))?;
        self.invalidate("library:playlists");
        Ok(PlaylistId(format!("VL{id}")))
    }
}

/// ¿La respuesta es la de alguien sin sesión? (YouTube responde 200 con un
/// mensaje "Sign in" en lugar de un error).
fn signed_out(v: &Value) -> bool {
    parse::find_all(v, "messageRenderer")
        .into_iter()
        .any(|m| parse::find_first(m, "signInEndpoint").is_some())
}

fn search_params(filter: SearchFilter) -> Option<&'static str> {
    match filter {
        SearchFilter::All => None,
        SearchFilter::Songs => Some("EgWKAQIIAWoMEA4QChADEAQQCRAF"),
        SearchFilter::Videos => Some("EgWKAQIQAWoMEA4QChADEAQQCRAF"),
        SearchFilter::Albums => Some("EgWKAQIYAWoMEA4QChADEAQQCRAF"),
        SearchFilter::Artists => Some("EgWKAQIgAWoMEA4QChADEAQQCRAF"),
        SearchFilter::Playlists => Some("EgeKAQQoAEABagwQDhAKEAMQBBAJEAU%3D"),
    }
}

fn filter_matches(filter: SearchFilter, item: &MediaItem) -> bool {
    match filter {
        SearchFilter::All => true,
        SearchFilter::Songs | SearchFilter::Videos => matches!(item, MediaItem::Track(_)),
        SearchFilter::Albums => matches!(item, MediaItem::Album(_)),
        SearchFilter::Artists => matches!(item, MediaItem::Artist(_)),
        SearchFilter::Playlists => matches!(item, MediaItem::Playlist(_)),
    }
}

/// Primera miniatura de al menos `px` de ancho (o la mayor).
#[allow(dead_code)]
pub fn best_thumbnail(thumbs: &[Thumbnail], px: u32) -> Option<&Thumbnail> {
    thumbs
        .iter()
        .find(|t| t.width >= px)
        .or_else(|| thumbs.last())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_respuesta_sin_sesion() {
        let v = json!({ "contents": { "itemSectionRenderer": { "contents": [ { "messageRenderer": {
            "button": { "buttonRenderer": { "navigationEndpoint": { "signInEndpoint": { "hack": true } } } } } } ] } } });
        assert!(signed_out(&v));
        assert!(!signed_out(
            &json!({ "contents": { "musicShelfRenderer": {} } })
        ));
    }
}

#[cfg(test)]
mod live_tests {
    //! Contra la cuenta real (solo lectura):
    //! `cargo test --lib -- --ignored --nocapture servicio_real`
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn servicio_real_de_punta_a_punta() {
        let dir = std::env::temp_dir().join(format!("xp-music-live-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(Db::open(&dir.join("t.db")).unwrap());
        let yt = YtService::new(None, db, dir.clone());

        let profile = yt
            .detect_browsers()
            .into_iter()
            .next()
            .expect("perfil de navegador");
        let accounts = yt.scan_browser(profile.path).await.unwrap();
        for a in &accounts {
            println!(
                "cuenta: {} {:?} (índice {}) en {:?}",
                a.name, a.handle, a.index, a.source
            );
        }
        let wanted = std::env::var("XP_MUSIC_ACCOUNT_NAME")
            .expect("XP_MUSIC_ACCOUNT_NAME con el nombre de tu cuenta");
        let account = accounts
            .into_iter()
            .find(|a| a.name == wanted)
            .expect("cuenta esperada");
        println!("estado: {:?}", yt.select_account(account).await.unwrap());

        let t = std::time::Instant::now();
        let pls = yt.library_playlists().await.unwrap();
        println!(
            "playlists ({} ms): {:?}",
            t.elapsed().as_millis(),
            pls.iter().map(|p| &p.title).collect::<Vec<_>>()
        );
        let t = std::time::Instant::now();
        let again = yt.library_playlists().await.unwrap();
        println!("playlists desde caché: {} ms", t.elapsed().as_millis());
        assert_eq!(pls, again);

        let mine = pls
            .iter()
            .find(|p| p.title == "bachata")
            .or(pls.get(1))
            .unwrap();
        let detail = yt.playlist(mine.id.clone()).await.unwrap();
        println!(
            "playlist {}: {} pistas, propia={}",
            detail.summary.title,
            detail.tracks.items.len(),
            detail.owned
        );

        let liked = yt.liked_songs().await.unwrap();
        println!("me gusta: {}", liked.items.len());
        let hist = yt.history().await.unwrap();
        println!(
            "historial: {:?}",
            hist.iter()
                .map(|s| (&s.title, s.items.len()))
                .collect::<Vec<_>>()
        );
        let home = yt.home(None).await.unwrap();
        println!(
            "inicio: {:?} · más: {}",
            home.items.iter().map(|s| &s.title).collect::<Vec<_>>(),
            home.continuation.is_some()
        );
        if let Some(c) = home.continuation {
            let more = yt.home(Some(c)).await.unwrap();
            println!(
                "inicio (2ª página): {:?}",
                more.items.iter().map(|s| &s.title).collect::<Vec<_>>()
            );
        }
        for f in [
            SearchFilter::All,
            SearchFilter::Songs,
            SearchFilter::Albums,
            SearchFilter::Artists,
            SearchFilter::Playlists,
        ] {
            let r = yt.search("aventura".into(), f).await.unwrap();
            println!(
                "buscar {f:?}: {} resultados · más: {}",
                r.items.len(),
                r.continuation.is_some()
            );
            if f == SearchFilter::Songs
                && let Some(c) = r.continuation
            {
                let more = yt.more_search(c, f).await.unwrap();
                println!("  2ª página: {}", more.items.len());
            }
        }
        println!(
            "sugerencias: {:?}",
            yt.suggestions("romeo".into()).await.unwrap()
        );
        let radio = yt.radio(TrackId("yQd4xQBlBa0".into())).await.unwrap();
        println!("radio: {} pistas", radio.len());
        let album = yt
            .album(BrowseId("MPREb_WixYTw0z5H2".into()))
            .await
            .unwrap();
        println!(
            "álbum: {} ({} pistas)",
            album.summary.title,
            album.tracks.len()
        );
        let artist = yt
            .artist(BrowseId("UCDaQK4nrsvz27YzufrhJfgg".into()))
            .await
            .unwrap();
        println!(
            "artista: {} · {} top · {} secciones",
            artist.summary.name,
            artist.top_songs.len(),
            artist.sections.len()
        );
        let info = yt
            .player_info(&TrackId("yQd4xQBlBa0".into()))
            .await
            .unwrap();
        println!(
            "player: tracking={} loudness={:?}",
            info.tracking_url.is_some(),
            info.loudness_db
        );

        // Renovación desde el navegador (como si hubiera caducado).
        yt.refresh_session().await.unwrap();
        println!("tras renovar: {:?}", yt.status());
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod dev_setup {
    /// Deja la app conectada con la cuenta indicada (datos reales de la app):
    /// `cargo test --lib -- --ignored preparar_sesion_real`
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn preparar_sesion_real() {
        use super::*;
        let dirs = crate::state::AppDirs::resolve().unwrap();
        let db = Arc::new(Db::open(&dirs.data.join("xp-music.db")).unwrap());
        let yt = YtService::new(None, db, dirs.data.clone());
        let profile = yt.detect_browsers().into_iter().next().unwrap();
        let wanted = std::env::var("XP_MUSIC_ACCOUNT_NAME")
            .expect("XP_MUSIC_ACCOUNT_NAME con el nombre de tu cuenta");
        let account = yt
            .scan_browser(profile.path)
            .await
            .unwrap()
            .into_iter()
            .find(|a| a.name == wanted)
            .unwrap();
        println!("{:?}", yt.select_account(account).await.unwrap());
    }
}

#[cfg(test)]
mod bench_accounts {
    //! Red real, solo lectura (menú de cuenta) con la sesión guardada en la app:
    //! `cargo test --lib bench_cuentas -- --ignored --nocapture`.
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn bench_cuentas() {
        let dirs = crate::state::AppDirs::resolve().unwrap();
        let store = SessionStore::new(&dirs.data);
        let cookie = store.load().expect("sesión guardada").cookie_header;
        for vuelta in 0..3 {
            let t = std::time::Instant::now();
            let accounts = YtService::accounts_in(&cookie).await.unwrap();
            println!(
                "vuelta {vuelta}: {} cuentas en {} ms",
                accounts.len(),
                t.elapsed().as_millis()
            );
        }
    }
}
