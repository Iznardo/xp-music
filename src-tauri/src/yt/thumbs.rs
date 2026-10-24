//! Cover art cached on disk, served to the UI through the `thumb` protocol
//! (`http://thumb.localhost/<encoded url>` on Windows) (FR-003-17).

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use tauri::http::{Request, Response, StatusCode};
use url::Url;

use crate::storage::Db;

/// Cache cap; once exceeded, the least recently used entries are deleted down to 90 %.
const MAX_BYTES: i64 = 200 * 1024 * 1024;
/// Maximum size of one image.
const MAX_IMAGE: usize = 4 * 1024 * 1024;
/// Maximum number of redirects followed (each hop is validated again).
const MAX_REDIRECTS: usize = 5;

pub struct ThumbCache {
    dir: PathBuf,
    db: Arc<Db>,
    http: Client,
}

impl ThumbCache {
    pub fn new(dir: PathBuf, db: Arc<Db>) -> Arc<Self> {
        let _ = std::fs::create_dir_all(&dir);
        // A redirect to a host outside the allowlist would turn it into an open proxy.
        let redirects = Policy::custom(|attempt| {
            if attempt.previous().len() < MAX_REDIRECTS && permitted(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        });
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(redirects)
            .build()
            .expect("HTTP client");
        Arc::new(Self { dir, db, http })
    }

    /// Handles a protocol request (blocking: call outside the UI thread).
    pub fn handle(&self, req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
        let path = req.uri().path().trim_start_matches('/');
        let Some((url, parsed)) = percent_decode(path).and_then(|u| allowed(&u).map(|p| (u, p)))
        else {
            return status(StatusCode::BAD_REQUEST);
        };
        match self.get(&url, parsed) {
            Some((mime, bytes)) => Response::builder()
                .header("Content-Type", mime)
                .header("Cache-Control", "max-age=31536000, immutable")
                .header("Access-Control-Allow-Origin", "*")
                .body(bytes)
                .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR)),
            None => status(StatusCode::NOT_FOUND),
        }
    }

    /// `url` is the cache key; `parsed` is what is actually requested (the validated
    /// host must be the same one reqwest connects to).
    fn get(&self, url: &str, parsed: Url) -> Option<(String, Vec<u8>)> {
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

        let resp = self.http.get(parsed).send().ok()?;
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
        .expect("empty response")
}

/// Only Google/YouTube images (avoids using the protocol as an open proxy).
/// It is parsed with the same parser reqwest uses: splitting the text by hand let
/// `https://evil.com#.ytimg.com` or `https://evil.com\x.ytimg.com` through, which
/// reqwest sends to `evil.com`.
fn allowed(url: &str) -> Option<Url> {
    let parsed = Url::parse(url).ok()?;
    // Cover art never has a fragment; rejecting it closes the door to `#` tricks.
    (parsed.fragment().is_none() && permitted(&parsed)).then_some(parsed)
}

fn permitted(url: &Url) -> bool {
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let path = url.path();
    // Covers of the automatic lists: mixes (`/image/radioart`, `/image/mixart`…)
    // and those of "Liked Music" and similar (fixed gstatic images). Only those paths.
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

/// Stable hash (file names must not change between versions).
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(url: &str) -> bool {
        allowed(url).is_some()
    }

    #[test]
    fn only_google_hosts() {
        assert!(ok("https://lh3.googleusercontent.com/abc=w120-h120"));
        assert!(ok("https://i.ytimg.com/vi/x/hqdefault.jpg"));
        assert!(ok("https://yt3.ggpht.com/x"));
        assert!(!ok("https://evil.com/lh3.googleusercontent.com"));
        assert!(!ok("https://googleusercontent.com.evil.com/x"));
        assert!(!ok("http://lh3.googleusercontent.com/x"));
        // Automatic lists: image paths only.
        assert!(ok("https://music.youtube.com/image/radioart?r=CjMK"));
        assert!(ok(
            "https://www.gstatic.com/youtube/media/ytm/images/pbg/liked-songs.png"
        ));
        assert!(!ok("https://music.youtube.com/youtubei/v1/browse"));
        assert!(!ok("https://www.gstatic.com/other/x.png"));
    }

    #[test]
    fn validated_host_is_the_requested_one() {
        // With the hand-rolled split, these passed the filter and reqwest went to evil.com.
        for url in [
            "https://evil.com#.ytimg.com",
            "https://evil.com#/i.ytimg.com/x",
            "https://evil.com\\x.ytimg.com",
            "https://evil.com\\p\\i.ytimg.com/vi/x",
            "https://i.ytimg.com@evil.com/x",
            "https://evil.com:443\\.ytimg.com",
            "https://music.youtube.com\\image/x#.ytimg.com",
        ] {
            assert!(!ok(url), "{url}");
        }
        // Userinfo, port and fragment are not needed for cover art.
        assert!(!ok("https://evil.com@i.ytimg.com/x"));
        assert!(!ok("https://i.ytimg.com:8443/x"));
        assert!(!ok("https://i.ytimg.com/vi/x/hqdefault.jpg#a"));
        // What is requested is the host that was validated.
        let u = allowed("https://i.ytimg.com/vi/x/hqdefault.jpg").unwrap();
        assert_eq!(u.host_str(), Some("i.ytimg.com"));
    }

    #[test]
    fn redirects_only_to_allowed_hosts() {
        let ok = |u: &str| permitted(&Url::parse(u).unwrap());
        assert!(ok("https://lh3.googleusercontent.com/a"));
        assert!(!ok("https://evil.com/a"));
        assert!(!ok("http://i.ytimg.com/a"));
        assert!(!ok("https://127.0.0.1/a"));
    }

    #[test]
    fn decodes_url() {
        assert_eq!(
            percent_decode("https%3A%2F%2Flh3.googleusercontent.com%2Fa%3Dw60-h60").as_deref(),
            Some("https://lh3.googleusercontent.com/a=w60-h60")
        );
        assert!(percent_decode("%zz").is_none());
    }

    #[test]
    fn stable_hash() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }
}
