mod commands;
mod domain;
mod error;
mod logging;
mod settings;
mod state;
mod storage;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::state::{AppDirs, AppState};

/// Ruta del archivo de bindings TS, relativa a `src-tauri/`.
const BINDINGS_PATH: &str = "../src/lib/bindings.ts";

/// Registro único de comandos, eventos y tipos compartidos con el frontend.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::settings_get,
            commands::settings_set
        ])
        .events(collect_events![settings::Settings])
        // Tipos de dominio aún sin comando que los use (se exportan igualmente).
        .typ::<domain::Track>()
        .typ::<domain::PlaylistSummary>()
        .typ::<error::AppError>()
}

fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default()
                .header("// GENERADO por tauri-specta desde src-tauri. No editar a mano.\n"),
            BINDINGS_PATH,
        )
        .expect("no se pudieron exportar los bindings TS");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let dirs = AppDirs::resolve().expect("no se pudieron crear las carpetas de la app");
    let _log_guard = logging::init(&dirs.logs);
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "arrancando XP Music");

    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    let db = storage::Db::open(&dirs.data.join("xp-music.db"))
        .expect("no se pudo abrir la base de datos");
    match settings::Settings::load(&db) {
        Ok(s) => tracing::info!(theme = ?s.theme, volume = s.volume, "ajustes cargados"),
        Err(e) => tracing::warn!(error = %e, "no se pudieron cargar los ajustes"),
    }

    tauri::Builder::default()
        // Debe ir primero: una segunda instancia solo enfoca la ventana existente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(AppState { db })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error al ejecutar la aplicación");
}

#[cfg(test)]
mod tests {
    /// `cargo test export_bindings` regenera `src/lib/bindings.ts` sin abrir la app.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
