mod audio;
mod commands;
mod domain;
mod error;
mod logging;
mod os;
mod player;
mod settings;
mod state;
mod storage;
mod tools;
mod ui_state;
mod updater;
mod viz;
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
#[cfg(any(debug_assertions, test))]
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
            commands::player_hint,
            commands::queue_play_index,
            commands::queue_add,
            commands::queue_remove,
            commands::queue_move,
            commands::queue_clear,
            commands::queue_set_autoplay,
            commands::auth_status,
            commands::auth_sign_in,
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
            ui_state::ui_log,
            ui_state::viz_set_active,
            tools::tools_versions,
            tools::tools_update_now,
            tools::tools_open_logs_dir,
            updater::update_status,
            updater::update_set_token,
            updater::update_check,
            updater::update_install,
        ])
        .events(collect_events![
            settings::Settings,
            player::types::PlayerState,
            player::types::PositionUpdate,
            player::types::QueueSnapshot,
            player::types::PlayerError,
            yt::service::AuthStatus,
            yt::service::CacheUpdated,
            viz::VizFrame,
            updater::UpdateInfo,
        ])
        .typ::<error::AppError>()
}

#[cfg(any(debug_assertions, test))]
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
        // Debe ir primero: una segunda instancia solo muestra la ventana existente (o la recrea).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            os::show_main(app)
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        // Sin clave pública (compilación de desarrollo) `update_check` lo rechaza antes de usarlo.
        .plugin(
            tauri_plugin_updater::Builder::new()
                .pubkey(updater::PUBKEY.unwrap_or_default())
                .build(),
        )
        // Portadas: http://thumb.localhost/<url codificada>
        .register_asynchronous_uri_scheme_protocol("thumb", move |_ctx, request, responder| {
            let thumbs = Arc::clone(&thumbs);
            std::thread::spawn(move || responder.respond(thumbs.handle(&request)));
        })
        .manage(AppState {
            db: Arc::clone(&db),
        })
        .manage(ui_state::UiState::default())
        .manage(os::OsState::default())
        .manage(updater::UpdaterState::default())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let handle = app.handle().clone();
            let yt = YtService::new(Some(handle.clone()), Arc::clone(&db), data_dir);
            let reporter = YtReporter::new(Arc::clone(&yt), Arc::clone(&db));
            let player = PlayerService::start(
                handle.clone(),
                Arc::clone(&db),
                build_resolver(Arc::clone(&db), Arc::clone(&yt)),
                Arc::new(YtRadio(Arc::clone(&yt))),
                Arc::new(reporter.clone()),
            );
            reporter.attach(&player);
            app.manage(viz::VizService::new(handle.clone(), Arc::clone(&player)));
            app.manage(Arc::clone(&yt));
            app.manage(Arc::clone(&player));

            tools::spawn_daily_check();
            updater::spawn_startup_check(handle.clone(), Arc::clone(&db));

            // --- Integración con Windows (006) ---
            if let Err(e) = os::tray::create(&handle, Arc::clone(&player), Arc::clone(&yt)) {
                tracing::error!(error = %e, "no se pudo crear el icono de la bandeja");
            }
            let lock_db = Arc::clone(&db);
            let media = os::media::start(Arc::clone(&player), move || {
                settings::Settings::load(&lock_db)
                    .map(|s| s.pause_on_lock)
                    .unwrap_or(false)
            });
            let tb_player = Arc::clone(&player);
            os::taskbar::set_handler(move |id| match id {
                os::taskbar::BTN_PREV => tb_player.previous(),
                os::taskbar::BTN_TOGGLE => tb_player.toggle(),
                os::taskbar::BTN_NEXT => tb_player.next(),
                _ => {}
            });
            let obs_app = handle.clone();
            let obs_db = Arc::clone(&db);
            let last_track = std::sync::Mutex::new(None::<String>);
            player.observe(Box::new(move |s| {
                let _ = media.send(os::media::MediaMsg::State(Box::new(s.clone())));
                let playing = matches!(
                    s.status,
                    player::types::PlaybackStatus::Playing
                        | player::types::PlaybackStatus::Buffering
                );
                let _ = obs_app.run_on_main_thread(move || os::taskbar::set_playing(playing));
                // "Reproduciendo ahora" si la ventana no está a la vista (FR-006-07).
                let id = s.track.as_ref().map(|t| t.id.0.clone());
                let mut last = last_track.lock().unwrap();
                if id.is_some() && id != *last {
                    let first = last.is_none();
                    *last = id;
                    let wants = settings::Settings::load(&obs_db)
                        .map(|x| x.notifications)
                        .unwrap_or(true);
                    if !first
                        && wants
                        && !os::main_visible(&obs_app)
                        && let Some(t) = &s.track
                    {
                        use tauri_plugin_notification::NotificationExt;
                        let artist = t
                            .artists
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        let _ = obs_app
                            .notification()
                            .builder()
                            .title(&t.title)
                            .body(artist)
                            .show();
                    }
                }
            }));

            let s = settings::Settings::load(&db).unwrap_or_default();
            os::apply_settings(&handle, &s);
            let minimized = s.start_minimized || std::env::args().any(|a| a == "--minimized");
            if minimized {
                tracing::info!("arranque minimizado en la bandeja");
            } else {
                os::show_main(&handle);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error al construir la aplicación");

    app.run(|app, event| match event {
        // Cerrar la ventana no cierra la app si se sigue en la bandeja (FR-006-03).
        tauri::RunEvent::ExitRequested {
            api, code: None, ..
        } => {
            let quitting = app
                .try_state::<os::OsState>()
                .is_some_and(|o| o.quitting.load(std::sync::atomic::Ordering::Acquire));
            let to_tray = app
                .try_state::<AppState>()
                .and_then(|s| settings::Settings::load(&s.db).ok())
                .map(|s| s.close_to_tray)
                .unwrap_or(true);
            if to_tray && !quitting {
                api.prevent_exit();
                tracing::info!("ventana cerrada: sigue en la bandeja");
            }
        }
        tauri::RunEvent::Exit => {
            if let Some(player) = app.try_state::<Arc<PlayerService>>() {
                player.shutdown();
            }
            tracing::info!("XP Music cerrado");
        }
        _ => {}
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
