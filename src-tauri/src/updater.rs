//! Actualización de la propia app desde las releases públicas de GitHub (specs/007).
//!
//! `latest.json` de la última release dice qué versión hay y dónde está el
//! instalador; el updater de Tauri comprueba su firma con la clave pública, que
//! se inyecta al compilar (`XP_MUSIC_UPDATER_PUBKEY`, lo hace `scripts/release.ps1`).
//! Sin ella la comprobación queda desactivada.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use specta::Type;
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
    /// Esta compilación trae la clave pública del updater.
    pub enabled: bool,
    pub pending: Option<UpdateInfo>,
}

#[derive(Default)]
pub struct UpdaterState {
    pending: Mutex<Option<(UpdateInfo, Update)>>,
}

fn not_configured() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "Esta compilación no tiene activadas las actualizaciones automáticas.",
    )
}

pub async fn check(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    if PUBKEY.is_none() {
        return Err(not_configured());
    }
    let endpoint = url::Url::parse(LATEST_JSON).map_err(|e| AppError::internal(e.to_string()))?;
    let upd_err = |e: tauri_plugin_updater::Error| {
        tracing::warn!(error = %e, "updater");
        AppError::new(
            ErrorCode::Network,
            format!("No se pudo comprobar la actualización: {e}"),
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
    if PUBKEY.is_none() {
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
    UpdaterStatus {
        enabled: PUBKEY.is_some(),
        pending: state
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .map(|(i, _)| i.clone()),
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
    let update = {
        let state = app.state::<UpdaterState>();
        let pending = state.pending.lock().unwrap();
        pending.as_ref().map(|(_, u)| u.clone())
    };
    let Some(update) = update else {
        return Err(AppError::new(
            ErrorCode::NotFound,
            "No hay ninguna actualización pendiente.",
        ));
    };
    tracing::info!(version = %update.version, "descargando actualización");
    let fail = |e: tauri_plugin_updater::Error| {
        tracing::error!(error = %e, "fallo al actualizar");
        AppError::internal(format!("No se pudo instalar la actualización: {e}"))
    };
    // Primero se descarga (y se verifica la firma); si falla, todo sigue como estaba.
    let bytes = update.download(|_, _| {}, || {}).await.map_err(fail)?;
    // Guardar la cola antes de que el instalador cierre la app.
    if let Some(player) = app.try_state::<std::sync::Arc<crate::player::service::PlayerService>>() {
        player.shutdown();
    }
    tracing::info!(version = %update.version, "instalando actualización");
    update.install(bytes).map_err(fail)?;
    app.restart();
}
