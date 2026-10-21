//! Estado de la interfaz que debe sobrevivir a la WebView (principio II) y
//! operaciones de ventana: sonidos, última vista y modo compacto (004).

use std::sync::{Arc, Mutex};

use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, State, WebviewWindow};

use crate::audio::sounds::UiSound;
use crate::error::{AppError, AppResult};
use crate::player::service::PlayerService;
use crate::settings::Settings;
use crate::state::AppState;

/// Tamaño del modo compacto (píxeles lógicos).
const COMPACT: (f64, f64) = (460.0, 150.0);
const MIN_FULL: (f64, f64) = (640.0, 420.0);

/// Tamaño, posición y si estaba maximizada antes de pasar a una ventana pequeña.
type Restore = (PhysicalSize<u32>, PhysicalPosition<i32>, bool);

#[derive(Default)]
pub struct UiState {
    last_route: Mutex<Option<String>>,
    /// Tamaño y posición en modo completo, para volver a ellos.
    restore: Mutex<Option<Restore>>,
    compact: Mutex<bool>,
    /// Modo máscara (skin de WMP).
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
    tracing::warn!(error = %e, "no se pudo cambiar el modo de ventana");
    AppError::internal("No se pudo cambiar el tamaño de la ventana.")
}

/// Ventana pequeña de tamaño fijo (modo compacto o máscara). Guarda el tamaño y
/// la posición del modo completo la primera vez, para volver a ellos.
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
        *restore = Some((window.outer_size()?, window.outer_position()?, maximized));
    }
    window.set_min_size(None::<LogicalSize<f64>>)?;
    window.set_size(size)?;
    window.set_resizable(false)?;
    window.set_always_on_top(on_top)?;
    Ok(())
}

/// Vuelve al tamaño y la posición del modo completo.
fn restore_full(window: &WebviewWindow, ui: &UiState) -> tauri::Result<()> {
    window.set_always_on_top(false)?;
    window.set_resizable(true)?;
    window.set_min_size(Some(LogicalSize::new(MIN_FULL.0, MIN_FULL.1)))?;
    if let Some((size, pos, maximized)) = ui.restore.lock().unwrap().take() {
        window.set_size(size)?;
        window.set_position(pos)?;
        if maximized {
            window.maximize()?;
        }
    }
    Ok(())
}

/// Cambia entre modo completo y compacto (FR-004-13).
#[tauri::command]
#[specta::specta]
pub fn window_set_compact(
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

/// Modo máscara: la ventana toma el tamaño de la vista del skin (píxeles
/// lógicos); `None` vuelve al modo completo.
#[tauri::command]
#[specta::specta]
pub fn window_set_skin(
    window: WebviewWindow,
    ui: State<'_, UiState>,
    size: Option<(f64, f64)>,
) -> AppResult<()> {
    let mut skin = ui.skin.lock().unwrap();
    match size {
        Some((w, h)) => {
            if !(w.is_finite() && h.is_finite()) {
                return Err(AppError::internal("Tamaño de máscara no válido."));
            }
            let size = LogicalSize::new(w.clamp(16.0, 4096.0), h.clamp(16.0, 4096.0));
            shrink(&window, &ui, size, false).map_err(window_err)?;
            *ui.compact.lock().unwrap() = false;
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

/// Errores y avisos del frontend al log de la app (la WebView no tiene consola visible).
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

/// Activa el analizador solo mientras la visualización está a la vista (FR-005-03).
#[tauri::command]
#[specta::specta]
pub fn viz_set_active(viz: State<'_, crate::viz::VizService>, active: bool) {
    viz.set_active(active);
}

/// Forma del skin para los clics (specs/008): la unión de los polígonos de sus
/// piezas, en píxeles físicos relativos al área de cliente; `None` = ventana
/// rectangular (maximizada, modo compacto).
///
/// No se recorta la ventana con `SetWindowRgn`: con región, Windows deja de
/// componer bien la transparencia por píxel y los bordes curvos salen en escalera
/// (y con píxeles blancos). La ventana es transparente y los bordes suavizados los
/// pinta la interfaz; para que los clics en lo transparente lleguen a lo que hay
/// detrás, un hilo mira dónde está el ratón y activa o desactiva
/// `set_ignore_cursor_events`.
///
/// Es asíncrono a propósito: no corre en el hilo principal, que es el que atiende
/// a la ventana (antes podía quedarse esperando al hilo de sondeo y colgarse).
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
        /// Se guarda al fijar la forma: pedírselo a la ventana desde el hilo de
        /// sondeo pasa por el hilo principal y, con el cerrojo cogido, podía colgar
        /// la app (AppHang).
        hwnd: isize,
        rgn: HRGN,
        ignoring: bool,
    }
    // La región solo se usa desde el hilo de sondeo y bajo el mutex.
    unsafe impl Send for Shape {}

    static SHAPE: Mutex<Option<Shape>> = Mutex::new(None);
    static POLLER: OnceLock<()> = OnceLock::new();

    /// Nunca se llama a la ventana con el cerrojo cogido.
    pub fn set(window: WebviewWindow, polygons: Option<Vec<Vec<(i32, i32)>>>) {
        let rgn = polygons.and_then(|p| unsafe { union(p) });
        let hwnd = window.hwnd().map(|h| h.0 as isize).ok();
        let old = SHAPE.lock().unwrap().take();
        if let Some(old) = old {
            unsafe { DeleteObject(old.rgn) };
        }
        // Al cambiar de forma, la ventana vuelve a recibir los clics.
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

    /// ~25 veces por segundo: ¿está el ratón sobre una parte transparente?
    fn poll() {
        loop {
            std::thread::sleep(Duration::from_millis(40));
            // Se decide con el cerrojo puesto y se aplica sin él: el cambio pasa por
            // el hilo principal, que también puede estar esperando este cerrojo.
            let change = {
                let mut guard = SHAPE.lock().unwrap();
                let Some(shape) = guard.as_mut() else {
                    continue;
                };
                // El HWND guardado: nada de pedírselo a la ventana con el cerrojo cogido.
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
                    // Solo importa dentro del rectángulo de la ventana.
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
