//! External tools and their maintenance (specs/007): versions and
//! automatic updates of yt-dlp, which is what breaks the most when
//! YouTube changes something.

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
/// yt-dlp release file for a system and architecture (`std::env::consts`).
fn asset_name(os: &str, arch: &str) -> Option<&'static str> {
    Some(match (os, arch) {
        ("windows", "x86_64") => "yt-dlp_win.zip",
        ("windows", "aarch64") => "yt-dlp_win_arm64.zip",
        ("linux", "x86_64") => "yt-dlp_linux",
        ("linux", "aarch64") => "yt-dlp_linux_aarch64",
        // A single universal Mach-O for Intel and Apple Silicon.
        ("macos", _) => "yt-dlp_macos",
        _ => return None,
    })
}

/// Name it is installed (and bundled) under, the same on every architecture.
pub const YT_DLP_EXE: &str = if cfg!(windows) {
    "yt-dlp.exe"
} else if cfg!(target_os = "macos") {
    "yt-dlp_macos"
} else {
    "yt-dlp_linux"
};

pub static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir);
}
const SUMS: &str = "SHA2-256SUMS";
/// Automatic check at most once a day (FR-007-08).
const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);
/// After resolution failures, a forced check at most once an hour (FR-007-09).
const FORCED_EVERY: Duration = Duration::from_secs(3600);

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolVersions {
    pub app: String,
    pub yt_dlp: Option<String>,
    pub deno: Option<String>,
    /// The yt-dlp copy in use was updated by the app itself.
    pub yt_dlp_updated: bool,
}

/// Folder of the tools updated by the app.
pub fn bin_dir() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join("xp-music").join("bin"))
}

/// `XP_MUSIC_TOOLS` (e.g. the Nix package) provides yt-dlp and deno: whoever installs
/// them manages them, not the app.
fn managed_externally() -> bool {
    std::env::var_os("XP_MUSIC_TOOLS").is_some()
}

/// yt-dlp updated by the app (may not exist).
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
        .expect("HTTP client")
}

fn net_err(what: &str, e: impl std::fmt::Display) -> AppError {
    tracing::warn!(error = %e, "{what}");
    AppError::new(
        ErrorCode::Network,
        tr!(
            "Could not check for a new yt-dlp version.",
            "No se pudo comprobar si hay una versión nueva de yt-dlp."
        ),
    )
}

static LAST: Mutex<Option<Instant>> = Mutex::new(None);

/// Updates yt-dlp if there is a new version. Returns the installed version if it changed.
/// `force`: ignores the daily limit (but not the hourly one, to avoid hammering GitHub).
pub fn update_yt_dlp(force: bool) -> AppResult<Option<String>> {
    update_inner(force, false)
}

fn update_inner(force: bool, reinstall: bool) -> AppResult<Option<String>> {
    if managed_externally() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            tr!(
                "yt-dlp is managed by the system package: update it with that.",
                "yt-dlp lo gestiona el paquete del sistema: actualízalo con él."
            ),
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
        .map_err(|e| net_err("yt-dlp version query", e))?;
    let current = ToolPaths::locate().and_then(|t| version_of(&t.yt_dlp));
    if !reinstall && current.as_deref() == Some(rel.tag_name.as_str()) {
        tracing::info!(version = %rel.tag_name, "yt-dlp is already up to date");
        return Ok(None);
    }
    let Some(asset) = asset_name(std::env::consts::OS, std::env::consts::ARCH) else {
        return Err(AppError::internal(tr!(
            "There is no updatable yt-dlp for this platform.",
            "No hay yt-dlp actualizable para esta plataforma."
        )));
    };
    let url = |name: &str| {
        rel.assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.clone())
    };
    let (Some(zip_url), Some(sums_url)) = (url(asset), url(SUMS)) else {
        return Err(AppError::internal(tr!(
            "The new yt-dlp release does not include the expected package.",
            "La versión nueva de yt-dlp no trae el paquete esperado."
        )));
    };
    let sums = client
        .get(sums_url)
        .send()
        .and_then(|r| r.text())
        .map_err(|e| net_err("checksum download", e))?;
    let expected = sums
        .lines()
        .find_map(|l| {
            l.split_once(char::is_whitespace)
                .filter(|(_, n)| n.trim() == asset)
                .map(|(h, _)| h.to_ascii_lowercase())
        })
        .ok_or_else(|| {
            AppError::internal(tr!(
                "The SHA-256 checksum of the yt-dlp package is missing.",
                "Falta la suma SHA-256 del paquete de yt-dlp."
            ))
        })?;
    let mut zip_bytes = Vec::new();
    client
        .get(zip_url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| net_err("yt-dlp download", e))?
        .read_to_end(&mut zip_bytes)
        .map_err(|e| net_err("yt-dlp download", e))?;
    let got: String = Sha256::digest(&zip_bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got != expected {
        tracing::error!("yt-dlp hash mismatch; discarding");
        return Err(AppError::internal(tr!(
            "The downloaded yt-dlp package failed verification.",
            "El paquete descargado de yt-dlp no pasó la verificación."
        )));
    }
    install(&zip_bytes)?;
    tracing::info!(from = ?current, to = %rel.tag_name, "yt-dlp updated");
    Ok(Some(rel.tag_name))
}

/// Unpacks into `bin/yt-dlp.new`, checks that it starts and replaces the previous one.
fn install(zip_bytes: &[u8]) -> AppResult<()> {
    let bin = bin_dir().ok_or_else(|| {
        AppError::internal(tr!("No local data folder.", "Sin carpeta de datos local."))
    })?;
    install_into(zip_bytes, &bin)
}

fn install_into(zip_bytes: &[u8], bin: &Path) -> AppResult<()> {
    let io = |e: std::io::Error| {
        AppError::new(
            ErrorCode::Storage,
            tr!(
                "Could not install yt-dlp: {e}",
                "No se pudo instalar yt-dlp: {e}",
                e = e
            ),
        )
    };
    let staging = bin.join("yt-dlp.new");
    let target = bin.join("yt-dlp");
    let old = bin.join("yt-dlp.old");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(io)?;

    #[cfg(windows)]
    {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| {
            AppError::internal(tr!(
                "Invalid yt-dlp package: {e}",
                "Paquete de yt-dlp no válido: {e}",
                e = e
            ))
        })?;
        for i in 0..archive.len() {
            let mut f = archive
                .by_index(i)
                .map_err(|e| AppError::internal(e.to_string()))?;
            // `enclosed_name` rejects paths that would escape the folder (../).
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
        // The ARM64 zip contains `yt-dlp_arm64.exe`; the app and fetch-tools call it `yt-dlp.exe`.
        let arm64 = staging.join("yt-dlp_arm64.exe");
        if arm64.is_file() && !staging.join(YT_DLP_EXE).exists() {
            std::fs::rename(&arm64, staging.join(YT_DLP_EXE)).map_err(io)?;
        }
    }
    let exe = staging.join(YT_DLP_EXE);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // ponytail: onefile starts a bit slower; avoids incompatible Python libraries in AppImage.
        std::fs::write(&exe, zip_bytes).map_err(io)?;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).map_err(io)?;
    }
    if version_of(&exe).is_none() {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(AppError::internal(tr!(
            "The new yt-dlp version does not start; keeping the previous one.",
            "La versión nueva de yt-dlp no arranca; se mantiene la anterior."
        )));
    }
    let _ = std::fs::remove_dir_all(&old);
    if target.exists() {
        std::fs::rename(&target, &old).map_err(io)?;
    }
    std::fs::rename(&staging, &target).map_err(io)?;
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

/// Launches yt-dlp and deno once when the app opens: the first cold start
/// (antivirus scanning them, disk) is the slowest part, and this way the first
/// song doesn't pay for it, since its fast start only covers ~1 min.
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
            tracing::debug!(ms = t0.elapsed().as_millis() as u64, "tools warmed up");
        })
        .ok();
}

/// Daily background check at startup.
pub fn spawn_daily_check() {
    std::thread::Builder::new()
        .name("ytdlp-update".into())
        .spawn(|| {
            // Don't compete with startup or the first song.
            std::thread::sleep(Duration::from_secs(60));
            if let Err(e) = update_yt_dlp(false) {
                tracing::info!(error = %e, "could not check yt-dlp");
            }
        })
        .ok();
}

// ---------------------------------------------------------------- commands

/// Versions of the app and the tools ("About" tab).
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

/// Checks for a new yt-dlp version right now. Returns the installed one if it changed.
#[tauri::command]
#[specta::specta]
pub async fn tools_update_now() -> AppResult<Option<String>> {
    // The user asked for it: no rate limit.
    *LAST.lock().unwrap() = None;
    tauri::async_runtime::spawn_blocking(|| update_yt_dlp(true))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?
}

/// Opens the logs folder in the file explorer.
#[tauri::command]
#[specta::specta]
pub fn tools_open_logs_dir() -> AppResult<()> {
    let dirs = crate::state::AppDirs::resolve().map_err(|e| AppError::internal(e.to_string()))?;
    crate::os::open_dir(&dirs.logs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deno_version_is_trimmed() {
        // Simulates the first line of `deno --version`.
        let first = "deno 2.9.7 (stable, release, x86_64-pc-windows-msvc)";
        let v = first
            .strip_prefix("deno ")
            .map_or(first, |r| r.split_whitespace().next().unwrap_or(r));
        assert_eq!(v, "2.9.7");
    }

    #[test]
    fn asset_per_platform() {
        for (os, arch, asset) in [
            ("windows", "x86_64", "yt-dlp_win.zip"),
            ("windows", "aarch64", "yt-dlp_win_arm64.zip"),
            ("linux", "x86_64", "yt-dlp_linux"),
            ("linux", "aarch64", "yt-dlp_linux_aarch64"),
            ("macos", "x86_64", "yt-dlp_macos"),
            ("macos", "aarch64", "yt-dlp_macos"),
        ] {
            assert_eq!(asset_name(os, arch), Some(asset), "{os} {arch}");
        }
        assert_eq!(asset_name("linux", "riscv64"), None);
    }

    #[test]
    fn install_rejects_corrupt_package() {
        assert!(install(b"this is not a zip").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn install_keeps_executable_and_rejects_broken_version() {
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

    /// Real download: `cargo test --lib -- --ignored --nocapture update_yt_dlp_for_real`.
    #[test]
    #[ignore]
    fn update_yt_dlp_for_real() {
        println!("before: {:?}", versions());
        println!("result: {:?}", update_inner(true, true));
        println!("after: {:?}", versions());
    }
}
