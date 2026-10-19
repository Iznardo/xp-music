mod audio;
mod commands;
mod domain;
mod error;
mod logging;
mod player;
mod settings;
mod state;
mod storage;
mod ui_state;
mod yt;

use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{Builder, collect_commands, collect_events};

use crate::audio::resolver::{
    CachedResolver, MissingTools, StreamResolver, ToolPaths, YtDlpResolver,
};
use crate::player::service::PlayerService;
use crate::state::{AppDirs, AppState};
use crate::yt::playback::{YtRadio, YtReporter};
use crate::yt::service::YtService;
use crate::yt::thumbs::ThumbCache;

/// Ruta del archivo de bindings TS, relativa a `src-tauri/`.
const BINDINGS_PATH: &str = "../src/lib/bindings.ts";

/// Registro único de comandos, eventos y tipos compartidos con el frontend.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::settings_get,
            commands::settings_set,
            commands::player_get_state,
            commands::player_play,
            commands::player_pause,
            commands::player_toggle,
            commands::player_stop,
            commands::player_next,
            commands::player_previous,
            commands::player_seek,
            commands::player_set_volume,
            commands::player_set_muted,
            commands::player_set_shuffle,
            commands::player_set_repeat,
            commands::player_list_devices,
            commands::player_set_device,
            commands::queue_get,
            commands::queue_set,
            commands::queue_play_index,
            commands::queue_add,
            commands::queue_remove,
            commands::queue_move,
            commands::queue_clear,
            commands::queue_set_autoplay,
            commands::auth_status,
            commands::auth_detect_browsers,
            commands::auth_scan_browser,
            commands::auth_scan_cookie,
            commands::auth_select_account,
            commands::auth_logout,
            commands::yt_home,
            commands::yt_library_playlists,
            commands::yt_library_albums,
            commands::yt_library_artists,
            commands::yt_library_songs,
            commands::yt_liked_songs,
            commands::yt_history,
            commands::yt_more_tracks,
            commands::yt_playlist,
            commands::yt_album,
            commands::yt_artist,
            commands::yt_search,
            commands::yt_search_more,
            commands::yt_search_suggestions,
            commands::yt_radio,
            commands::yt_rate,
            commands::yt_add_to_playlist,
            commands::yt_create_playlist,
            ui_state::ui_play_sound,
            ui_state::ui_get_last_route,
            ui_state::ui_set_last_route,
            ui_state::window_is_compact,
            ui_state::window_set_compact,
        ])
        .events(collect_events![
            settings::Settings,
            player::types::PlayerState,
            player::types::PositionUpdate,
            player::types::QueueSnapshot,
            player::types::PlayerError,
            yt::service::AuthStatus,
            yt::service::CacheUpdated,
        ])
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

/// yt-dlp con caché; si faltan las herramientas, un resolvedor que lo explica.
fn build_resolver(db: Arc<storage::Db>, yt: Arc<YtService>) -> Arc<dyn StreamResolver> {
    match ToolPaths::locate() {
        Some(tools) => {
            tracing::info!(yt_dlp = %tools.yt_dlp.display(), "herramientas encontradas");
            let cookies = Box::new(move || yt.netscape_cookies());
            let cached = CachedResolver::new(YtDlpResolver::new(tools, Some(cookies)), db);
            cached.purge_expired();
            Arc::new(cached)
        }
        None => {
            tracing::error!("no se encuentran yt-dlp.exe y deno.exe");
            Arc::new(MissingTools)
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let dirs = AppDirs::resolve().expect("no se pudieron crear las carpetas de la app");
    let _log_guard = logging::init(&dirs.logs);
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "arrancando XP Music");

    let builder = specta_builder();
    #[cfg(debug_assertions)]
    export_bindings(&builder);

    let db = Arc::new(
        storage::Db::open(&dirs.data.join("xp-music.db"))
            .expect("no se pudo abrir la base de datos"),
    );
    match settings::Settings::load(&db) {
        Ok(s) => tracing::info!(theme = ?s.theme, volume = s.volume, "ajustes cargados"),
        Err(e) => tracing::warn!(error = %e, "no se pudieron cargar los ajustes"),
    }
    let thumbs = ThumbCache::new(dirs.thumbs.clone(), Arc::clone(&db));
    let data_dir = dirs.data.clone();

    let app = tauri::Builder::default()
        // Debe ir primero: una segunda instancia solo enfoca la ventana existente.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // Portadas: http://thumb.localhost/<url codificada>
        .register_asynchronous_uri_scheme_protocol("thumb", move |_ctx, request, responder| {
            let thumbs = Arc::clone(&thumbs);
            std::thread::spawn(move || responder.respond(thumbs.handle(&request)));
        })
        .manage(AppState {
            db: Arc::clone(&db),
        })
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let yt = YtService::new(Some(app.handle().clone()), Arc::clone(&db), data_dir);
            let reporter = YtReporter::new(Arc::clone(&yt), Arc::clone(&db));
            let player = PlayerService::start(
                app.handle().clone(),
                Arc::clone(&db),
                build_resolver(Arc::clone(&db), Arc::clone(&yt)),
                Arc::new(YtRadio(Arc::clone(&yt))),
                Arc::new(reporter.clone()),
            );
            reporter.attach(&player);
            app.manage(yt);
            app.manage(player);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error al construir la aplicación");

    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            if let Some(player) = app.try_state::<Arc<PlayerService>>() {
                player.shutdown();
            }
            tracing::info!("XP Music cerrado");
        }
    });
}

#[cfg(test)]
mod tests {
    /// `cargo test export_bindings` regenera `src/lib/bindings.ts` sin abrir la app.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
