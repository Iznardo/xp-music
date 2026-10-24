//! UI state that must survive the WebView (principle II) and window
//! operations: sounds, last view and compact mode (004).

use std::sync::{Arc, Mutex};

use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, State, WebviewWindow};

use crate::audio::sounds::UiSound;
use crate::error::{AppError, AppResult};
use crate::player::service::PlayerService;
use crate::settings::Settings;
use crate::state::AppState;

/// Compact mode size (logical pixels).
const COMPACT: (f64, f64) = (460.0, 150.0);
const MIN_FULL: (f64, f64) = (640.0, 420.0);

/// Size, position and whether it was maximized before switching to a small window.
type Restore = (PhysicalSize<u32>, Option<PhysicalPosition<i32>>, bool);

#[derive(Default)]
pub struct UiState {
    last_route: Mutex<Option<String>>,
    /// Size and position in full mode, to return to them.
    restore: Mutex<Option<Restore>>,
    compact: Mutex<bool>,
    /// Skin mode (WMP skin).
    skin: Mutex<bool>,
}

#[tauri::command]
#[specta::specta]
pub fn ui_play_sound(
    player: State<'_, Arc<PlayerService>>,
    app: State<'_, AppState>,
    sound: UiSound,
) {
    if Settings::load(&app.db).map(|s| s.xp_sounds).unwrap_or(true) {
        player.play_sound(sound);
    }
}

#[tauri::command]
#[specta::specta]
pub fn ui_get_last_route(ui: State<'_, UiState>) -> Option<String> {
    ui.last_route.lock().unwrap().clone()
}

#[tauri::command]
#[specta::specta]
pub fn ui_set_last_route(ui: State<'_, UiState>, route: String) {
    *ui.last_route.lock().unwrap() = Some(route);
}

#[tauri::command]
#[specta::specta]
pub fn window_is_compact(ui: State<'_, UiState>) -> bool {
    *ui.compact.lock().unwrap()
}

fn window_err(e: tauri::Error) -> AppError {
    tracing::warn!(error = %e, "could not change the window mode");
    AppError::internal(tr!(
        "Could not resize the window.",
        "No se pudo cambiar el tamaño de la ventana."
    ))
}

/// Small fixed-size window (compact or skin mode). Saves the full-mode size and
/// position the first time, to return to them.
fn shrink(
    window: &WebviewWindow,
    ui: &UiState,
    size: LogicalSize<f64>,
    on_top: bool,
) -> tauri::Result<()> {
    let mut restore = ui.restore.lock().unwrap();
    if restore.is_none() {
        let maximized = window.is_maximized()?;
        if maximized {
            window.unmaximize()?;
        }
        *restore = Some((
            window.outer_size()?,
            window.outer_position().ok(),
            maximized,
        ));
    }
    #[cfg(target_os = "linux")]
    {
        // GTK may discard the pending resize when resizable is turned off.
        // Equal limits fix the size without that race.
        window.set_min_size(Some(size))?;
        window.set_max_size(Some(size))?;
    }
    #[cfg(not(target_os = "linux"))]
    window.set_min_size(None::<LogicalSize<f64>>)?;
    window.set_size(size)?;
    #[cfg(not(target_os = "linux"))]
    window.set_resizable(false)?;
    window.set_always_on_top(on_top)?;
    Ok(())
}

/// Returns to the full-mode size and position.
fn restore_full(window: &WebviewWindow, ui: &UiState) -> tauri::Result<()> {
    window.set_always_on_top(false)?;
    window.set_resizable(true)?;
    #[cfg(target_os = "linux")]
    window.set_max_size(None::<LogicalSize<f64>>)?;
    window.set_min_size(Some(LogicalSize::new(MIN_FULL.0, MIN_FULL.1)))?;
    if let Some((size, pos, maximized)) = ui.restore.lock().unwrap().take() {
        window.set_size(size)?;
        // Wayland does not allow querying or setting global positions.
        if let Some(pos) = pos {
            let _ = window.set_position(pos);
        }
        if maximized {
            window.maximize()?;
        }
    }
    Ok(())
}

/// Switches between full and compact mode (FR-004-13).
#[tauri::command]
#[specta::specta]
pub async fn window_set_compact(
    window: WebviewWindow,
    ui: State<'_, UiState>,
    enabled: bool,
) -> AppResult<()> {
    let mut compact = ui.compact.lock().unwrap();
    if *compact == enabled {
        return Ok(());
    }
    if enabled {
        shrink(&window, &ui, LogicalSize::new(COMPACT.0, COMPACT.1), true).map_err(window_err)?;
        *ui.skin.lock().unwrap() = false;
    } else {
        restore_full(&window, &ui).map_err(window_err)?;
    }
    *compact = enabled;
    Ok(())
}

/// Skin mode: the window takes the size of the skin view (logical
/// pixels); `None` returns to full mode.
#[tauri::command]
#[specta::specta]
pub async fn window_set_skin(
    window: WebviewWindow,
    ui: State<'_, UiState>,
    size: Option<(f64, f64)>,
) -> AppResult<()> {
    // Same order as window_set_compact: compact before skin.
    let mut compact = ui.compact.lock().unwrap();
    let mut skin = ui.skin.lock().unwrap();
    match size {
        Some((w, h)) => {
            if !(w.is_finite() && h.is_finite()) {
                return Err(AppError::internal(tr!(
                    "Invalid skin size.",
                    "Tamaño de máscara no válido."
                )));
            }
            let size = LogicalSize::new(w.clamp(16.0, 4096.0), h.clamp(16.0, 4096.0));
            shrink(&window, &ui, size, false).map_err(window_err)?;
            *compact = false;
            *skin = true;
        }
        None if *skin => {
            restore_full(&window, &ui).map_err(window_err)?;
            *skin = false;
        }
        None => {}
    }
    Ok(())
}

/// Frontend errors and warnings into the app log (the WebView has no visible console).
#[tauri::command]
#[specta::specta]
pub fn ui_log(level: String, message: String) {
    let message = crate::logging::redact(&message);
    match level.as_str() {
        "error" => tracing::error!(target: "ui", "{message}"),
        "warn" => tracing::warn!(target: "ui", "{message}"),
        _ => tracing::info!(target: "ui", "{message}"),
    }
}

/// Enables the analyzer only while the visualization is in view (FR-005-03).
#[tauri::command]
#[specta::specta]
pub fn viz_set_active(viz: State<'_, crate::viz::VizService>, active: bool) {
    viz.set_active(active);
}

/// Skin shape for clicks (specs/008): the union of the polygons of its
/// pieces, in physical pixels relative to the client area; `None` = rectangular
/// window (maximized, compact mode).
///
/// The window is not clipped with `SetWindowRgn`: with a region, Windows stops
/// compositing per-pixel transparency properly and curved edges come out stair-stepped
/// (and with white pixels). The window is transparent and the UI paints the smoothed
/// edges; for clicks on transparent areas to reach whatever is behind, a thread
/// watches where the mouse is and toggles `set_ignore_cursor_events`.
///
/// It is async on purpose: it does not run on the main thread, which is the one
/// serving the window (it used to be able to wait on the polling thread and hang).
#[tauri::command]
#[specta::specta]
pub async fn window_set_shape(
    window: WebviewWindow,
    polygons: Option<Vec<Vec<(i32, i32)>>>,
) -> AppResult<()> {
    #[cfg(windows)]
    shape::set(window, polygons);
    #[cfg(not(windows))]
    let _ = (window, polygons);
    Ok(())
}

#[cfg(windows)]
mod shape {
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    use tauri::WebviewWindow;
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreatePolygonRgn, DeleteObject, HRGN, PtInRegion, RGN_OR, ScreenToClient,
        WINDING,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClientRect, GetCursorPos, IsWindowVisible,
    };

    struct Shape {
        window: WebviewWindow,
        /// Stored when the shape is set: asking the window for it from the polling
        /// thread goes through the main thread and, with the lock held, could hang
        /// the app (AppHang).
        hwnd: isize,
        rgn: HRGN,
        ignoring: bool,
    }
    // The region is only used from the polling thread and under the mutex.
    unsafe impl Send for Shape {}

    static SHAPE: Mutex<Option<Shape>> = Mutex::new(None);
    static POLLER: OnceLock<()> = OnceLock::new();

    /// The window is never called while the lock is held.
    pub fn set(window: WebviewWindow, polygons: Option<Vec<Vec<(i32, i32)>>>) {
        let rgn = polygons.and_then(|p| unsafe { union(p) });
        let hwnd = window.hwnd().map(|h| h.0 as isize).ok();
        let old = SHAPE.lock().unwrap().take();
        if let Some(old) = old {
            unsafe { DeleteObject(old.rgn) };
        }
        // When the shape changes, the window receives clicks again.
        let _ = window.set_ignore_cursor_events(false);
        match (rgn, hwnd) {
            (Some(rgn), Some(hwnd)) => {
                *SHAPE.lock().unwrap() = Some(Shape {
                    window,
                    hwnd,
                    rgn,
                    ignoring: false,
                });
                POLLER.get_or_init(|| {
                    std::thread::Builder::new()
                        .name("window-shape".into())
                        .spawn(poll)
                        .ok();
                });
            }
            (Some(rgn), None) => unsafe {
                DeleteObject(rgn);
            },
            _ => {}
        }
    }

    unsafe fn union(polygons: Vec<Vec<(i32, i32)>>) -> Option<HRGN> {
        let mut total: HRGN = std::ptr::null_mut();
        for poly in polygons.into_iter().filter(|p| p.len() >= 3) {
            let pts: Vec<POINT> = poly.into_iter().map(|(x, y)| POINT { x, y }).collect();
            let rgn = unsafe { CreatePolygonRgn(pts.as_ptr(), pts.len() as i32, WINDING) };
            if rgn.is_null() {
                continue;
            }
            if total.is_null() {
                total = rgn;
            } else {
                unsafe {
                    CombineRgn(total, total, rgn, RGN_OR);
                    DeleteObject(rgn);
                }
            }
        }
        (!total.is_null()).then_some(total)
    }

    /// ~25 times per second: is the mouse over a transparent part?
    fn poll() {
        loop {
            std::thread::sleep(Duration::from_millis(40));
            // Decided with the lock held and applied without it: the change goes through
            // the main thread, which may also be waiting on this lock.
            let change = {
                let mut guard = SHAPE.lock().unwrap();
                let Some(shape) = guard.as_mut() else {
                    continue;
                };
                // The stored HWND: never ask the window for it while holding the lock.
                let hwnd = shape.hwnd as HWND;
                let mut pt = POINT { x: 0, y: 0 };
                let mut rc = RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                };
                let ignore = unsafe {
                    if IsWindowVisible(hwnd) == 0 || GetCursorPos(&mut pt) == 0 {
                        continue;
                    }
                    ScreenToClient(hwnd, &mut pt);
                    GetClientRect(hwnd, &mut rc);
                    let in_client =
                        pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom;
                    // Only matters inside the window rectangle.
                    if !in_client {
                        continue;
                    }
                    PtInRegion(shape.rgn, pt.x, pt.y) == 0
                };
                if ignore == shape.ignoring {
                    continue;
                }
                shape.ignoring = ignore;
                (shape.window.clone(), ignore)
            };
            let _ = change.0.set_ignore_cursor_events(change.1);
        }
    }
}
