//! Portadas con caché en disco, servidas a la UI por el protocolo `thumb`
//! (`http://thumb.localhost/<url codificada>` en Windows) (FR-003-17).

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use reqwest::blocking::Client;
use tauri::http::{Request, Response, StatusCode};

use crate::storage::Db;

/// Tope de la caché; al pasarlo se borra lo menos usado hasta bajar al 90 %.
const MAX_BYTES: i64 = 200 * 1024 * 1024;
/// Tamaño máximo de una imagen.
const MAX_IMAGE: usize = 4 * 1024 * 1024;

pub struct ThumbCache {
    dir: PathBuf,
    db: Arc<Db>,
    http: Client,
}

impl ThumbCache {
    pub fn new(dir: PathBuf, db: Arc<Db>) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("cliente HTTP");
        Arc::new(Self { dir, db, http })
    }

    /// Atiende una petición del protocolo (bloqueante: llamar fuera del hilo de UI).
    pub fn handle(&self, req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
        let path = req.uri().path().trim_start_matches('/');
        let Some(url) = percent_decode(path).filter(|u| allowed(u)) else {
            return status(StatusCode::BAD_REQUEST);
        };
        match self.get(&url) {
            Some((mime, bytes)) => Response::builder()
                .header("Content-Type", mime)
                .header("Cache-Control", "max-age=31536000, immutable")
                .header("Access-Control-Allow-Origin", "*")
                .body(bytes)
                .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR)),
            None => status(StatusCode::NOT_FOUND),
        }
    }

    fn get(&self, url: &str) -> Option<(String, Vec<u8>)> {
        let name = format!("{:016x}", fnv1a(url));
        let file = self.dir.join(&name);
        let hit: Option<String> = self
            .db
            .conn()
            .query_row(
                "SELECT mime FROM thumb_index WHERE name = ?1 AND url = ?2",
                (&name, url),
                |r| r.get(0),
            )
            .ok();
        if let Some(mime) = hit
            && let Ok(bytes) = std::fs::read(&file)
        {
            let _ = self.db.conn().execute(
                "UPDATE thumb_index SET last_access = ?1 WHERE name = ?2",
                (crate::yt::session::now_secs() as i64, &name),
            );
            return Some((mime, bytes));
        }

        let resp = self.http.get(url).send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let mime = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .filter(|m| m.starts_with("image/"))?
            .to_owned();
        let bytes = resp.bytes().ok()?.to_vec();
        if bytes.len() > MAX_IMAGE {
            return None;
        }
        if std::fs::File::create(&file)
            .and_then(|mut f| f.write_all(&bytes))
            .is_ok()
        {
            let _ = self.db.conn().execute(
                "INSERT INTO thumb_index (name, url, mime, bytes, last_access) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(name) DO UPDATE SET url = excluded.url, mime = excluded.mime, bytes = excluded.bytes, last_access = excluded.last_access",
                (&name, url, &mime, bytes.len() as i64, crate::yt::session::now_secs() as i64),
            );
            self.prune();
        }
        Some((mime, bytes))
    }

    fn prune(&self) {
        let conn = self.db.conn();
        let total: i64 = conn
            .query_row("SELECT COALESCE(SUM(bytes), 0) FROM thumb_index", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        if total <= MAX_BYTES {
            return;
        }
        let Ok(mut stmt) =
            conn.prepare("SELECT name, bytes FROM thumb_index ORDER BY last_access ASC")
        else {
            return;
        };
        let rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map(|it| it.flatten().collect())
            .unwrap_or_default();
        let mut excess = total - MAX_BYTES * 9 / 10;
        for (name, bytes) in rows {
            if excess <= 0 {
                break;
            }
            let _ = std::fs::remove_file(self.dir.join(&name));
            let _ = conn.execute("DELETE FROM thumb_index WHERE name = ?1", [&name]);
            excess -= bytes;
        }
    }
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .body(Vec::new())
        .expect("respuesta vacía")
}

/// Solo imágenes de Google/YouTube (evita usar el protocolo como proxy abierto).
fn allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?']).next().unwrap_or("");
    let path = &rest[host.len()..];
    // Portadas de las listas automáticas: mixes (`/image/radioart`, `/image/mixart`…)
    // y las de "Liked Music" y similares (imágenes fijas de gstatic). Solo esas rutas.
    if (host == "music.youtube.com" && path.starts_with("/image/"))
        || (host == "www.gstatic.com" && path.starts_with("/youtube/"))
    {
        return true;
    }
    ["googleusercontent.com", "ggpht.com", "ytimg.com"]
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

pub(crate) fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Hash estable (los nombres de archivo no pueden cambiar entre versiones).
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_hosts_de_google() {
        assert!(allowed("https://lh3.googleusercontent.com/abc=w120-h120"));
        assert!(allowed("https://i.ytimg.com/vi/x/hqdefault.jpg"));
        assert!(allowed("https://yt3.ggpht.com/x"));
        assert!(!allowed("https://evil.com/lh3.googleusercontent.com"));
        assert!(!allowed("https://googleusercontent.com.evil.com/x"));
        assert!(!allowed("http://lh3.googleusercontent.com/x"));
        // Listas automáticas: solo las rutas de imágenes.
        assert!(allowed("https://music.youtube.com/image/radioart?r=CjMK"));
        assert!(allowed(
            "https://www.gstatic.com/youtube/media/ytm/images/pbg/liked-songs.png"
        ));
        assert!(!allowed("https://music.youtube.com/youtubei/v1/browse"));
        assert!(!allowed("https://www.gstatic.com/other/x.png"));
    }

    #[test]
    fn decodifica_url() {
        assert_eq!(
            percent_decode("https%3A%2F%2Flh3.googleusercontent.com%2Fa%3Dw60-h60").as_deref(),
            Some("https://lh3.googleusercontent.com/a=w60-h60")
        );
        assert!(percent_decode("%zz").is_none());
    }

    #[test]
    fn hash_estable() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }
}
