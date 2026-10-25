//! YouTube Music session: import from the browser and encrypted storage.
//!
//! - Cookies are read from the profile of a Firefox-like browser (Zen, Firefox,
//!   LibreWolf…) by copying `cookies.sqlite` **and its WAL** (recently rotated
//!   cookies only live there) and keeping **a single container**: mixing
//!   containers produces an invalid session.
//! - On Windows the session is stored encrypted with DPAPI (tied to the user) in
//!   `%APPDATA%\xp-music\session.bin`. On Linux it is stored in Secret Service,
//!   isolated by the SessionStore path; on macOS, in the Keychain with the same key;
//!   never in plain text on disk (principle VI).
//!   The Windows Credential Manager is not used because of its 2.5 KB limit.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult, ErrorCode};

/// What gets stored (encrypted).
#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    /// Value of the `Cookie` header for music.youtube.com.
    pub cookie_header: String,
    /// Same cookies (+ google.com) in Netscape format, for yt-dlp.
    pub netscape: String,
    /// Account index within the session (`X-Goog-AuthUser`).
    pub auth_user: u32,
    /// Origin, to re-import automatically when it expires. `None` = pasted by hand.
    pub source: Option<CookieSource>,
    pub account_name: Option<String>,
    #[serde(default)]
    pub account_handle: Option<String>,
    #[serde(default)]
    pub account_photo: Option<String>,
    pub imported_at: u64,
    /// Session started in the app's own window (renewed through it).
    #[serde(default)]
    pub app_login: bool,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the cookies.
        f.debug_struct("Session")
            .field("auth_user", &self.auth_user)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CookieSource {
    pub profile: PathBuf,
    /// Firefox `originAttributes`: `""` = no container, `"^userContextId=1"`…
    pub container: String,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ------------------------------------------------------------------ profiles

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    pub browser: String,
    pub name: String,
    pub path: String,
}

/// Profiles of Firefox-based browsers that have a cookie database.
pub fn detect_profiles() -> Vec<BrowserProfile> {
    #[cfg(target_os = "linux")]
    let browsers = {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };
        linux_profile_dirs(&home)
    };
    #[cfg(not(target_os = "linux"))]
    let browsers = {
        let Some(appdata) = dirs::config_dir() else {
            return Vec::new();
        };
        // macOS: ~/Library/Application Support/Firefox; Windows: %APPDATA%\Mozilla\Firefox.
        let firefox = if cfg!(target_os = "macos") {
            appdata.join("Firefox")
        } else {
            appdata.join("Mozilla").join("Firefox")
        };
        [
            ("Zen", appdata.join("zen").join("Profiles")),
            ("Firefox", firefox.join("Profiles")),
            ("LibreWolf", appdata.join("librewolf").join("Profiles")),
            ("Floorp", appdata.join("Floorp").join("Profiles")),
            ("Waterfox", appdata.join("Waterfox").join("Profiles")),
        ]
    };
    detect_profiles_in(browsers)
}

#[cfg(target_os = "linux")]
fn linux_profile_dirs(home: &Path) -> Vec<(&'static str, PathBuf)> {
    [
        ("Zen", ".zen"),
        ("Firefox", ".mozilla/firefox"),
        ("Firefox", ".config/mozilla/firefox"),
        ("LibreWolf", ".librewolf"),
        ("Floorp", ".floorp"),
        ("Waterfox", ".waterfox"),
        ("Zen", ".var/app/app.zen_browser.zen/.zen"),
        ("Firefox", ".var/app/org.mozilla.firefox/.mozilla/firefox"),
        (
            "LibreWolf",
            ".var/app/io.gitlab.librewolf-community/.librewolf",
        ),
        ("Floorp", ".var/app/one.ablaze.floorp/.floorp"),
        ("Waterfox", ".var/app/net.waterfox.waterfox/.waterfox"),
        ("Firefox", "snap/firefox/common/.mozilla/firefox"),
    ]
    .into_iter()
    .map(|(browser, dir)| (browser, home.join(dir)))
    .collect()
}

fn detect_profiles_in(
    browsers: impl IntoIterator<Item = (&'static str, PathBuf)>,
) -> Vec<BrowserProfile> {
    let mut out = Vec::new();
    for (browser, dir) in browsers {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut found: Vec<(std::time::SystemTime, BrowserProfile)> = entries
            .flatten()
            .filter(|e| e.path().join("cookies.sqlite").is_file())
            .map(|e| {
                let path = e.path();
                let folder = e.file_name().to_string_lossy().into_owned();
                let name = folder
                    .split_once('.')
                    .map_or(folder.clone(), |(_, n)| n.to_owned());
                let modified = std::fs::metadata(path.join("cookies.sqlite"))
                    .and_then(|m| m.modified())
                    .unwrap_or(UNIX_EPOCH);
                (
                    modified,
                    BrowserProfile {
                        browser: browser.to_owned(),
                        name,
                        path: path.display().to_string(),
                    },
                )
            })
            .collect();
        // Most recently used first.
        found.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
        out.extend(found.into_iter().map(|(_, p)| p));
    }
    out
}

// ------------------------------------------------------------------ cookies

/// Copies `cookies.sqlite` (+ WAL) to a temporary folder and opens it.
struct CookieDb {
    dir: PathBuf,
    conn: Connection,
}

impl CookieDb {
    fn open(profile: &Path) -> AppResult<Self> {
        let src = profile.join("cookies.sqlite");
        if !src.is_file() {
            return Err(AppError::new(
                ErrorCode::NotFound,
                tr!(
                    "That profile has no cookie database.",
                    "Ese perfil no tiene base de cookies."
                ),
            ));
        }
        let dir = std::env::temp_dir().join(format!(
            "xp-music-cookies-{}-{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            // Don't reuse someone else's directory or expose the cookies to other users.
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&dir)
                .map_err(|e| AppError::internal(e.to_string()))?;
        }
        #[cfg(not(unix))]
        std::fs::create_dir_all(&dir).map_err(|e| AppError::internal(e.to_string()))?;
        let copy = |name: &str| -> std::io::Result<()> {
            let from = profile.join(name);
            if from.is_file() {
                std::fs::copy(&from, dir.join(name))?;
            }
            Ok(())
        };
        let res = copy("cookies.sqlite").and_then(|_| copy("cookies.sqlite-wal"));
        if let Err(e) = res {
            let _ = std::fs::remove_dir_all(&dir);
            tracing::warn!(error = %e, "could not copy the cookie database");
            return Err(AppError::internal(tr!(
                "Could not read the browser's cookie database.",
                "No se pudo leer la base de cookies del navegador."
            )));
        }
        // Read/write so SQLite incorporates the copy's WAL.
        let conn = Connection::open_with_flags(
            dir.join("cookies.sqlite"),
            OpenFlags::SQLITE_OPEN_READ_WRITE,
        )?;
        Ok(Self { dir, conn })
    }
}

impl Drop for CookieDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    /// `originAttributes` (internal identifier).
    pub id: String,
    pub name: String,
}

/// Profile containers that have a signed-in YouTube session.
pub fn logged_in_containers(profile: &Path) -> AppResult<Vec<Container>> {
    let db = CookieDb::open(profile)?;
    let mut stmt = db.conn.prepare(
        "SELECT originAttributes FROM moz_cookies WHERE host = '.youtube.com' AND name = 'LOGIN_INFO'
         GROUP BY originAttributes ORDER BY MAX(lastAccessed) DESC",
    )?;
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let names = container_names(profile);
    Ok(ids
        .into_iter()
        // Only regular containers; per-site isolation (firstPartyDomain…) is discarded.
        .filter(|id| id.is_empty() || id.starts_with("^userContextId="))
        .map(|id| {
            let name = if id.is_empty() {
                tr!("No container", "Sin contenedor")
            } else {
                let n = id.trim_start_matches("^userContextId=");
                names
                    .iter()
                    .find(|(k, _)| k == n)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| tr!("Container {n}", "Contenedor {n}", n = n))
            };
            Container { id, name }
        })
        .collect())
}

/// Names from `containers.json` (the default ones come as l10n keys).
fn container_names(profile: &Path) -> Vec<(String, String)> {
    let Ok(raw) = std::fs::read_to_string(profile.join("containers.json")) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    v.get("identities")
        .and_then(|i| i.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(|i| {
                    let id = i.get("userContextId")?.as_u64()?.to_string();
                    let name = i
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(str::to_owned)
                        .or_else(|| {
                            let key = i.get("l10nId")?.as_str()?;
                            Some(match key {
                                k if k.contains("personal") => tr!("Personal", "Personal"),
                                k if k.contains("work") => tr!("Work", "Trabajo"),
                                k if k.contains("banking") => tr!("Banking", "Banca"),
                                k if k.contains("shopping") => tr!("Shopping", "Compras"),
                                _ => return None,
                            })
                        })?;
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// YouTube/Google cookies of a container → (Cookie header, Netscape).
pub fn read_cookies(source: &CookieSource) -> AppResult<(String, String)> {
    let db = CookieDb::open(&source.profile)?;
    let mut stmt = db.conn.prepare(
        "SELECT host, name, value, path, expiry, isSecure FROM moz_cookies
         WHERE originAttributes = ?1 AND host IN ('.youtube.com', 'youtube.com', 'music.youtube.com', '.google.com')
         ORDER BY host, name",
    )?;
    let rows: Vec<(String, String, String, String, i64, bool)> = stmt
        .query_map([&source.container], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    if !rows
        .iter()
        .any(|r| r.0 == ".youtube.com" && r.1 == "SAPISID")
    {
        return Err(AppError::new(
            ErrorCode::NotAuthenticated,
            tr!(
                "There is no signed-in YouTube session in that browser.",
                "No hay sesión de YouTube iniciada en ese navegador."
            ),
        ));
    }
    let header = rows
        .iter()
        .filter(|r| r.0.ends_with("youtube.com"))
        .map(|r| format!("{}={}", r.1, r.2))
        .collect::<Vec<_>>()
        .join("; ");
    let mut netscape = String::from("# Netscape HTTP Cookie File\n");
    for (host, name, value, path, expiry, secure) in &rows {
        // Recent Firefox stores the expiry in milliseconds.
        let expiry = if *expiry > 100_000_000_000 {
            expiry / 1000
        } else {
            *expiry
        };
        let sub = if host.starts_with('.') {
            "TRUE"
        } else {
            "FALSE"
        };
        let sec = if *secure { "TRUE" } else { "FALSE" };
        netscape.push_str(&format!(
            "{host}\t{sub}\t{path}\t{sec}\t{expiry}\t{name}\t{value}\n"
        ));
    }
    Ok((header, netscape))
}

/// `Cookie` header pasted by hand: `cookie: …`, quotes or line breaks are accepted.
pub fn clean_pasted_cookie(raw: &str) -> AppResult<String> {
    let mut s = raw.trim().trim_matches(['"', '\'']).to_owned();
    if let Some(rest) = s
        .strip_prefix("cookie:")
        .or_else(|| s.strip_prefix("Cookie:"))
    {
        s = rest.trim().to_owned();
    }
    let s = s.replace(['\r', '\n'], "");
    if !s.contains("SAPISID=") {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            tr!(
                "That does not look like the Cookie header of music.youtube.com (SAPISID is missing).",
                "Eso no parece la cabecera Cookie de music.youtube.com (falta SAPISID)."
            ),
        ));
    }
    Ok(s)
}

/// Minimal Netscape file built from a Cookie header (session pasted by hand).
pub fn netscape_from_header(header: &str) -> String {
    let mut out = String::from("# Netscape HTTP Cookie File\n");
    let expiry = now_secs() + 365 * 24 * 3600;
    for kv in header.split(';') {
        if let Some((k, v)) = kv.trim().split_once('=') {
            out.push_str(&format!(
                ".youtube.com\tTRUE\t/\tTRUE\t{expiry}\t{k}\t{v}\n"
            ));
        }
    }
    out
}

// ------------------------------------------------------------------ encrypted storage

/// System store, for the logs.
#[cfg(not(windows))]
const KEYRING: &str = if cfg!(target_os = "macos") {
    "Keychain"
} else {
    "Secret Service"
};

pub struct SessionStore {
    #[cfg(windows)]
    path: PathBuf,
    #[cfg(not(windows))]
    entry: keyring::Result<keyring::Entry>,
}

impl SessionStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            #[cfg(windows)]
            path: data_dir.join("session.bin"),
            #[cfg(not(windows))]
            entry: {
                use sha2::{Digest, Sha256};
                let path = data_dir.join("session.bin");
                // Stable identity, no collisions from paths with non-UTF-8 bytes.
                let hash: String = Sha256::digest(path.as_os_str().as_encoded_bytes())
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                keyring::Entry::new("com.iznardo.xpmusic", &format!("session-{hash}"))
            },
        }
    }

    #[cfg(windows)]
    pub fn load(&self) -> Option<Session> {
        let enc = std::fs::read(&self.path).ok()?;
        let plain = dpapi::unprotect(&enc)
            .map_err(|e| tracing::warn!(error = %e, "could not decrypt the session"))
            .ok()?;
        serde_json::from_slice(&plain)
            .map_err(|e| tracing::warn!(error = %e, "stored session is not valid"))
            .ok()
    }

    #[cfg(windows)]
    pub fn save(&self, s: &Session) -> AppResult<()> {
        let plain = serde_json::to_vec(s)?;
        let enc =
            dpapi::protect(&plain).map_err(|e| AppError::internal(format!("encryption: {e}")))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, enc)
            .and_then(|_| std::fs::rename(&tmp, &self.path))
            .map_err(|e| {
                tracing::error!(error = %e, "could not save the session");
                AppError::new(
                    ErrorCode::Storage,
                    tr!(
                        "Could not save the session.",
                        "No se pudo guardar la sesión."
                    ),
                )
            })
    }

    #[cfg(windows)]
    pub fn clear(&self) -> AppResult<()> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AppError::new(
                ErrorCode::Storage,
                tr!(
                    "Could not delete the session.",
                    "No se pudo borrar la sesión."
                ),
            )),
        }
    }

    #[cfg(not(windows))]
    fn entry(&self) -> AppResult<&keyring::Entry> {
        self.entry.as_ref().map_err(|_| {
            tracing::error!("could not initialize the {KEYRING} entry");
            AppError::new(
                ErrorCode::Storage,
                if cfg!(target_os = "macos") {
                    tr!(
                        "Could not access the Keychain.",
                        "No se pudo acceder al Llavero."
                    )
                } else {
                    tr!(
                        "Could not access Secret Service.",
                        "No se pudo acceder a Secret Service."
                    )
                },
            )
        })
    }

    #[cfg(not(windows))]
    pub fn load(&self) -> Option<Session> {
        let plain = match self.entry().ok()?.get_secret() {
            Ok(plain) => plain,
            Err(keyring::Error::NoEntry) => return None,
            Err(_) => {
                tracing::warn!("could not read the session from {KEYRING}");
                return None;
            }
        };
        serde_json::from_slice(&plain)
            // Do not log the JSON error: it may include cookie values.
            .map_err(|_| tracing::warn!("invalid session in {KEYRING}"))
            .ok()
    }

    #[cfg(not(windows))]
    pub fn save(&self, s: &Session) -> AppResult<()> {
        let plain = serde_json::to_vec(s)?;
        self.entry()?.set_secret(&plain).map_err(|_| {
            tracing::error!("could not save the session in {KEYRING}");
            AppError::new(
                ErrorCode::Storage,
                if cfg!(target_os = "macos") {
                    tr!(
                        "Could not save the session in the Keychain.",
                        "No se pudo guardar la sesión en el Llavero."
                    )
                } else {
                    tr!(
                        "Could not save the session in Secret Service.",
                        "No se pudo guardar la sesión en Secret Service."
                    )
                },
            )
        })
    }

    #[cfg(not(windows))]
    pub fn clear(&self) -> AppResult<()> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => {
                tracing::error!("could not delete the session from {KEYRING}");
                Err(AppError::new(
                    ErrorCode::Storage,
                    if cfg!(target_os = "macos") {
                        tr!(
                            "Could not delete the session from the Keychain.",
                            "No se pudo borrar la sesión del Llavero."
                        )
                    } else {
                        tr!(
                            "Could not delete the session from Secret Service.",
                            "No se pudo borrar la sesión de Secret Service."
                        )
                    },
                ))
            }
        }
    }
}

#[cfg(windows)]
mod dpapi {
    //! Encryption bound to the Windows user (CryptProtectData).
    use std::ptr;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        // SAFETY: Windows returned a valid `cbData`-byte buffer to be freed with LocalFree.
        unsafe {
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            LocalFree(out.pbData as _);
            v
        }
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(data);
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        // SAFETY: pointers are valid during the call; Windows fills in `out`.
        let ok = unsafe {
            CryptProtectData(
                &input,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if ok == 0 {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(take(out))
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        let input = blob(data);
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: ptr::null_mut(),
        };
        // SAFETY: same as in `protect`.
        let ok = unsafe {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if ok == 0 {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(take(out))
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn round_trip() {
            let secret = b"SAPISID=abc; SID=def";
            let enc = super::protect(secret).unwrap();
            assert!(
                !enc.windows(secret.len()).any(|w| w == secret),
                "must not remain in plain text"
            );
            assert_eq!(super::unprotect(&enc).unwrap(), secret);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_pasted_cookie() {
        assert_eq!(
            clean_pasted_cookie("cookie: SAPISID=x; SID=y\n").unwrap(),
            "SAPISID=x; SID=y"
        );
        assert!(clean_pasted_cookie("hello").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn session_encrypted_on_disk() {
        let dir = std::env::temp_dir().join(format!("xp-music-test-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SessionStore::new(&dir);
        let s = test_session();
        store.save(&s).unwrap();
        let raw = std::fs::read(dir.join("session.bin")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("secret123"));
        let back = store.load().unwrap();
        assert_eq!(back.cookie_header, "SAPISID=secret123");
        assert_eq!(back.auth_user, 1);
        assert!(
            !format!("{back:?}").contains("secret"),
            "Debug must not show cookies"
        );
        store.clear().unwrap();
        assert!(store.load().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    fn test_session() -> Session {
        Session {
            cookie_header: "SAPISID=secret123".into(),
            netscape: String::new(),
            auth_user: 1,
            source: None,
            account_name: Some("alex".into()),
            account_handle: None,
            account_photo: None,
            imported_at: 1,
            app_login: false,
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn session_in_keyring_mock() {
        let dir = std::env::temp_dir().join(format!("xp-music-test-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        // Per-instance injection: don't change keyring's global builder.
        let mut store = SessionStore::new(&dir);
        store.entry = Ok(keyring::Entry::new_with_credential(Box::new(
            keyring::mock::MockCredential::default(),
        )));
        let entry = store.entry().unwrap();
        let mock = entry
            .get_credential()
            .downcast_ref::<keyring::mock::MockCredential>()
            .unwrap();
        assert!(store.load().is_none());
        let mut s = test_session();
        s.netscape = "long-cookie".repeat(1024);
        store.save(&s).unwrap();
        assert_eq!(entry.get_secret().unwrap(), serde_json::to_vec(&s).unwrap());
        let back = store.load().unwrap();
        assert_eq!(back.cookie_header, s.cookie_header);
        assert_eq!(back.netscape, s.netscape);
        assert_eq!(back.auth_user, s.auth_user);
        assert!(!format!("{back:?}").contains("secret"));

        mock.set_error(keyring::Error::NoStorageAccess(Box::new(
            std::io::Error::other("storage locked: secret123"),
        )));
        let err = store.save(&s).unwrap_err();
        assert_eq!(err.code, ErrorCode::Storage);
        assert!(!err.message.contains("secret123"));
        mock.set_error(keyring::Error::PlatformFailure(Box::new(
            std::io::Error::other("no D-Bus"),
        )));
        assert!(store.load().is_none());
        mock.set_error(keyring::Error::NoStorageAccess(Box::new(
            std::io::Error::other("storage locked"),
        )));
        assert_eq!(store.clear().unwrap_err().code, ErrorCode::Storage);
        assert!(
            store.load().is_some(),
            "must not pretend the credential was deleted"
        );
        entry.set_secret(br#"{"auth_user":"secret123"}"#).unwrap();
        assert!(store.load().is_none(), "reject invalid session JSON");
        store.clear().unwrap();
        store.clear().unwrap(); // Deleting a missing entry is harmless.
        assert!(matches!(entry.get_secret(), Err(keyring::Error::NoEntry)));
        assert!(store.load().is_none());

        let unavailable = SessionStore {
            entry: Err(keyring::Error::Invalid(
                "username".into(),
                "secret123".into(),
            )),
        };
        assert_eq!(unavailable.save(&s).unwrap_err().code, ErrorCode::Storage);
        assert!(unavailable.load().is_none());
        assert_eq!(unavailable.clear().unwrap_err().code, ErrorCode::Storage);
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Linux: needs D-Bus and an unlocked Secret Service (e.g. gnome-keyring);
    /// macOS: the unlocked login Keychain.
    /// `cargo test --lib session_keyring_real -- --ignored --nocapture`
    #[cfg(not(windows))]
    #[test]
    #[ignore = "needs a real, unlocked credential store"]
    fn session_keyring_real() {
        let dir = std::env::temp_dir().join(format!("xp-music-keyring-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SessionStore::new(&dir);
        let other = SessionStore::new(&dir.join("other-profile"));
        let s = test_session();
        assert!(store.load().is_none());
        store.save(&s).unwrap();
        let reopened = SessionStore::new(&dir);
        let back = reopened.load();
        let isolated = other.load().is_none();
        store.clear().unwrap(); // Clean up before the roundtrip assertions.
        assert!(matches!(
            reopened.entry().unwrap().get_secret(),
            Err(keyring::Error::NoEntry)
        ));
        let back = back.expect("session persisted in Secret Service");
        assert_eq!(back.cookie_header, s.cookie_header);
        assert_eq!(back.auth_user, s.auth_user);
        assert!(isolated, "paths must not share credentials");
        assert!(
            std::fs::read_dir(&dir).unwrap().next().is_none(),
            "session.bin and temporary files must not exist"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(not(windows))]
    #[test]
    fn keyring_users_isolated_by_path() {
        // (service, user) of the native credential.
        let ids = |store: &SessionStore| {
            let cred = store.entry().unwrap().get_credential();
            #[cfg(target_os = "linux")]
            {
                let a = &cred
                    .downcast_ref::<keyring::secret_service::SsCredential>()
                    .unwrap()
                    .attributes;
                (a["service"].clone(), a["username"].clone())
            }
            #[cfg(target_os = "macos")]
            {
                let m = cred
                    .downcast_ref::<keyring::macos::MacCredential>()
                    .unwrap();
                (m.service.clone(), m.account.clone())
            }
        };
        let a = ids(&SessionStore::new(Path::new("/tmp/profile-a")));
        let b = ids(&SessionStore::new(Path::new("/tmp/profile-b")));
        assert_eq!(a.0, "com.iznardo.xpmusic");
        assert_ne!(a.1, b.1);
        assert_eq!(a, ids(&SessionStore::new(Path::new("/tmp/profile-a"))));
    }

    #[cfg(unix)]
    #[test]
    fn copies_cookies_to_private_directory() {
        use std::os::unix::fs::PermissionsExt;
        let profile =
            std::env::temp_dir().join(format!("xp-music-profile-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&profile).unwrap();
        Connection::open(profile.join("cookies.sqlite"))
            .unwrap()
            .execute("CREATE TABLE test (id INTEGER)", [])
            .unwrap();
        let db = CookieDb::open(&profile).unwrap();
        assert_eq!(
            std::fs::metadata(&db.dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        drop(db);
        std::fs::remove_dir_all(profile).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detects_linux_profiles() {
        let home =
            std::env::temp_dir().join(format!("xp-music-profiles-{}", rand::random::<u64>()));
        let roots = linux_profile_dirs(&home);
        for (_, root) in &roots {
            std::fs::create_dir_all(root.join("abc.personal")).unwrap();
            std::fs::write(root.join("abc.personal/cookies.sqlite"), []).unwrap();
            std::fs::create_dir_all(root.join("no-cookies")).unwrap();
        }
        let profiles = detect_profiles_in(roots.clone());
        assert_eq!(profiles.len(), roots.len());
        for ((browser, root), profile) in roots.iter().zip(&profiles) {
            assert_eq!(&profile.browser, browser);
            assert_eq!(profile.name, "personal");
            assert_eq!(
                profile.path,
                root.join("abc.personal").display().to_string()
            );
        }
        std::fs::remove_dir_all(&home).unwrap();
        assert!(detect_profiles_in(linux_profile_dirs(&home)).is_empty());
    }

    /// With the real Zen profile: `cargo test --lib -- --ignored --nocapture real_containers`.
    #[test]
    #[ignore]
    fn real_containers() {
        let profiles = detect_profiles();
        println!(
            "profiles: {:?}",
            profiles
                .iter()
                .map(|p| format!("{} / {}", p.browser, p.name))
                .collect::<Vec<_>>()
        );
        let p = profiles.first().expect("some profile");
        let containers = logged_in_containers(Path::new(&p.path)).unwrap();
        println!("containers with a session: {containers:?}");
        for c in &containers {
            let (header, netscape) = read_cookies(&CookieSource {
                profile: PathBuf::from(&p.path),
                container: c.id.clone(),
            })
            .unwrap();
            println!(
                "  {}: header {} cookies, netscape {} lines",
                c.name,
                header.matches('=').count(),
                netscape.lines().count()
            );
        }
    }
}
