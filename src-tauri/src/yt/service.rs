//! YouTube Music service: session, client, cache and operations (specs/003).

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

/// Maximum simultaneous requests to YouTube (NFR-003-02).
const MAX_CONCURRENT: usize = 4;
/// A session imported from the browser is refreshed if it is older than this.
const SESSION_REFRESH_AFTER: u64 = 3600;

// ------------------------------------------------------------------ contract

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
    /// The session expired and could not be renewed automatically.
    Expired(Option<String>),
}

/// Where a candidate account comes from when importing.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all_fields = "camelCase")]
pub enum AccountSource {
    /// Browser profile + container (`originAttributes`).
    Browser {
        profile: String,
        container: String,
        container_name: String,
    },
    /// Cookie pasted by hand (kept in memory until an account is chosen).
    Pasted,
    /// The app's own sign-in window.
    App,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionAccount {
    pub source: AccountSource,
    /// Index within the session (`X-Goog-AuthUser`).
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

// ------------------------------------------------------------------ service

pub struct YtService {
    /// `None` in tests: no events are emitted.
    app: Option<AppHandle>,
    db: Arc<Db>,
    store: SessionStore,
    session: RwLock<Option<Session>>,
    expired: RwLock<bool>,
    client: tokio::sync::Mutex<Option<Arc<Yt>>>,
    limiter: Semaphore,
    pasted: Mutex<Option<String>>,
    /// Cookies from the sign-in window, until an account is chosen.
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
        // Old sessions imported from the browser: renewed in the background.
        let stale = svc.session.read().unwrap().as_ref().is_some_and(|s| {
            (s.source.is_some() || s.app_login)
                && now_secs().saturating_sub(s.imported_at) > SESSION_REFRESH_AFTER
        });
        if stale {
            let s = Arc::clone(&svc);
            tauri::async_runtime::spawn(async move {
                if let Err(e) = s.refresh_session().await {
                    tracing::warn!(error = %e, "could not renew the session at startup");
                }
            });
        }
        svc
    }

    /// The session's Netscape cookies, so yt-dlp can retry with the account.
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
                    .unwrap_or_else(|| tr!("YouTube account", "Cuenta de YouTube")),
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
                tr!(
                    "Sign in to YouTube Music to see this.",
                    "Inicia sesión en YouTube Music para ver esto."
                ),
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------ client

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
                AppError::new(
                    ErrorCode::NotAuthenticated,
                    tr!("You are not signed in.", "No hay sesión iniciada."),
                )
            })?;
            (s.cookie_header.clone(), s.auth_user)
        };
        let c = Arc::new(Self::build_client(&cookie, auth_user).await?);
        *guard = Some(Arc::clone(&c));
        Ok(c)
    }

    /// Runs an authenticated query: limits concurrency, detects an expired session
    /// (including one that answers 200 with "Sign in") and renews it once.
    async fn query(&self, q: impl Fn() -> Raw) -> AppResult<Value> {
        self.require_session()?;
        let _permit = self
            .limiter
            .acquire()
            .await
            .map_err(|_| AppError::internal("limiter closed"))?;
        let mut renewed = false;
        let mut waited = false;
        loop {
            let yt = self.client().await?;
            let result = run(&yt, q()).await.and_then(|v| {
                if signed_out(&v) {
                    Err(AppError::new(
                        ErrorCode::AuthExpired,
                        tr!(
                            "The YouTube Music session has expired.",
                            "La sesión de YouTube Music ha caducado."
                        ),
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

    // ------------------------------------------------------------ session

    /// Re-reads the cookies from the source browser and checks the account.
    pub async fn refresh_session(&self) -> AppResult<()> {
        let current = self.session.read().unwrap().clone();
        let Some(mut s) = current else {
            return Err(AppError::new(
                ErrorCode::NotAuthenticated,
                tr!("You are not signed in.", "No hay sesión iniciada."),
            ));
        };
        let (header, netscape) = if s.app_login {
            let app = self
                .app
                .as_ref()
                .ok_or_else(|| AppError::internal("no AppHandle"))?;
            super::login::refresh(app)
                .await
                .inspect_err(|_| self.mark_expired())?
        } else {
            let Some(source) = s.source.clone() else {
                self.mark_expired();
                return Err(AppError::new(
                    ErrorCode::AuthExpired,
                    tr!(
                        "The session expired. Sign in again.",
                        "La sesión caducó. Vuelve a iniciar sesión."
                    ),
                ));
            };
            tokio::task::spawn_blocking(move || session::read_cookies(&source))
                .await
                .map_err(|e| AppError::internal(e.to_string()))?
                .inspect_err(|_| self.mark_expired())?
        };
        // The account's index may have changed if others were added in the browser.
        let accounts = Self::accounts_in(&header).await?;
        let found = accounts
            .iter()
            .find(|(_, name, _, _)| Some(name) == s.account_name.as_ref())
            .or_else(|| accounts.iter().find(|(i, ..)| *i == s.auth_user));
        let Some((index, name, handle, photo)) = found.cloned() else {
            self.mark_expired();
            return Err(AppError::new(
                ErrorCode::AuthExpired,
                tr!(
                    "The account is no longer in that browser. Sign in again.",
                    "La cuenta ya no está en ese navegador. Vuelve a iniciar sesión."
                ),
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
        tracing::info!(app_login = s_app_login, "YouTube Music session renewed");
        self.emit_status();
        Ok(())
    }

    fn mark_expired(&self) {
        *self.expired.write().unwrap() = true;
        self.emit_status();
    }

    /// Accounts present in a cookie: (index, name, handle, photo).
    ///
    /// A single token for all indices and the account queries run in
    /// parallel (before: token + query, one after the other, for each index).
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

        // Same criteria as before, in index order.
        let mut out = Vec::new();
        for (index, v) in answers {
            let v = match v {
                Ok(v) => v,
                Err(e) if index == 0 => return Err(e),
                Err(_) => break,
            };
            match parse::account_info(&v) {
                Some((name, handle, photos)) => {
                    // With a nonexistent index Google returns account 0 again.
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
                tr!(
                    "That session has no signed-in account.",
                    "Esa sesión no tiene ninguna cuenta iniciada."
                ),
            ));
        }
        Ok(out)
    }

    pub fn detect_browsers(&self) -> Vec<session::BrowserProfile> {
        session::detect_profiles()
    }

    /// Accounts available in a browser profile (all its sessions/containers).
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
                tr!(
                    "That browser has no signed-in YouTube session.",
                    "Ese navegador no tiene ninguna sesión de YouTube iniciada."
                ),
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
                    tracing::info!(container = %c.name, error = %e, "container without valid accounts")
                }
            }
        }
        if out.is_empty() {
            return Err(AppError::new(
                ErrorCode::AuthExpired,
                tr!(
                    "The session in that browser is not valid. Open music.youtube.com in it and try again.",
                    "La sesión de ese navegador no es válida. Abre music.youtube.com en él y vuelve a probar."
                ),
            ));
        }
        Ok(out)
    }

    /// Opens the Google sign-in window and returns the accounts of that session.
    pub async fn sign_in_app(&self) -> AppResult<Vec<SessionAccount>> {
        let app = self
            .app
            .as_ref()
            .ok_or_else(|| AppError::internal("no AppHandle"))?;
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

    /// Accounts in a cookie pasted by hand.
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
                    AppError::new(
                        ErrorCode::InvalidInput,
                        tr!("Paste the cookie again.", "Vuelve a pegar la cookie."),
                    )
                })?;
                let n = session::netscape_from_header(&cookie);
                (cookie, n, None)
            }
            AccountSource::App => {
                let (h, n) = self.app_login.lock().unwrap().take().ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidInput,
                        tr!("Sign in again.", "Vuelve a iniciar sesión."),
                    )
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
        tracing::info!(index = account.index, "YouTube Music account selected");
        self.emit_status();
        Ok(self.status())
    }

    pub async fn logout(&self) -> AppResult<()> {
        // Don't announce a sign-out if the persisted credentials are still accessible.
        self.store.clear()?;
        let was_app = self
            .session
            .read()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.app_login);
        if was_app {
            // Really sign out: the Google window's profile goes too.
            let _ = tokio::task::spawn_blocking(super::login::forget).await;
        }
        *self.session.write().unwrap() = None;
        *self.expired.write().unwrap() = false;
        *self.client.lock().await = None;
        self.clear_cache();
        self.emit_status();
        Ok(())
    }

    // ------------------------------------------------------------ cache

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

    /// Returns the cache instantly if present; if stale, refreshes it in the
    /// background and notifies with `yt://cache-updated` (FR-003-16).
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
                        Err(e) => tracing::debug!(error = %e, "could not refresh the cache"),
                    }
                });
            }
            return Ok(value);
        }
        let fresh = fetch(Arc::clone(self)).await?;
        self.cache_put(&key, &fresh);
        Ok(fresh)
    }

    /// Invalidates entries whose name starts with `prefix`.
    fn invalidate(&self, prefix: &str) {
        let pattern = format!("{}|{prefix}%", self.cache_prefix());
        let _ = self
            .db
            .conn()
            .execute("DELETE FROM api_cache WHERE key LIKE ?1", [pattern]);
    }

    // ------------------------------------------------------------ reads

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
        // History changes with every listen: short cache.
        self.cached("history", 60, |s| async move {
            Ok(parse::history_page(
                &s.query(|| Raw::browse("FEmusic_history")).await?,
            ))
        })
        .await
    }

    /// Next page of a track list (playlist, liked songs, library).
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
        // With a filter, only what was asked for (the top result may be of another type).
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

    /// Radio from a track (includes the seed first).
    pub async fn radio(&self, seed: TrackId) -> AppResult<Vec<Track>> {
        let v = self
            .query(|| Raw::new("next", json!({ "videoId": seed.0, "playlistId": format!("RDAMVM{}", seed.0), "isAudioOnly": true, "enablePersistentPlaylistPanel": true })))
            .await?;
        Ok(parse::radio_page(&v).items)
    }

    /// Web player data (history URL and loudness).
    pub async fn player_info(&self, id: &TrackId) -> AppResult<parse::PlayerInfo> {
        Ok(parse::player_info(
            &self
                .query(|| Raw::new("player", json!({ "videoId": id.0 })))
                .await?,
        ))
    }

    /// Records a listen in the history (FR-003-14).
    pub async fn report_listen(&self, tracking_url: String) -> AppResult<()> {
        self.require_session()?;
        let yt = self.client().await?;
        use ytmapi_rs::common::YoutubeID;
        yt.add_history_item(ytmapi_rs::common::SongTrackingUrl::from_raw(tracking_url))
            .await
            .map_err(|e| super::innertube::map_error("history", &e))?;
        self.invalidate("history");
        Ok(())
    }

    // ------------------------------------------------------------ writes

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
                tr!(
                    "YouTube Music did not accept the playlist change.",
                    "YouTube Music no aceptó el cambio en la playlist."
                ),
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
        let id = v.get("playlistId").and_then(Value::as_str).ok_or_else(|| {
            AppError::new(
                ErrorCode::Internal,
                tr!(
                    "Could not create the playlist.",
                    "No se pudo crear la playlist."
                ),
            )
        })?;
        self.invalidate("library:playlists");
        Ok(PlaylistId(format!("VL{id}")))
    }
}

/// Is the response that of someone who isn't signed in? (YouTube answers 200 with a
/// "Sign in" message instead of an error).
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

/// First thumbnail at least `px` wide (or the largest).
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
    fn detects_signed_out_response() {
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
    //! Against the real account (read-only):
    //! `cargo test --lib -- --ignored --nocapture real_service`
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn real_service_end_to_end() {
        let dir = std::env::temp_dir().join(format!("xp-music-live-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(Db::open(&dir.join("t.db")).unwrap());
        let yt = YtService::new(None, db, dir.clone());

        let profile = yt
            .detect_browsers()
            .into_iter()
            .next()
            .expect("browser profile");
        let accounts = yt.scan_browser(profile.path).await.unwrap();
        for a in &accounts {
            println!(
                "account: {} {:?} (index {}) in {:?}",
                a.name, a.handle, a.index, a.source
            );
        }
        let wanted = std::env::var("XP_MUSIC_ACCOUNT_NAME")
            .expect("XP_MUSIC_ACCOUNT_NAME with your account name");
        let account = accounts
            .into_iter()
            .find(|a| a.name == wanted)
            .expect("expected account");
        println!("status: {:?}", yt.select_account(account).await.unwrap());

        let t = std::time::Instant::now();
        let pls = yt.library_playlists().await.unwrap();
        println!(
            "playlists ({} ms): {:?}",
            t.elapsed().as_millis(),
            pls.iter().map(|p| &p.title).collect::<Vec<_>>()
        );
        let t = std::time::Instant::now();
        let again = yt.library_playlists().await.unwrap();
        println!("playlists from cache: {} ms", t.elapsed().as_millis());
        assert_eq!(pls, again);

        let mine = pls
            .iter()
            .find(|p| p.title == "bachata")
            .or(pls.get(1))
            .unwrap();
        let detail = yt.playlist(mine.id.clone()).await.unwrap();
        println!(
            "playlist {}: {} tracks, owned={}",
            detail.summary.title,
            detail.tracks.items.len(),
            detail.owned
        );

        let liked = yt.liked_songs().await.unwrap();
        println!("liked: {}", liked.items.len());
        let hist = yt.history().await.unwrap();
        println!(
            "history: {:?}",
            hist.iter()
                .map(|s| (&s.title, s.items.len()))
                .collect::<Vec<_>>()
        );
        let home = yt.home(None).await.unwrap();
        println!(
            "home: {:?} · more: {}",
            home.items.iter().map(|s| &s.title).collect::<Vec<_>>(),
            home.continuation.is_some()
        );
        if let Some(c) = home.continuation {
            let more = yt.home(Some(c)).await.unwrap();
            println!(
                "home (2nd page): {:?}",
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
                "search {f:?}: {} results · more: {}",
                r.items.len(),
                r.continuation.is_some()
            );
            if f == SearchFilter::Songs
                && let Some(c) = r.continuation
            {
                let more = yt.more_search(c, f).await.unwrap();
                println!("  2nd page: {}", more.items.len());
            }
        }
        println!(
            "suggestions: {:?}",
            yt.suggestions("romeo".into()).await.unwrap()
        );
        let radio = yt.radio(TrackId("yQd4xQBlBa0".into())).await.unwrap();
        println!("radio: {} tracks", radio.len());
        let album = yt
            .album(BrowseId("MPREb_WixYTw0z5H2".into()))
            .await
            .unwrap();
        println!(
            "album: {} ({} tracks)",
            album.summary.title,
            album.tracks.len()
        );
        let artist = yt
            .artist(BrowseId("UCDaQK4nrsvz27YzufrhJfgg".into()))
            .await
            .unwrap();
        println!(
            "artist: {} · {} top · {} sections",
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

        // Renewal from the browser (as if it had expired).
        yt.refresh_session().await.unwrap();
        println!("after renewing: {:?}", yt.status());
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod dev_setup {
    /// Leaves the app signed in with the given account (the app's real data):
    /// `cargo test --lib -- --ignored prepare_real_session`
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn prepare_real_session() {
        use super::*;
        let dirs = crate::state::AppDirs::resolve().unwrap();
        let db = Arc::new(Db::open(&dirs.data.join("xp-music.db")).unwrap());
        let yt = YtService::new(None, db, dirs.data.clone());
        let profile = yt.detect_browsers().into_iter().next().unwrap();
        let wanted = std::env::var("XP_MUSIC_ACCOUNT_NAME")
            .expect("XP_MUSIC_ACCOUNT_NAME with your account name");
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
    //! Real network, read-only (account menu) with the session stored in the app:
    //! `cargo test --lib bench_accounts -- --ignored --nocapture`.
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn bench_accounts() {
        let dirs = crate::state::AppDirs::resolve().unwrap();
        let store = SessionStore::new(&dirs.data);
        let cookie = store.load().expect("stored session").cookie_header;
        for round in 0..3 {
            let t = std::time::Instant::now();
            let accounts = YtService::accounts_in(&cookie).await.unwrap();
            println!(
                "round {round}: {} accounts in {} ms",
                accounts.len(),
                t.elapsed().as_millis()
            );
        }
    }
}
