//! Inicio de sesión dentro de la app: una ventana WebView2 con su propio perfil
//! (`%LOCALAPPDATA%\xp-music\login`) abre la página de Google; al llegar a
//! music.youtube.com con sesión se leen sus cookies. Así funciona con cualquier
//! navegador (o sin ninguno) y sirve para pasar la app a otras personas.
//!
//! El mismo perfil se reutiliza, con la ventana oculta, para **renovar** la sesión
//! cuando Google rota las cookies: basta con volver a cargar music.youtube.com.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::webview::{Cookie, PageLoadEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{AppError, AppResult, ErrorCode};

const LABEL: &str = "login";
const MUSIC: &str = "https://music.youtube.com/";
/// Inicio de sesión de Google que vuelve a YouTube Music.
const SIGN_IN: &str = "https://accounts.google.com/ServiceLogin?service=youtube&passive=true&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26app%3Ddesktop%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F";
/// Para la renovación en segundo plano.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

/// Cookies listas para usar: (cabecera `Cookie` de music.youtube.com, Netscape para yt-dlp).
pub type Cookies = (String, String);

fn data_dir() -> AppResult<PathBuf> {
    dirs::data_local_dir()
        .map(|d| d.join("xp-music").join("login"))
        .ok_or_else(|| AppError::internal("Sin carpeta de datos local."))
}

/// Abre la ventana de inicio de sesión y espera a que termine (o se cierre).
pub async fn sign_in(app: &AppHandle) -> AppResult<Cookies> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Ya hay una ventana de inicio de sesión abierta.",
        ));
    }
    run(app, SIGN_IN, true, None).await
}

/// Vuelve a cargar YouTube Music con el perfil guardado, sin enseñar nada, y
/// devuelve las cookies nuevas.
pub async fn refresh(app: &AppHandle) -> AppResult<Cookies> {
    if app.get_webview_window(LABEL).is_some() {
        return Err(AppError::new(
            ErrorCode::AuthExpired,
            "Inicio de sesión en curso.",
        ));
    }
    run(app, MUSIC, false, Some(REFRESH_TIMEOUT)).await
}

/// Borra el perfil del inicio de sesión (al cerrar sesión).
pub fn forget() {
    if let Ok(dir) = data_dir() {
        // WebView2 puede tardar un momento en soltar los ficheros.
        for _ in 0..10 {
            if !dir.exists() || std::fs::remove_dir_all(&dir).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        tracing::warn!("no se pudo borrar el perfil de inicio de sesión");
    }
}

async fn run(
    app: &AppHandle,
    start: &str,
    visible: bool,
    timeout: Option<Duration>,
) -> AppResult<Cookies> {
    let (tx, rx) = tokio::sync::oneshot::channel::<AppResult<Cookies>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let finished = Arc::new(AtomicBool::new(false));

    let on_load_tx = Arc::clone(&tx);
    let on_load_done = Arc::clone(&finished);
    let builder = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::External(start.parse().expect("URL fija")),
    )
    .title("Iniciar sesión en YouTube Music - XP Music")
    .inner_size(480.0, 640.0)
    .center()
    .visible(visible)
    .data_directory(data_dir()?)
    .on_page_load(move |win, payload| {
        if payload.event() != PageLoadEvent::Finished
            || payload.url().host_str() != Some("music.youtube.com")
            || on_load_done.load(Ordering::Acquire)
        {
            return;
        }
        // Leer cookies dentro de un manejador de eventos bloquea WebView2: otro hilo.
        let tx = Arc::clone(&on_load_tx);
        let done = Arc::clone(&on_load_done);
        std::thread::spawn(move || {
            let result = read(&win);
            let has_session = result.is_ok();
            // Sin sesión todavía (p. ej. página de bienvenida): se sigue esperando
            // salvo en la renovación, donde no habrá nadie para iniciar sesión.
            if !has_session && win.is_visible().unwrap_or(false) {
                return;
            }
            if done.swap(true, Ordering::AcqRel) {
                return;
            }
            if let Some(tx) = tx.lock().unwrap().take() {
                let _ = tx.send(result);
            }
            let _ = win.close();
        });
    });
    let win = builder.build().map_err(|e| {
        tracing::error!(error = %e, "no se pudo abrir la ventana de inicio de sesión");
        AppError::internal(format!(
            "No se pudo abrir la ventana de inicio de sesión: {e}"
        ))
    })?;

    // Si el usuario cierra la ventana sin terminar.
    let close_tx = Arc::clone(&tx);
    let close_done = Arc::clone(&finished);
    win.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed)
            && !close_done.swap(true, Ordering::AcqRel)
            && let Some(tx) = close_tx.lock().unwrap().take()
        {
            let _ = tx.send(Err(AppError::new(
                ErrorCode::InvalidInput,
                "Se cerró la ventana sin iniciar sesión.",
            )));
        }
    });

    let result = match timeout {
        Some(t) => match tokio::time::timeout(t, rx).await {
            Ok(r) => r,
            Err(_) => {
                finished.store(true, Ordering::Release);
                let _ = win.close();
                return Err(AppError::new(
                    ErrorCode::AuthExpired,
                    "YouTube Music no respondió al renovar la sesión.",
                ));
            }
        },
        None => rx.await,
    };
    result.unwrap_or_else(|_| Err(AppError::internal("Inicio de sesión interrumpido.")))
}

fn read(win: &WebviewWindow) -> AppResult<Cookies> {
    let all = win
        .cookies()
        .map_err(|e| AppError::internal(e.to_string()))?;
    let wanted: Vec<&Cookie<'static>> = all
        .iter()
        .filter(|c| {
            c.domain()
                .is_some_and(|d| d.ends_with("youtube.com") || d.ends_with("google.com"))
        })
        .collect();
    if !wanted
        .iter()
        .any(|c| c.name() == "SAPISID" && c.domain().is_some_and(|d| d.ends_with("youtube.com")))
    {
        return Err(AppError::new(
            ErrorCode::NotAuthenticated,
            "No hay sesión de YouTube iniciada.",
        ));
    }
    Ok(to_cookies(&wanted))
}

fn to_cookies(cookies: &[&Cookie<'static>]) -> Cookies {
    let header = cookies
        .iter()
        .filter(|c| c.domain().is_some_and(|d| d.ends_with("youtube.com")))
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; ");
    let fallback = super::session::now_secs() + 365 * 24 * 3600;
    let mut netscape = String::from("# Netscape HTTP Cookie File\n");
    for c in cookies {
        let domain = c.domain().unwrap_or_default();
        // WebView2 da el dominio sin punto inicial aunque valga para subdominios.
        let host = if domain.starts_with('.') {
            domain.to_owned()
        } else {
            format!(".{domain}")
        };
        let expiry = c
            .expires_datetime()
            .map(|d| d.unix_timestamp().max(0) as u64)
            .unwrap_or(fallback);
        let secure = if c.secure().unwrap_or(true) {
            "TRUE"
        } else {
            "FALSE"
        };
        netscape.push_str(&format!(
            "{host}\tTRUE\t{}\t{secure}\t{expiry}\t{}\t{}\n",
            c.path().unwrap_or("/"),
            c.name(),
            c.value()
        ));
    }
    (header, netscape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cabecera_y_netscape_desde_cookies() {
        let a = Cookie::build(("SAPISID", "abc"))
            .domain("youtube.com")
            .path("/")
            .build();
        let b = Cookie::build(("SID", "x"))
            .domain(".google.com")
            .path("/")
            .build();
        let (header, netscape) = to_cookies(&[&a, &b]);
        assert_eq!(header, "SAPISID=abc");
        assert!(netscape.contains(".youtube.com\tTRUE\t/\tTRUE\t"));
        assert!(netscape.contains("\tSID\tx\n"));
    }
}
