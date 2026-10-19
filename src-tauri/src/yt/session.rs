//! Sesión de YouTube Music: importación desde el navegador y almacenamiento cifrado.
//!
//! - Las cookies se leen del perfil de un navegador tipo Firefox (Zen, Firefox,
//!   LibreWolf…) copiando `cookies.sqlite` **y su WAL** (las cookies rotadas hace
//!   poco solo están ahí) y quedándose con **un único contenedor**: mezclar
//!   contenedores produce una sesión inválida.
//! - La sesión se guarda cifrada con DPAPI (ligada al usuario de Windows) en
//!   `%APPDATA%\xp-music\session.bin`; nunca en claro (principio VI). No se usa el
//!   Administrador de credenciales porque la cookie supera su límite de 2,5 KB.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::{AppError, AppResult, ErrorCode};

/// Lo que se guarda (cifrado).
#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    /// Valor de la cabecera `Cookie` para music.youtube.com.
    pub cookie_header: String,
    /// Mismas cookies (+ google.com) en formato Netscape, para yt-dlp.
    pub netscape: String,
    /// Índice de cuenta dentro de la sesión (`X-Goog-AuthUser`).
    pub auth_user: u32,
    /// Origen, para reimportar sola cuando caduque. `None` = pegada a mano.
    pub source: Option<CookieSource>,
    pub account_name: Option<String>,
    #[serde(default)]
    pub account_handle: Option<String>,
    #[serde(default)]
    pub account_photo: Option<String>,
    pub imported_at: u64,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Nunca imprimir las cookies.
        f.debug_struct("Session")
            .field("auth_user", &self.auth_user)
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CookieSource {
    pub profile: PathBuf,
    /// `originAttributes` de Firefox: `""` = sin contenedor, `"^userContextId=1"`…
    pub container: String,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ------------------------------------------------------------------ perfiles

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserProfile {
    pub browser: String,
    pub name: String,
    pub path: String,
}

/// Perfiles de navegadores basados en Firefox con base de cookies.
pub fn detect_profiles() -> Vec<BrowserProfile> {
    let Some(appdata) = dirs::config_dir() else {
        return Vec::new();
    };
    let browsers = [
        ("Zen", appdata.join("zen").join("Profiles")),
        (
            "Firefox",
            appdata.join("Mozilla").join("Firefox").join("Profiles"),
        ),
        ("LibreWolf", appdata.join("librewolf").join("Profiles")),
        ("Floorp", appdata.join("Floorp").join("Profiles")),
        ("Waterfox", appdata.join("Waterfox").join("Profiles")),
    ];
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
        // El más usado recientemente primero.
        found.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
        out.extend(found.into_iter().map(|(_, p)| p));
    }
    out
}

// ------------------------------------------------------------------ cookies

/// Copia `cookies.sqlite` (+ WAL) a una carpeta temporal y la abre.
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
                "Ese perfil no tiene base de cookies.",
            ));
        }
        let dir = std::env::temp_dir().join(format!(
            "xp-music-cookies-{}-{}",
            std::process::id(),
            rand::random::<u32>()
        ));
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
            tracing::warn!(error = %e, "no se pudo copiar la base de cookies");
            return Err(AppError::internal(
                "No se pudo leer la base de cookies del navegador.",
            ));
        }
        // Lectura/escritura para que SQLite incorpore el WAL de la copia.
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
    /// `originAttributes` (identificador interno).
    pub id: String,
    pub name: String,
}

/// Contenedores del perfil que tienen sesión de YouTube iniciada.
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
        // Solo contenedores normales; se descartan aislamientos por sitio (firstPartyDomain…).
        .filter(|id| id.is_empty() || id.starts_with("^userContextId="))
        .map(|id| {
            let name = if id.is_empty() {
                "Sin contenedor".to_owned()
            } else {
                let n = id.trim_start_matches("^userContextId=");
                names
                    .iter()
                    .find(|(k, _)| k == n)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_else(|| format!("Contenedor {n}"))
            };
            Container { id, name }
        })
        .collect())
}

/// Nombres de `containers.json` (los predeterminados vienen como claves l10n).
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
                            Some(
                                match key {
                                    k if k.contains("personal") => "Personal",
                                    k if k.contains("work") => "Trabajo",
                                    k if k.contains("banking") => "Banca",
                                    k if k.contains("shopping") => "Compras",
                                    _ => return None,
                                }
                                .to_owned(),
                            )
                        })?;
                    Some((id, name))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Cookies de YouTube/Google de un contenedor → (cabecera Cookie, Netscape).
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
            "No hay sesión de YouTube iniciada en ese navegador.",
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
        // Firefox reciente guarda la caducidad en milisegundos.
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

/// Cabecera `Cookie` pegada a mano: se aceptan `cookie: …`, comillas o saltos de línea.
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
            "Eso no parece la cabecera Cookie de music.youtube.com (falta SAPISID).",
        ));
    }
    Ok(s)
}

/// Netscape mínimo a partir de una cabecera Cookie (sesión pegada a mano).
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

// ------------------------------------------------------------------ almacenamiento cifrado

pub struct SessionStore {
    path: PathBuf,
}

impl SessionStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join("session.bin"),
        }
    }

    pub fn load(&self) -> Option<Session> {
        let enc = std::fs::read(&self.path).ok()?;
        let plain = dpapi::unprotect(&enc)
            .map_err(|e| tracing::warn!(error = %e, "no se pudo descifrar la sesión"))
            .ok()?;
        serde_json::from_slice(&plain)
            .map_err(|e| tracing::warn!(error = %e, "sesión guardada no válida"))
            .ok()
    }

    pub fn save(&self, s: &Session) -> AppResult<()> {
        let plain = serde_json::to_vec(s)?;
        let enc =
            dpapi::protect(&plain).map_err(|e| AppError::internal(format!("cifrado: {e}")))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, enc)
            .and_then(|_| std::fs::rename(&tmp, &self.path))
            .map_err(|e| {
                tracing::error!(error = %e, "no se pudo guardar la sesión");
                AppError::new(ErrorCode::Storage, "No se pudo guardar la sesión.")
            })
    }

    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

mod dpapi {
    //! Cifrado ligado al usuario de Windows (CryptProtectData).
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
        // SAFETY: Windows devolvió un búfer válido de `cbData` bytes que hay que liberar con LocalFree.
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
        // SAFETY: punteros válidos durante la llamada; `out` lo rellena Windows.
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
        // SAFETY: igual que en `protect`.
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
        fn ida_y_vuelta() {
            let secret = b"SAPISID=abc; SID=def";
            let enc = super::protect(secret).unwrap();
            assert!(
                !enc.windows(secret.len()).any(|w| w == secret),
                "no debe quedar en claro"
            );
            assert_eq!(super::unprotect(&enc).unwrap(), secret);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limpia_cookie_pegada() {
        assert_eq!(
            clean_pasted_cookie("cookie: SAPISID=x; SID=y\n").unwrap(),
            "SAPISID=x; SID=y"
        );
        assert!(clean_pasted_cookie("hola").is_err());
    }

    #[test]
    fn sesion_cifrada_en_disco() {
        let dir = std::env::temp_dir().join(format!("xp-music-test-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = SessionStore::new(&dir);
        let s = Session {
            cookie_header: "SAPISID=secreto123".into(),
            netscape: String::new(),
            auth_user: 1,
            source: None,
            account_name: Some("chema".into()),
            account_handle: None,
            account_photo: None,
            imported_at: 1,
        };
        store.save(&s).unwrap();
        let raw = std::fs::read(dir.join("session.bin")).unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("secreto123"));
        let back = store.load().unwrap();
        assert_eq!(back.cookie_header, "SAPISID=secreto123");
        assert_eq!(back.auth_user, 1);
        assert!(
            !format!("{back:?}").contains("secreto"),
            "Debug no debe mostrar cookies"
        );
        store.clear();
        assert!(store.load().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Con el perfil real de Zen: `cargo test --lib -- --ignored --nocapture contenedores_reales`.
    #[test]
    #[ignore]
    fn contenedores_reales() {
        let profiles = detect_profiles();
        println!(
            "perfiles: {:?}",
            profiles
                .iter()
                .map(|p| format!("{} / {}", p.browser, p.name))
                .collect::<Vec<_>>()
        );
        let p = profiles.first().expect("algún perfil");
        let containers = logged_in_containers(Path::new(&p.path)).unwrap();
        println!("contenedores con sesión: {containers:?}");
        for c in &containers {
            let (header, netscape) = read_cookies(&CookieSource {
                profile: PathBuf::from(&p.path),
                container: c.id.clone(),
            })
            .unwrap();
            println!(
                "  {}: cabecera {} cookies, netscape {} líneas",
                c.name,
                header.matches('=').count(),
                netscape.lines().count()
            );
        }
    }
}
