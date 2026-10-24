//! Actualización de la propia app desde las releases públicas de GitHub (specs/007).
//!
//! `latest.json` de la última release dice qué versión hay y dónde está el
//! instalador; el updater de Tauri comprueba su firma con la clave pública, que
//! se inyecta al compilar (`XP_MUSIC_UPDATER_PUBKEY`). Solo se actualizan
//! instalaciones con un formato compatible y no gestionadas externamente.
//! macOS queda fuera mientras la app vaya con firma ad hoc: cada versión cambia de
//! firma y el Llavero volvería a pedir permiso para leer la sesión tras actualizar.

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use specta::Type;
use tauri::utils::config::BundleType;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event;

use crate::error::{AppError, AppResult, ErrorCode};

/// Siempre apunta a la última release publicada.
const LATEST_JSON: &str =
    "https://github.com/Iznardo/xp-music/releases/latest/download/latest.json";
const LAST_CHECK_KEY: &str = "update.last_check";

pub const PUBKEY: Option<&str> = option_env!("XP_MUSIC_UPDATER_PUBKEY");

#[derive(Serialize, Type, Event, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "update://available")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterStatus {
    /// Las actualizaciones automáticas están disponibles en esta instalación.
    pub enabled: bool,
    pub pending: Option<UpdateInfo>,
}

#[derive(Default)]
pub struct UpdaterState {
    pending: Mutex<Option<(UpdateInfo, Update)>>,
}

fn installation_allows_updates(
    pubkey: Option<&str>,
    bundle: Option<BundleType>,
    executable: Option<&Path>,
    disabled: bool,
) -> bool {
    pubkey.is_some_and(|key| !key.trim().is_empty())
        && !disabled
        && matches!(
            bundle,
            Some(
                BundleType::Deb
                    | BundleType::AppImage
                    | BundleType::Rpm
                    | BundleType::Msi
                    | BundleType::Nsis
            )
        )
        // El marcador del bundle puede seguir siendo Deb tras reempaquetar en Nix.
        && executable.is_some_and(|path| !path.starts_with("/nix/store"))
}

fn enabled() -> bool {
    installation_allows_updates(
        PUBKEY,
        tauri::utils::platform::bundle_type(),
        tauri::utils::platform::current_exe().ok().as_deref(),
        std::env::var_os("XP_MUSIC_DISABLE_UPDATER").is_some_and(|value| value == "1")
            || std::env::var_os("FLATPAK_ID").is_some()
            || Path::new("/.flatpak-info").exists()
            || std::env::var_os("SNAP").is_some(),
    )
}

fn not_configured() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        tr!(
            "Automatic updates are not available in this installation.",
            "Las actualizaciones automáticas no están disponibles en esta instalación."
        ),
    )
}

pub async fn check(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    if !enabled() {
        return Err(not_configured());
    }
    let endpoint = url::Url::parse(LATEST_JSON).map_err(|e| AppError::internal(e.to_string()))?;
    let upd_err = |e: tauri_plugin_updater::Error| {
        tracing::warn!(error = %e, "updater");
        AppError::new(
            ErrorCode::Network,
            tr!(
                "Could not check for updates: {e}",
                "No se pudo comprobar la actualización: {e}",
                e = e
            ),
        )
    };
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(upd_err)?
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(upd_err)?;
    let found = updater.check().await.map_err(upd_err)?;
    let state = app.state::<UpdaterState>();
    let mut pending = state.pending.lock().unwrap();
    *pending = found.map(|u| {
        let info = UpdateInfo {
            version: u.version.clone(),
            notes: u.body.clone(),
            date: u.date.map(|d| d.to_string()),
        };
        (info, u)
    });
    if let Some((info, _)) = pending.as_ref() {
        tracing::info!(version = %info.version, "hay una versión nueva de XP Music");
    }
    Ok(pending.as_ref().map(|(i, _)| i.clone()))
}

/// Comprobación al arrancar, como mucho una vez al día.
pub fn spawn_startup_check(app: AppHandle, db: std::sync::Arc<crate::storage::Db>) {
    if !enabled() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(90)).await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last: u64 = db
            .kv_get(LAST_CHECK_KEY)
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if now.saturating_sub(last) < 24 * 3600 {
            return;
        }
        let _ = db.kv_set(LAST_CHECK_KEY, &now.to_string());
        match check(&app).await {
            Ok(Some(info)) => {
                let _ = info.emit(&app);
            }
            Ok(None) => tracing::info!("XP Music está al día"),
            Err(e) => tracing::info!(error = %e, "no se pudo comprobar la actualización"),
        }
    });
}

// ---------------------------------------------------------------- comandos

#[tauri::command]
#[specta::specta]
pub fn update_status(state: tauri::State<'_, UpdaterState>) -> UpdaterStatus {
    let enabled = enabled();
    UpdaterStatus {
        enabled,
        pending: if enabled {
            state
                .pending
                .lock()
                .unwrap()
                .as_ref()
                .map(|(i, _)| i.clone())
        } else {
            None
        },
    }
}

#[tauri::command]
#[specta::specta]
pub async fn update_check(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    check(&app).await
}

/// Descarga e instala la versión pendiente y reinicia la app.
#[tauri::command]
#[specta::specta]
pub async fn update_install(app: AppHandle) -> AppResult<()> {
    if !enabled() {
        return Err(not_configured());
    }
    let update = {
        let state = app.state::<UpdaterState>();
        let pending = state.pending.lock().unwrap();
        pending.as_ref().map(|(_, u)| u.clone())
    };
    let Some(update) = update else {
        return Err(AppError::new(
            ErrorCode::NotFound,
            tr!(
                "There is no pending update.",
                "No hay ninguna actualización pendiente."
            ),
        ));
    };
    tracing::info!(version = %update.version, "descargando actualización");
    let fail = |e: tauri_plugin_updater::Error| {
        tracing::error!(error = %e, "fallo al actualizar");
        AppError::internal(tr!(
            "Could not install the update: {e}",
            "No se pudo instalar la actualización: {e}",
            e = e
        ))
    };
    // Primero se descarga (y se verifica la firma); si falla, todo sigue como estaba.
    let bytes = update.download(|_, _| {}, || {}).await.map_err(fail)?;
    // Guardar sin parar la reproducción: la instalación puede fallar o cancelarse.
    if let Some(player) = app.try_state::<std::sync::Arc<crate::player::service::PlayerService>>() {
        player.save_now();
    }
    tracing::info!(version = %update.version, "instalando actualización");
    update.install(bytes).map_err(fail)?;
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_requires_key_and_supported_bundle() {
        let executable = Some(Path::new("/usr/bin/xp-music"));
        for bundle in [
            BundleType::Deb,
            BundleType::AppImage,
            BundleType::Rpm,
            BundleType::Msi,
            BundleType::Nsis,
        ] {
            for key in [None, Some(""), Some(" \n")] {
                assert!(!installation_allows_updates(
                    key,
                    Some(bundle.clone()),
                    executable,
                    false,
                ));
            }
            assert!(installation_allows_updates(
                Some("key"),
                Some(bundle),
                executable,
                false,
            ));
        }
        // macOS (.app / .dmg) se actualiza a mano mientras sea experimental.
        for bundle in [None, Some(BundleType::Dmg), Some(BundleType::App)] {
            assert!(!installation_allows_updates(
                Some("key"),
                bundle,
                executable,
                false,
            ));
        }
    }

    #[test]
    fn updater_rejects_external_management_and_unknown_executable() {
        for bundle in [BundleType::Deb, BundleType::AppImage, BundleType::Rpm] {
            for executable in [
                None,
                Some(Path::new("/nix/store/hash-xp-music/bin/.xp-music-wrapped")),
            ] {
                assert!(!installation_allows_updates(
                    Some("key"),
                    Some(bundle.clone()),
                    executable,
                    false,
                ));
            }
            // La exclusión de Nix respeta los límites de los componentes de la ruta.
            assert!(installation_allows_updates(
                Some("key"),
                Some(bundle.clone()),
                Some(Path::new("/nix/store-other/bin/xp-music")),
                false,
            ));
            // Opt-out explícito, Flatpak y Snap comparten este veto.
            assert!(!installation_allows_updates(
                Some("key"),
                Some(bundle),
                Some(Path::new("/usr/bin/xp-music")),
                true,
            ));
        }
    }
}
