//! In-app sign-in: a WebView2 window with its own profile
//! (`%LOCALAPPDATA%\xp-music\login`) opens the Google page; once it reaches
//! music.youtube.com with a session, its cookies are read. This works with any
//! browser (or none) and makes it possible to hand the app to other people.
//!
//! The same profile is reused, with the window hidden, to **renew** the session
//! when Google rotates the cookies: reloading music.youtube.com is enough.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::webview::{Cookie, PageLoadEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{AppError, AppResult, ErrorCode};

const LABEL: &str = "login";
const MUSIC: &str = "https://music.youtube.com/";
/// Google sign-in that returns to YouTube Music.
const SIGN_IN: &str = "https://accounts.google.com/ServiceLogin?service=youtube&passive=true&continue=https%3A%2F%2Fwww.youtube.com%2Fsignin%3Faction_handle_signin%3Dtrue%26app%3Ddesktop%26next%3Dhttps%253A%252F%252Fmusic.youtube.com%252F";
/// For the background renewal.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(30);

/// Ready-to-use cookies: (`Cookie` header for music.youtube.com, Netscape format for yt-dlp).
pub type Cookies = (String, String);

fn data_dir() -> AppResult<PathBuf> {
    dirs::data_local_dir()
        .map(|d| d.join("xp-music").join("login"))
        .ok_or_else(|| {
            AppError::internal(tr!("No local data folder.", "Sin carpeta de datos local."))
        })
}

/// Opens the sign-in window and waits for it to finish (or be closed).
pub async fn sign_in(app: &AppHandle) -> AppResult<Cookies> {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.set_focus();
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            tr!(
                "A sign-in window is already open.",
                "Ya hay una ventana de inicio de sesión abierta."
            ),
        ));
    }
    run(app, SIGN_IN, true, None).await
}

/// Reloads YouTube Music with the saved profile, without showing anything, and
/// returns the new cookies.
pub async fn refresh(app: &AppHandle) -> AppResult<Cookies> {
    if app.get_webview_window(LABEL).is_some() {
        return Err(AppError::new(
            ErrorCode::AuthExpired,
            tr!("Sign-in in progress.", "Inicio de sesión en curso."),
        ));
    }
    run(app, MUSIC, false, Some(REFRESH_TIMEOUT)).await
}

/// Deletes the sign-in profile (on sign-out).
pub fn forget() {
    if let Ok(dir) = data_dir() {
        // WebView2 may take a moment to release the files.
        for _ in 0..10 {
            if !dir.exists() || std::fs::remove_dir_all(&dir).is_ok() {
                return;
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        tracing::warn!("could not delete the sign-in profile");
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
        WebviewUrl::External(start.parse().expect("fixed URL")),
    )
    .title(tr!(
        "Sign in to YouTube Music - XP Music",
        "Iniciar sesión en YouTube Music - XP Music"
    ))
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
        // Reading cookies inside an event handler blocks WebView2: use another thread.
        let tx = Arc::clone(&on_load_tx);
        let done = Arc::clone(&on_load_done);
        std::thread::spawn(move || {
            let result = read(&win);
            let has_session = result.is_ok();
            // No session yet (e.g. welcome page): keep waiting, except when
            // renewing, where nobody will be there to sign in.
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
        tracing::error!(error = %e, "could not open the sign-in window");
        AppError::internal(tr!(
            "Could not open the sign-in window: {e}",
            "No se pudo abrir la ventana de inicio de sesión: {e}",
            e = e
        ))
    })?;

    // If the user closes the window without finishing.
    let close_tx = Arc::clone(&tx);
    let close_done = Arc::clone(&finished);
    win.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed)
            && !close_done.swap(true, Ordering::AcqRel)
            && let Some(tx) = close_tx.lock().unwrap().take()
        {
            let _ = tx.send(Err(AppError::new(
                ErrorCode::InvalidInput,
                tr!(
                    "The window was closed without signing in.",
                    "Se cerró la ventana sin iniciar sesión."
                ),
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
                    tr!(
                        "YouTube Music did not respond while renewing the session.",
                        "YouTube Music no respondió al renovar la sesión."
                    ),
                ));
            }
        },
        None => rx.await,
    };
    result.unwrap_or_else(|_| {
        Err(AppError::internal(tr!(
            "Sign-in interrupted.",
            "Inicio de sesión interrumpido."
        )))
    })
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
            tr!(
                "You are not signed in to YouTube.",
                "No hay sesión de YouTube iniciada."
            ),
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
        // WebView2 gives the domain without a leading dot even if it covers subdomains.
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
    fn header_and_netscape_from_cookies() {
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
