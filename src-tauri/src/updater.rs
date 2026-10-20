//! Actualización de la propia app desde el repo privado de GitHub (specs/007).
//!
//! Las descargas de assets de un repo privado solo funcionan por la API con token,
//! así que primero se busca `latest.json` en la última release y se configura el
//! updater de Tauri en tiempo de ejecución con esa URL y las cabeceras.
//! La clave pública del updater se inyecta al compilar (`XP_MUSIC_UPDATER_PUBKEY`,
//! lo hace `scripts/release.ps1`); sin ella la comprobación queda desactivada.

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event;

use crate::error::{AppError, AppResult, ErrorCode};

const REPO: &str = "Iznardo/xp-music";
const KEYRING_SERVICE: &str = "xp-music";
const KEYRING_USER: &str = "github-token";
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
    pub has_token: bool,
    pub pending: Option<UpdateInfo>,
}

#[derive(Default)]
pub struct UpdaterState {
    pending: Mutex<Option<(UpdateInfo, Update)>>,
}

// ---------------------------------------------------------------- token

fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| AppError::internal(format!("Administrador de credenciales: {e}")))
}

fn token() -> Option<String> {
    entry()
        .ok()?
        .get_password()
        .ok()
        .filter(|t| !t.trim().is_empty())
}

// ---------------------------------------------------------------- comprobación

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    /// URL de la API (la única que sirve con token en un repo privado).
    url: String,
}

fn not_configured() -> AppError {
    AppError::new(
        ErrorCode::InvalidInput,
        "Esta compilación no tiene activadas las actualizaciones automáticas.",
    )
}

/// URL de la API del `latest.json` de la última release.
async fn latest_json_url(token: &str) -> AppResult<url::Url> {
    let net = |e: reqwest::Error| {
        // El mensaje de reqwest no incluye cabeceras: no se filtra el token.
        tracing::warn!(error = %e, "consulta de releases");
        AppError::new(ErrorCode::Network, "No se pudo consultar GitHub.")
    };
    let resp = reqwest::Client::new()
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .bearer_auth(token)
        .header(
            "User-Agent",
            concat!("xp-music/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(net)?;
    match resp.status().as_u16() {
        401 | 403 => {
            return Err(AppError::new(
                ErrorCode::NotAuthenticated,
                "GitHub rechazó el token de actualizaciones (¿caducado o sin permiso de lectura?).",
            ));
        }
        // Sin releases todavía.
        404 => {
            return Err(AppError::new(
                ErrorCode::NotFound,
                "Aún no hay versiones publicadas.",
            ));
        }
        _ => {}
    }
    let rel: Release = resp
        .error_for_status()
        .map_err(net)?
        .json()
        .await
        .map_err(net)?;
    let asset = rel
        .assets
        .into_iter()
        .find(|a| a.name == "latest.json")
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::NotFound,
                "La última versión no trae latest.json.",
            )
        })?;
    url::Url::parse(&asset.url).map_err(|e| AppError::internal(e.to_string()))
}

pub async fn check(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    if PUBKEY.is_none() {
        return Err(not_configured());
    }
    let Some(token) = token() else {
        return Err(AppError::new(
            ErrorCode::NotAuthenticated,
            "Falta el token de GitHub para buscar actualizaciones.",
        ));
    };
    let endpoint = latest_json_url(&token).await?;
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
        .header("Authorization", format!("Bearer {token}"))
        .map_err(upd_err)?
        // Assets por la API: sin esta cabecera devuelve el JSON del asset, no el fichero.
        .header("Accept", "application/octet-stream")
        .map_err(upd_err)?
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

/// Comprobación al arrancar, como mucho una vez al día y sin molestar si no hay token.
pub fn spawn_startup_check(app: AppHandle, db: std::sync::Arc<crate::storage::Db>) {
    if PUBKEY.is_none() || token().is_none() {
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
        has_token: token().is_some(),
        pending: state
            .pending
            .lock()
            .unwrap()
            .as_ref()
            .map(|(i, _)| i.clone()),
    }
}

/// Guarda (o borra, con `None`) el token de GitHub en el Administrador de credenciales.
#[tauri::command]
#[specta::specta]
pub fn update_set_token(token: Option<String>) -> AppResult<()> {
    let e = entry()?;
    match token.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()) {
        Some(t) => e
            .set_password(&t)
            .map_err(|e| AppError::internal(format!("No se pudo guardar el token: {e}"))),
        None => match e.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::internal(format!(
                "No se pudo borrar el token: {e}"
            ))),
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
