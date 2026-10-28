//! System media controls (SMTC on Windows, MPRIS on Linux, MediaPlayer on macOS).
//!
//! On Windows, a hidden window on its own thread keeps SMTC and the session lock
//! notifications alive even when the WebView is destroyed (in the tray).
//! On Linux and macOS no window is needed (on macOS the app's event loop, which
//! already runs on the main thread, is enough); pausing on lock is not supported.

use std::sync::Arc;
#[cfg(windows)]
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use tauri::AppHandle;
#[cfg(windows)]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification,
};
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, MSG, PM_REMOVE, PeekMessageW,
    RegisterClassW, TranslateMessage, WM_WTSSESSION_CHANGE, WNDCLASSW,
};

use crate::player::service::PlayerService;
use crate::player::types::{PlaybackStatus, PlayerState};

#[cfg(windows)]
const WTS_SESSION_LOCK: usize = 0x7;

/// What to do when the session locks (set by `start`).
#[cfg(windows)]
static ON_LOCK: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

#[cfg(windows)]
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_WTSSESSION_CHANGE && wparam == WTS_SESSION_LOCK {
        if let Some(f) = ON_LOCK.get() {
            f();
        }
        return 0;
    }
    // SAFETY: standard forwarding to the default procedure.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(windows)]
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Hidden window (never shown) for SMTC and session notifications.
#[cfg(windows)]
fn create_hidden_window() -> Option<HWND> {
    let class = wide("XpMusicHidden");
    // SAFETY: initialized structures and NUL-terminated strings that live through the calls.
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

/// What the player sends to the media controls thread.
pub enum MediaMsg {
    State(Box<PlayerState>),
}

/// Starts the thread. On Windows, `pause_on_lock` is queried on every lock.
pub fn start(
    app: AppHandle,
    player: Arc<PlayerService>,
    pause_on_lock: impl Fn() -> bool + Send + Sync + 'static,
) -> Sender<MediaMsg> {
    let (tx, rx) = channel::<MediaMsg>();
    #[cfg(windows)]
    {
        let lock_player = Arc::clone(&player);
        let _ = ON_LOCK.set(Box::new(move || {
            if pause_on_lock() {
                tracing::info!("session locked: pausing");
                lock_player.pause();
            }
        }));
    }
    #[cfg(not(windows))]
    {
        let _ = pause_on_lock;
        tracing::info!("pause on session lock is not supported on this platform (Windows only)");
    }
    thread::Builder::new()
        .name("os-media".into())
        .spawn(move || run(app, player, rx))
        .expect("media controls thread");
    tx
}

fn run(app: AppHandle, player: Arc<PlayerService>, rx: Receiver<MediaMsg>) {
    #[cfg(windows)]
    let hwnd = {
        let Some(hwnd) = create_hidden_window() else {
            tracing::error!("could not create the hidden window; no media keys");
            return;
        };
        // SAFETY: valid hwnd, created on this thread.
        unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) };
        Some(hwnd as *mut _)
    };
    #[cfg(not(windows))]
    let hwnd = None;

    let config = PlatformConfig {
        display_name: "XP Music",
        dbus_name: "xp_music",
        hwnd,
    };
    let mut controls = match MediaControls::new(config) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!(error = ?e, "system media controls not available");
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
            // MPRIS `Volume` writes (DMS, playerctl volume, ...).
            MediaControlEvent::SetVolume(v) => p.set_volume(v as f32),
            // MPRIS `Seek(offset)` (`playerctl position 10+`, ...).
            MediaControlEvent::SeekBy(dir, d) => {
                let pos = p.state().position_ms;
                let d = d.as_millis().min(u32::MAX as u128) as u32;
                p.seek(match dir {
                    SeekDirection::Forward => pos.saturating_add(d),
                    SeekDirection::Backward => pos.saturating_sub(d),
                })
            }
            // Same as the tray menu's "Show" and "Exit".
            MediaControlEvent::Raise => {
                let a = app.clone();
                let _ = app.run_on_main_thread(move || super::show_main(&a));
            }
            MediaControlEvent::Quit => {
                let a = app.clone();
                let _ = app.run_on_main_thread(move || super::quit(&a));
            }
            _ => {}
        });
        if let Err(e) = res {
            tracing::warn!(error = ?e, "could not hook up the media controls");
        }
    }

    let mut last_track: Option<String> = None;
    let mut last_status: Option<PlaybackStatus> = None;
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut last_volume: Option<f32> = None;
    loop {
        // Messages for the hidden window (session lock).
        // SAFETY: standard message loop of the thread that owns the window.
        #[cfg(windows)]
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
                        // The largest cover, straight from YouTube. A local song's is only
                        // reachable through the app's own protocol: none for the system.
                        let cover = t
                            .thumbnails
                            .iter()
                            .filter(|x| x.url.starts_with("http"))
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
                // Only MPRIS exposes a volume property.
                #[cfg(all(unix, not(target_os = "macos")))]
                if Some(s.volume) != last_volume {
                    last_volume = Some(s.volume);
                    let _ = c.set_volume(s.volume as f64);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
