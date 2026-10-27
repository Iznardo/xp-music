//! Windows Media Player skins: the original `.wmz` files the user puts in their
//! skins folder. Here they are only listed and served; the engine that interprets
//! them runs in the WebView, inside an isolated iframe (`src/skin-frame/`),
//! served by the `skin` protocol (`http://skin.localhost/…` on Windows):
//!
//! - `/frame/<file>`: the engine, embedded in the executable.
//! - `/wmz/<id>`: the `.wmz` as is (id = relative path inside the folder).
//!
//! The frame has no access to the app (sandbox without same origin) and its CSP only
//! lets it load the engine, read the `.wmz` and show thumbnails.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use specta::Type;
use tauri::http::{Request, Response, StatusCode};

use crate::error::{AppError, AppResult};

/// Size cap for a `.wmz` (the largest in the collection are around 10 MB).
const MAX_WMZ: u64 = 64 * 1024 * 1024;
/// Maximum subfolder depth when searching for skins.
const MAX_DEPTH: usize = 4;

const CSP: &str = "default-src 'none'; script-src skin: http://skin.localhost 'unsafe-eval'; \
style-src 'unsafe-inline'; img-src blob: data: thumb: http://thumb.localhost cover: http://cover.localhost; connect-src skin: http://skin.localhost";

/// The frame engine, embedded in the executable.
const FRAME: &[(&str, &str)] = &[
    (
        "index.html",
        include_str!("../../src/skin-frame/index.html"),
    ),
    ("main.js", include_str!("../../src/skin-frame/main.js")),
    ("zip.js", include_str!("../../src/skin-frame/zip.js")),
    ("wms.js", include_str!("../../src/skin-frame/wms.js")),
    ("util.js", include_str!("../../src/skin-frame/util.js")),
    ("images.js", include_str!("../../src/skin-frame/images.js")),
    (
        "elements.js",
        include_str!("../../src/skin-frame/elements.js"),
    ),
    (
        "objects.js",
        include_str!("../../src/skin-frame/objects.js"),
    ),
    (
        "runtime.js",
        include_str!("../../src/skin-frame/runtime.js"),
    ),
];

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SkinInfo {
    /// Relative path inside the skins folder, with `/`.
    pub id: String,
    /// File name without `.wmz`.
    pub name: String,
    pub bytes: u32,
}

/// The user's skins folder.
pub fn dir() -> AppResult<PathBuf> {
    let dirs = crate::state::AppDirs::resolve().map_err(|e| AppError::internal(e.to_string()))?;
    let dir = dirs.data.join("skins");
    std::fs::create_dir_all(&dir).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(dir)
}

fn walk(base: &Path, dir: &Path, depth: usize, out: &mut Vec<SkinInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            if depth < MAX_DEPTH {
                walk(base, &path, depth + 1, out);
            }
            continue;
        }
        let is_wmz = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("wmz"));
        if !is_wmz || meta.len() > MAX_WMZ {
            continue;
        }
        let Ok(rel) = path.strip_prefix(base) else {
            continue;
        };
        let id = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(SkinInfo {
            id,
            name,
            bytes: meta.len() as u32,
        });
    }
}

/// The skins in a folder (and its subfolders), by name.
pub fn list_in(base: &Path) -> Vec<SkinInfo> {
    let mut out = Vec::new();
    walk(base, base, 0, &mut out);
    out.sort_by_key(|s| s.name.to_lowercase());
    out
}

/// Path of a skin from its id, without leaving the folder.
fn resolve(base: &Path, id: &str) -> Option<PathBuf> {
    let rel = Path::new(id);
    if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    let is_wmz = rel
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wmz"));
    if !is_wmz {
        return None;
    }
    let path = base.join(rel);
    let canon = path.canonicalize().ok()?;
    let root = base.canonicalize().ok()?;
    (canon.starts_with(&root) && canon.is_file()).then_some(canon)
}

#[tauri::command]
#[specta::specta]
pub fn skins_list() -> AppResult<Vec<SkinInfo>> {
    let dir = dir()?;
    let list = list_in(&dir);
    tracing::info!(dir = %dir.display(), count = list.len(), "skins found");
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub fn skins_open_dir() -> AppResult<()> {
    crate::os::open_dir(&dir()?)
}

/// Serves the `skin` protocol (blocking: call off the UI thread).
pub fn handle(req: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = req.uri().path();
    if let Some(name) = path.strip_prefix("/frame/") {
        return match FRAME.iter().find(|(n, _)| *n == name) {
            Some((n, body)) => {
                let mime = if n.ends_with(".html") {
                    "text/html; charset=utf-8"
                } else {
                    "text/javascript; charset=utf-8"
                };
                ok(mime, body.as_bytes().to_vec())
            }
            None => status(StatusCode::NOT_FOUND),
        };
    }
    if let Some(id) = path.strip_prefix("/wmz/") {
        let Some(id) = crate::yt::thumbs::percent_decode(id) else {
            return status(StatusCode::BAD_REQUEST);
        };
        let file = dir().ok().and_then(|base| resolve(&base, &id));
        return match file.and_then(|f| {
            let len = f.metadata().ok()?.len();
            (len <= MAX_WMZ).then(|| std::fs::read(f).ok()).flatten()
        }) {
            Some(bytes) => ok("application/octet-stream", bytes),
            None => status(StatusCode::NOT_FOUND),
        };
    }
    status(StatusCode::NOT_FOUND)
}

fn ok(mime: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .header("Content-Type", mime)
        .header("Content-Security-Policy", CSP)
        // The frame has no origin (sandbox): without this it could not load its modules.
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .body(Vec::new())
        .expect("empty response")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("xpm-skins-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("pack")).unwrap();
        d
    }

    #[test]
    fn lists_wmz_in_subfolders() {
        let d = temp_dir("list");
        std::fs::write(d.join("Blue.wmz"), b"x").unwrap();
        std::fs::write(d.join("pack").join("green.WMZ"), b"xy").unwrap();
        std::fs::write(d.join("note.txt"), b"x").unwrap();
        let l = list_in(&d);
        let ids: Vec<_> = l.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["Blue.wmz", "pack/green.WMZ"]);
        assert_eq!(l[1].name, "green");
        assert_eq!(l[1].bytes, 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn does_not_leave_the_folder() {
        let d = temp_dir("path");
        std::fs::write(d.join("pack").join("a.wmz"), b"x").unwrap();
        std::fs::write(d.join("secret.txt"), b"x").unwrap();
        assert!(resolve(&d, "pack/a.wmz").is_some());
        assert!(resolve(&d, "../a.wmz").is_none());
        assert!(resolve(&d, "pack/../secret.txt").is_none());
        assert!(resolve(&d, "secret.txt").is_none());
        assert!(resolve(&d, "C:/Windows/win.ini").is_none());
        assert!(resolve(&d, "/etc/x.wmz").is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn isolated_csp_on_linux_and_windows() {
        for base in ["skin://localhost", "http://skin.localhost"] {
            let req = Request::builder()
                .uri(format!("{base}/frame/index.html"))
                .body(Vec::new())
                .unwrap();
            let res = handle(&req);
            assert_eq!(res.status(), StatusCode::OK);
            let csp = res.headers()["Content-Security-Policy"].to_str().unwrap();
            assert_eq!(
                csp.split("; ").collect::<Vec<_>>(),
                [
                    "default-src 'none'",
                    "script-src skin: http://skin.localhost 'unsafe-eval'",
                    "style-src 'unsafe-inline'",
                    "img-src blob: data: thumb: http://thumb.localhost cover: http://cover.localhost",
                    "connect-src skin: http://skin.localhost",
                ]
            );
            assert_eq!(res.headers()["Access-Control-Allow-Origin"], "*");
        }
    }

    #[test]
    fn frame_is_embedded_with_its_csp() {
        let req = Request::builder()
            .uri("http://skin.localhost/frame/main.js")
            .body(Vec::new())
            .unwrap();
        let res = handle(&req);
        assert_eq!(res.status(), StatusCode::OK);
        assert!(
            res.headers()["Content-Security-Policy"]
                .to_str()
                .unwrap()
                .contains("default-src 'none'")
        );
        let req = Request::builder()
            .uri("http://skin.localhost/frame/../lib.rs")
            .body(Vec::new())
            .unwrap();
        assert_eq!(handle(&req).status(), StatusCode::NOT_FOUND);
    }
}
