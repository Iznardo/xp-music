//! Icono del área de notificación con menú dinámico (FR-006-01/02).

use std::sync::{Arc, Mutex};

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::domain::LikeStatus;
use crate::player::service::PlayerService;
use crate::player::types::{PlaybackStatus, PlayerState};
use crate::yt::service::YtService;

/// Lo que cambia el menú; si no cambia, no se reconstruye.
#[derive(Clone, PartialEq, Default)]
struct MenuKey {
    playing: bool,
    track: Option<(String, String)>,
    liked: bool,
}

fn key(s: &PlayerState) -> MenuKey {
    MenuKey {
        playing: matches!(
            s.status,
            PlaybackStatus::Playing | PlaybackStatus::Buffering | PlaybackStatus::Loading
        ),
        track: s.track.as_ref().map(|t| {
            (
                t.id.0.clone(),
                format!(
                    "{} — {}",
                    t.title,
                    t.artists
                        .iter()
                        .map(|a| a.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        }),
        liked: s
            .track
            .as_ref()
            .is_some_and(|t| t.like_status == Some(LikeStatus::Like)),
    }
}

fn build_menu(app: &AppHandle, k: &MenuKey) -> tauri::Result<Menu<tauri::Wry>> {
    let now = k
        .track
        .as_ref()
        .map(|(_, t)| truncate(t, 60))
        .unwrap_or_else(|| tr!("Nothing is playing", "No se está reproduciendo nada"));
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "now", now, false, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "toggle",
                if k.playing {
                    tr!("Pause", "Pausa")
                } else {
                    tr!("Play", "Reproducir")
                },
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "previous",
                tr!("Previous", "Anterior"),
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "next", tr!("Next", "Siguiente"), true, None::<&str>)?,
            &CheckMenuItem::with_id(
                app,
                "like",
                tr!("Like", "Me gusta"),
                k.track.is_some(),
                k.liked,
                None::<&str>,
            )?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(
                app,
                "show",
                tr!("Show XP Music", "Mostrar XP Music"),
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(
                app,
                "full",
                tr!("Back to full mode", "Volver al modo completo"),
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "quit", tr!("Exit", "Salir"), true, None::<&str>)?,
        ],
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}

pub fn create(
    app: &AppHandle,
    player: Arc<PlayerService>,
    yt: Arc<YtService>,
) -> tauri::Result<()> {
    let initial = key(&player.state());
    let menu = build_menu(app, &initial)?;
    let p = Arc::clone(&player);
    let liked_now = Arc::new(Mutex::new(initial.clone()));
    let liked_for_menu = Arc::clone(&liked_now);
    let tray: TrayIcon = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("icono de la app"))
        .tooltip("XP Music")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "toggle" => p.toggle(),
            "next" => p.next(),
            "previous" => p.previous(),
            "show" => super::show_main(app),
            // Salida de emergencia si una máscara no deja volver.
            "full" => {
                tracing::info!("bandeja: volver al modo completo");
                super::show_main(app);
                let _ = app.emit("ui://full-mode", ());
            }
            "quit" => super::quit(app),
            "like" => {
                let Some(track) = p.state().track else { return };
                let like = !liked_for_menu.lock().unwrap().liked;
                let yt = Arc::clone(&yt);
                tauri::async_runtime::spawn(async move {
                    let status = if like {
                        LikeStatus::Like
                    } else {
                        LikeStatus::Indifferent
                    };
                    if let Err(e) = yt.rate(track.id, status).await {
                        tracing::warn!(error = %e, "no se pudo marcar Me gusta desde la bandeja");
                    }
                });
                liked_for_menu.lock().unwrap().liked = like;
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if super::main_visible(app) {
                    super::hide_main(app);
                } else {
                    super::show_main(app);
                }
            }
        })
        .build(app)?;

    // Menú y texto emergente al día con el reproductor.
    let app2 = app.clone();
    player.observe(Box::new(move |s| {
        let k = key(s);
        let mut last = liked_now.lock().unwrap();
        if *last == k {
            return;
        }
        *last = k.clone();
        drop(last);
        refresh(&tray, &app2, &k);
    }));
    Ok(())
}

/// Updates the tooltip and menu from the player state.
fn refresh(tray: &TrayIcon, app: &AppHandle, k: &MenuKey) {
    let tip = k
        .track
        .as_ref()
        .map(|(_, t)| truncate(&format!("{t} - XP Music"), 120))
        .unwrap_or_else(|| "XP Music".into());
    let _ = tray.set_tooltip(Some(tip));
    match build_menu(app, k) {
        Ok(m) => {
            let _ = tray.set_menu(Some(m));
        }
        Err(e) => tracing::warn!(error = %e, "no se pudo actualizar el menú de la bandeja"),
    }
}

/// Rebuilds the menu in the current language (after a language change).
pub fn relocalize(app: &AppHandle) {
    let (Some(tray), Some(player)) = (
        app.tray_by_id("main"),
        app.try_state::<Arc<PlayerService>>(),
    ) else {
        return;
    };
    refresh(&tray, app, &key(&player.state()));
}
