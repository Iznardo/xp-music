//! Controles multimedia del sistema (SMTC) y bloqueo de sesión (FR-006-04/09).
//!
//! SMTC necesita una ventana: se crea una propia, oculta y en su propio hilo,
//! que vive todo el proceso. Así las teclas multimedia y el panel de volumen de
//! Windows funcionan aunque la WebView esté destruida (en la bandeja).

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, MSG, PM_REMOVE, PeekMessageW,
    RegisterClassW, TranslateMessage, WM_WTSSESSION_CHANGE, WNDCLASSW,
};

use crate::player::service::PlayerService;
use crate::player::types::{PlaybackStatus, PlayerState};

const WTS_SESSION_LOCK: usize = 0x7;

/// Qué hacer al bloquearse la sesión (lo fija `start`).
static ON_LOCK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_WTSSESSION_CHANGE && wparam == WTS_SESSION_LOCK {
        if let Some(f) = ON_LOCK.get() {
            f();
        }
        return 0;
    }
    // SAFETY: reenvío estándar al procedimiento por defecto.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Ventana oculta (sin mostrar nunca) para SMTC y avisos de sesión.
fn create_hidden_window() -> Option<HWND> {
    let class = wide("XpMusicHidden");
    // SAFETY: estructuras inicializadas y cadenas terminadas en 0 que viven durante las llamadas.
    unsafe {
        let hinstance = GetModuleHandleW(std::ptr::null());
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        RegisterClassW(&wc);
        let title = wide("XP Music");
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinstance,
            std::ptr::null(),
        );
        (!hwnd.is_null()).then_some(hwnd)
    }
}

/// Lo que el reproductor manda al hilo de SMTC.
pub enum MediaMsg {
    State(Box<PlayerState>),
}

/// Arranca el hilo. `pause_on_lock` se consulta en cada bloqueo (ajuste vivo).
pub fn start(
    player: Arc<PlayerService>,
    pause_on_lock: impl Fn() -> bool + Send + Sync + 'static,
) -> Sender<MediaMsg> {
    let (tx, rx) = channel::<MediaMsg>();
    let lock_player = Arc::clone(&player);
    let _ = ON_LOCK.set(Box::new(move || {
        if pause_on_lock() {
            tracing::info!("sesión bloqueada: pausa");
            lock_player.pause();
        }
    }));
    thread::Builder::new()
        .name("os-media".into())
        .spawn(move || run(player, rx))
        .expect("hilo de controles multimedia");
    tx
}

fn run(player: Arc<PlayerService>, rx: Receiver<MediaMsg>) {
    let Some(hwnd) = create_hidden_window() else {
        tracing::error!("no se pudo crear la ventana oculta; sin teclas multimedia");
        return;
    };
    // SAFETY: hwnd válido, creado en este hilo.
    unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) };

    let config = PlatformConfig {
        display_name: "XP Music",
        dbus_name: "xp_music",
        hwnd: Some(hwnd as *mut _),
    };
    let mut controls = match MediaControls::new(config) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!(error = ?e, "SMTC no disponible");
            None
        }
    };
    if let Some(c) = controls.as_mut() {
        let p = Arc::clone(&player);
        let res = c.attach(move |ev| match ev {
            MediaControlEvent::Play => p.play(),
            MediaControlEvent::Pause => p.pause(),
            MediaControlEvent::Toggle => p.toggle(),
            MediaControlEvent::Next => p.next(),
            MediaControlEvent::Previous => p.previous(),
            MediaControlEvent::Stop => p.stop(),
            MediaControlEvent::SetPosition(MediaPosition(d)) => {
                p.seek(d.as_millis().min(u32::MAX as u128) as u32)
            }
            _ => {}
        });
        if let Err(e) = res {
            tracing::warn!(error = ?e, "no se pudieron enganchar los controles multimedia");
        }
    }

    let mut last_track: Option<String> = None;
    let mut last_status: Option<PlaybackStatus> = None;
    loop {
        // Mensajes de la ventana oculta (bloqueo de sesión).
        // SAFETY: bucle de mensajes estándar del hilo dueño de la ventana.
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(MediaMsg::State(s)) => {
                let Some(c) = controls.as_mut() else { continue };
                let id = s.track.as_ref().map(|t| t.id.0.clone());
                if id != last_track {
                    last_track = id;
                    if let Some(t) = &s.track {
                        let artist = t
                            .artists
                            .iter()
                            .map(|a| a.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        // La portada mayor, directamente de YouTube (Windows la descarga).
                        let cover = t
                            .thumbnails
                            .iter()
                            .max_by_key(|x| x.width)
                            .map(|x| x.url.clone());
                        let _ = c.set_metadata(MediaMetadata {
                            title: Some(&t.title),
                            artist: Some(&artist),
                            album: t.album.as_ref().map(|a| a.name.as_str()),
                            cover_url: cover.as_deref(),
                            duration: s.duration_ms.map(|d| Duration::from_millis(d as u64)),
                        });
                    }
                }
                if Some(s.status) != last_status {
                    last_status = Some(s.status);
                    let progress = Some(MediaPosition(Duration::from_millis(s.position_ms as u64)));
                    let pb = match s.status {
                        PlaybackStatus::Playing
                        | PlaybackStatus::Buffering
                        | PlaybackStatus::Loading => MediaPlayback::Playing { progress },
                        PlaybackStatus::Paused => MediaPlayback::Paused { progress },
                        PlaybackStatus::Idle | PlaybackStatus::Error => MediaPlayback::Stopped,
                    };
                    let _ = c.set_playback(pb);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
