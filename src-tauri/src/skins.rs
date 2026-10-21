//! Máscaras (skins) de Windows Media Player: los `.wmz` originales que el usuario
//! pone en su carpeta de máscaras. Aquí solo se listan y se sirven; el motor que
//! los interpreta corre en la WebView, dentro de un iframe aislado
//! (`src/skin-frame/`), servido por el protocolo `skin`
//! (`http://skin.localhost/…` en Windows):
//!
//! - `/frame/<archivo>`: el motor, embebido en el ejecutable.
//! - `/wmz/<id>`: el `.wmz` tal cual (id = ruta relativa dentro de la carpeta).
//!
//! El marco no tiene acceso a la app (sandbox sin mismo origen) y su CSP solo le
//! deja cargar el motor, leer el `.wmz` y mostrar portadas.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use specta::Type;
use tauri::http::{Request, Response, StatusCode};

use crate::error::{AppError, AppResult};

/// Tope de tamaño de un `.wmz` (los más grandes de la colección rondan 10 MB).
const MAX_WMZ: u64 = 64 * 1024 * 1024;
/// Profundidad máxima de subcarpetas al buscar máscaras.
const MAX_DEPTH: usize = 4;

const CSP: &str = "default-src 'none'; script-src http://skin.localhost 'unsafe-eval'; \
style-src 'unsafe-inline'; img-src blob: data: http://thumb.localhost; connect-src http://skin.localhost";

/// El motor del marco, embebido en el ejecutable.
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
    /// Ruta relativa dentro de la carpeta de máscaras, con `/`.
    pub id: String,
    /// Nombre del archivo sin `.wmz`.
    pub name: String,
    pub bytes: u32,
}

/// Carpeta de máscaras del usuario.
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

/// Las máscaras de una carpeta (y sus subcarpetas), por nombre.
pub fn list_in(base: &Path) -> Vec<SkinInfo> {
    let mut out = Vec::new();
    walk(base, base, 0, &mut out);
    out.sort_by_key(|s| s.name.to_lowercase());
    out
}

/// Ruta de una máscara a partir de su id, sin salir de la carpeta.
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
    tracing::info!(dir = %dir.display(), count = list.len(), "máscaras encontradas");
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub fn skins_open_dir() -> AppResult<()> {
    std::process::Command::new("explorer.exe")
        .arg(dir()?)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::internal(e.to_string()))
}

/// Atiende el protocolo `skin` (bloqueante: llamar fuera del hilo de la UI).
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
        // El marco no tiene origen (sandbox): sin esto no podría cargar sus módulos.
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_else(|_| status(StatusCode::INTERNAL_SERVER_ERROR))
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .body(Vec::new())
        .expect("respuesta vacía")
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
    fn lista_wmz_en_subcarpetas() {
        let d = temp_dir("lista");
        std::fs::write(d.join("Azul.wmz"), b"x").unwrap();
        std::fs::write(d.join("pack").join("verde.WMZ"), b"xy").unwrap();
        std::fs::write(d.join("nota.txt"), b"x").unwrap();
        let l = list_in(&d);
        let ids: Vec<_> = l.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["Azul.wmz", "pack/verde.WMZ"]);
        assert_eq!(l[1].name, "verde");
        assert_eq!(l[1].bytes, 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn no_sale_de_la_carpeta() {
        let d = temp_dir("ruta");
        std::fs::write(d.join("pack").join("a.wmz"), b"x").unwrap();
        std::fs::write(d.join("secreto.txt"), b"x").unwrap();
        assert!(resolve(&d, "pack/a.wmz").is_some());
        assert!(resolve(&d, "../a.wmz").is_none());
        assert!(resolve(&d, "pack/../secreto.txt").is_none());
        assert!(resolve(&d, "secreto.txt").is_none());
        assert!(resolve(&d, "C:/Windows/win.ini").is_none());
        assert!(resolve(&d, "/etc/x.wmz").is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn el_marco_va_embebido_con_su_csp() {
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
