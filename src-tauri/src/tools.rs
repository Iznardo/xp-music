//! Herramientas externas y su mantenimiento (specs/007): versiones y
//! actualización automática de yt-dlp, que es lo que más se rompe cuando
//! YouTube cambia algo.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;

use crate::audio::resolver::ToolPaths;
use crate::error::{AppError, AppResult, ErrorCode};

const RELEASES: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";
#[cfg(windows)]
const ASSET: &str = "yt-dlp_win.zip";
#[cfg(not(windows))]
const ASSET: &str = "yt-dlp_linux";
pub const YT_DLP_EXE: &str = if cfg!(windows) {
    "yt-dlp.exe"
} else {
    "yt-dlp_linux"
};

pub static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}
const SUMS: &str = "SHA2-256SUMS";
/// Comprobación automática como mucho una vez al día (FR-007-08).
const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);
/// Tras fallos de resolución, una comprobación forzada como mucho cada hora (FR-007-09).
const FORCED_EVERY: Duration = Duration::from_secs(3600);

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolVersions {
    pub app: String,
    pub yt_dlp: Option<String>,
    pub deno: Option<String>,
    /// La copia de yt-dlp en uso la ha actualizado la propia app.
    pub yt_dlp_updated: bool,
}

/// Carpeta de las herramientas actualizadas por la app.
pub fn bin_dir() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join("xp-music").join("bin"))
}

/// `XP_MUSIC_TOOLS` (p. ej. el paquete de Nix) aporta yt-dlp y deno: los gestiona quien
/// los instala, no la app.
fn managed_externally() -> bool {
    std::env::var_os("XP_MUSIC_TOOLS").is_some()
}

/// yt-dlp actualizado por la app (puede no existir).
pub fn updated_yt_dlp() -> Option<PathBuf> {
    if managed_externally() {
        return None;
    }
    Some(bin_dir()?.join("yt-dlp").join(YT_DLP_EXE))
}

fn no_window(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

fn version_of(exe: &Path) -> Option<String> {
    let out = no_window(Command::new(exe).arg("--version"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let first = s.lines().next()?.trim();
    // "deno 2.9.7 (stable, …)" → "2.9.7"
    Some(
        first
            .strip_prefix("deno ")
            .map_or(first, |r| r.split_whitespace().next().unwrap_or(r))
            .to_owned(),
    )
}

pub fn versions() -> ToolVersions {
    let tools = ToolPaths::locate();
    let updated = updated_yt_dlp();
    ToolVersions {
        app: env!("CARGO_PKG_VERSION").to_owned(),
        yt_dlp: tools.as_ref().and_then(|t| version_of(&t.yt_dlp)),
        deno: tools.as_ref().and_then(|t| version_of(&t.deno)),
        yt_dlp_updated: tools.is_some_and(|t| Some(&t.yt_dlp) == updated.as_ref()),
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

fn http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("xp-music/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(120))
        .build()
        .expect("cliente HTTP")
}

fn net_err(what: &str, e: impl std::fmt::Display) -> AppError {
    tracing::warn!(error = %e, "{what}");
    AppError::new(
        ErrorCode::Network,
        "No se pudo comprobar si hay una versión nueva de yt-dlp.",
    )
}

static LAST: Mutex<Option<Instant>> = Mutex::new(None);

/// Actualiza yt-dlp si hay versión nueva. Devuelve la versión instalada si cambió.
/// `force`: ignora el límite diario (pero no el horario, para no machacar GitHub).
pub fn update_yt_dlp(force: bool) -> AppResult<Option<String>> {
    update_inner(force, false)
}

fn update_inner(force: bool, reinstall: bool) -> AppResult<Option<String>> {
    if managed_externally() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "yt-dlp lo gestiona el paquete del sistema: actualízalo con él.",
        ));
    }
    {
        let mut last = LAST.lock().unwrap();
        let limit = if force { FORCED_EVERY } else { CHECK_EVERY };
        if last.is_some_and(|t| t.elapsed() < limit) {
            return Ok(None);
        }
        *last = Some(Instant::now());
    }
    let client = http();
    let rel: Release = client
        .get(RELEASES)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| net_err("consulta de versiones de yt-dlp", e))?;
    let current = ToolPaths::locate().and_then(|t| version_of(&t.yt_dlp));
    if !reinstall && current.as_deref() == Some(rel.tag_name.as_str()) {
        tracing::info!(version = %rel.tag_name, "yt-dlp ya está al día");
        return Ok(None);
    }
    let url = |name: &str| {
        rel.assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.clone())
    };
    let (Some(zip_url), Some(sums_url)) = (url(ASSET), url(SUMS)) else {
        return Err(AppError::internal(
            "La versión nueva de yt-dlp no trae el paquete esperado.",
        ));
    };
    let sums = client
        .get(sums_url)
        .send()
        .and_then(|r| r.text())
        .map_err(|e| net_err("descarga de sumas", e))?;
    let expected = sums
        .lines()
        .find_map(|l| {
            l.split_once(char::is_whitespace)
                .filter(|(_, n)| n.trim() == ASSET)
                .map(|(h, _)| h.to_ascii_lowercase())
        })
        .ok_or_else(|| AppError::internal("Falta la suma SHA-256 del paquete de yt-dlp."))?;
    let mut zip_bytes = Vec::new();
    client
        .get(zip_url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| net_err("descarga de yt-dlp", e))?
        .read_to_end(&mut zip_bytes)
        .map_err(|e| net_err("descarga de yt-dlp", e))?;
    let got: String = Sha256::digest(&zip_bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got != expected {
        tracing::error!("hash de yt-dlp no coincide; se descarta");
        return Err(AppError::internal(
            "El paquete descargado de yt-dlp no pasó la verificación.",
        ));
    }
    install(&zip_bytes)?;
    tracing::info!(desde = ?current, hasta = %rel.tag_name, "yt-dlp actualizado");
    Ok(Some(rel.tag_name))
}

/// Descomprime en `bin/yt-dlp.new`, comprueba que arranca y sustituye a la anterior.
fn install(zip_bytes: &[u8]) -> AppResult<()> {
    let bin = bin_dir().ok_or_else(|| AppError::internal("Sin carpeta de datos local."))?;
    install_into(zip_bytes, &bin)
}

fn install_into(zip_bytes: &[u8], bin: &Path) -> AppResult<()> {
    let io = |e: std::io::Error| {
        AppError::new(
            ErrorCode::Storage,
            format!("No se pudo instalar yt-dlp: {e}"),
        )
    };
    let staging = bin.join("yt-dlp.new");
    let target = bin.join("yt-dlp");
    let old = bin.join("yt-dlp.old");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(io)?;

    #[cfg(windows)]
    {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))
            .map_err(|e| AppError::internal(format!("Paquete de yt-dlp no válido: {e}")))?;
        for i in 0..archive.len() {
            let mut f = archive
                .by_index(i)
                .map_err(|e| AppError::internal(e.to_string()))?;
            // `enclosed_name` rechaza rutas que saldrían de la carpeta (../).
            let Some(rel) = f.enclosed_name() else {
                continue;
            };
            let out = staging.join(rel);
            if f.is_dir() {
                std::fs::create_dir_all(&out).map_err(io)?;
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            let mut w = std::fs::File::create(&out).map_err(io)?;
            std::io::copy(&mut f, &mut w).map_err(io)?;
        }
    }
    let exe = staging.join(YT_DLP_EXE);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // ponytail: onefile arranca algo más lento; evita bibliotecas Python incompatibles en AppImage.
        std::fs::write(&exe, zip_bytes).map_err(io)?;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
    }
    if version_of(&exe).is_none() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AppError::internal(
            "La versión nueva de yt-dlp no arranca; se mantiene la anterior.",
        ));
    }
    let _ = std::fs::remove_dir_all(&old);
    if target.exists() {
        std::fs::rename(&target, &old).map_err(io)?;
    }
    std::fs::rename(&staging, &target).map_err(io)?;
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

/// Lanza yt-dlp y deno una vez al abrir la app: el primer arranque en frío
/// (antivirus que los analiza, disco) es lo más lento y si no lo paga la primera
/// canción, cuyo arranque rápido solo cubre ~1 min.
pub fn spawn_warmup() {
    std::thread::Builder::new()
        .name("tools-warmup".into())
        .spawn(|| {
            let Some(tools) = ToolPaths::locate() else {
                return;
            };
            let t0 = std::time::Instant::now();
            let _ = version_of(&tools.yt_dlp);
            let _ = version_of(&tools.deno);
            tracing::debug!(
                ms = t0.elapsed().as_millis() as u64,
                "herramientas precalentadas"
            );
        })
        .ok();
}

/// Comprobación diaria en segundo plano al arrancar.
pub fn spawn_daily_check() {
    std::thread::Builder::new()
        .name("ytdlp-update".into())
        .spawn(|| {
            // Que no compita con el arranque ni con la primera canción.
            std::thread::sleep(Duration::from_secs(60));
            if let Err(e) = update_yt_dlp(false) {
                tracing::info!(error = %e, "no se pudo comprobar yt-dlp");
            }
        })
        .ok();
}

// ---------------------------------------------------------------- comandos

/// Versiones de la app y de las herramientas (pestaña "Acerca de").
#[tauri::command]
#[specta::specta]
pub async fn tools_versions() -> ToolVersions {
    tauri::async_runtime::spawn_blocking(versions)
        .await
        .unwrap_or_else(|_| ToolVersions {
            app: env!("CARGO_PKG_VERSION").to_owned(),
            yt_dlp: None,
            deno: None,
            yt_dlp_updated: false,
        })
}

/// Busca ya una versión nueva de yt-dlp. Devuelve la instalada si ha cambiado.
#[tauri::command]
#[specta::specta]
pub async fn tools_update_now() -> AppResult<Option<String>> {
    // Lo pide el usuario: sin límite de frecuencia.
    *LAST.lock().unwrap() = None;
    tauri::async_runtime::spawn_blocking(|| update_yt_dlp(true))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

/// Abre la carpeta de registros en el Explorador.
#[tauri::command]
#[specta::specta]
pub fn tools_open_logs_dir() -> AppResult<()> {
    let dirs = crate::state::AppDirs::resolve().map_err(|e| AppError::internal(e.to_string()))?;
    let opener = if cfg!(windows) {
        "explorer.exe"
    } else {
        "xdg-open"
    };
    Command::new(opener)
        .arg(&dirs.logs)
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::internal(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_de_deno_se_recorta() {
        // Simula la primera línea de `deno --version`.
        let first = "deno 2.9.7 (stable, release, x86_64-pc-windows-msvc)";
        let v = first
            .strip_prefix("deno ")
            .map_or(first, |r| r.split_whitespace().next().unwrap_or(r));
        assert_eq!(v, "2.9.7");
    }

    #[test]
    fn instalar_rechaza_paquete_corrupto() {
        assert!(install(b"esto no es un zip").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn instalar_conserva_ejecutable_y_rechaza_version_rota() {
        let bin =
            std::env::temp_dir().join(format!("xp-music-tools-test-{}", rand::random::<u32>()));
        let package = |script: &str| script.as_bytes().to_vec();
        install_into(&package("#!/bin/sh\necho test-version\n"), &bin).unwrap();
        let exe = bin.join("yt-dlp").join(YT_DLP_EXE);
        assert_eq!(version_of(&exe).as_deref(), Some("test-version"));
        assert!(install_into(&package("#!/missing/interpreter\n"), &bin).is_err());
        assert_eq!(version_of(&exe).as_deref(), Some("test-version"));
        std::fs::remove_dir_all(bin).unwrap();
    }

    /// Descarga real: `cargo test --lib -- --ignored --nocapture actualizar_yt_dlp_real`.
    #[test]
    #[ignore]
    fn actualizar_yt_dlp_real() {
        println!("antes: {:?}", versions());
        println!("resultado: {:?}", update_inner(true, true));
        println!("después: {:?}", versions());
    }
}
