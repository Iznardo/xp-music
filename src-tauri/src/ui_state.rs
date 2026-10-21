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
const COMPACT: (f64, f64) = (420.0, 118.0);
const MIN_FULL: (f64, f64) = (640.0, 420.0);

/// Tamaño, posición y si estaba maximizada antes de entrar en modo compacto.
type Restore = (PhysicalSize<u32>, PhysicalPosition<i32>, bool);

#[derive(Default)]
pub struct UiState {
    last_route: Mutex<Option<String>>,
    /// Tamaño y posición en modo completo, para volver a ellos.
    restore: Mutex<Option<Restore>>,
    compact: Mutex<bool>,
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

/// Cambia entre modo completo y compacto (FR-004-13).
#[tauri::command]
#[specta::specta]
pub fn window_set_compact(
    window: WebviewWindow,
    ui: State<'_, UiState>,
    enabled: bool,
) -> AppResult<()> {
    let err = |e: tauri::Error| {
        tracing::warn!(error = %e, "no se pudo cambiar el modo de ventana");
        AppError::internal("No se pudo cambiar el tamaño de la ventana.")
    };
    let mut compact = ui.compact.lock().unwrap();
    if *compact == enabled {
        return Ok(());
    }
    if enabled {
        let maximized = window.is_maximized().map_err(err)?;
        if maximized {
            window.unmaximize().map_err(err)?;
        }
        *ui.restore.lock().unwrap() = Some((
            window.outer_size().map_err(err)?,
            window.outer_position().map_err(err)?,
            maximized,
        ));
        window.set_min_size(None::<LogicalSize<f64>>).map_err(err)?;
        window
            .set_size(LogicalSize::new(COMPACT.0, COMPACT.1))
            .map_err(err)?;
        window.set_resizable(false).map_err(err)?;
        window.set_always_on_top(true).map_err(err)?;
    } else {
        window.set_always_on_top(false).map_err(err)?;
        window.set_resizable(true).map_err(err)?;
        window
            .set_min_size(Some(LogicalSize::new(MIN_FULL.0, MIN_FULL.1)))
            .map_err(err)?;
        if let Some((size, pos, maximized)) = ui.restore.lock().unwrap().take() {
            window.set_size(size).map_err(err)?;
            window.set_position(pos).map_err(err)?;
            if maximized {
                window.maximize().map_err(err)?;
            }
        }
    }
    *compact = enabled;
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

/// Da a la ventana la forma del skin (`SetWindowRgn`): la unión de los polígonos
/// de sus piezas. Fuera de ella no se pinta nada ni se reciben clics. Puntos en
/// píxeles físicos relativos al área de cliente; `None` = ventana rectangular
/// (maximizada, modo compacto).
#[tauri::command]
#[specta::specta]
pub fn window_set_shape(
    window: WebviewWindow,
    polygons: Option<Vec<Vec<(i32, i32)>>>,
) -> AppResult<()> {
    #[cfg(windows)]
    {
        let hwnd = window
            .hwnd()
            .map_err(|e| AppError::internal(e.to_string()))?
            .0 as isize;
        let _ = window.run_on_main_thread(move || unsafe { set_region(hwnd, polygons) });
    }
    #[cfg(not(windows))]
    let _ = (window, polygons);
    Ok(())
}

#[cfg(windows)]
unsafe fn set_region(hwnd: isize, polygons: Option<Vec<Vec<(i32, i32)>>>) {
    use windows_sys::Win32::Foundation::{HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, CombineRgn, CreatePolygonRgn, CreateRectRgn, DeleteObject, OffsetRgn,
        RGN_COPY, RGN_OR, SetWindowRgn, WINDING,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;

    let hwnd = hwnd as HWND;
    let polygons: Vec<_> = polygons
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.len() >= 3)
        .collect();
    if polygons.is_empty() {
        unsafe { SetWindowRgn(hwnd, std::ptr::null_mut(), 1) };
        return;
    }
    // La región va en coordenadas de ventana; sin decoración, Windows aún deja un
    // marco invisible para redimensionar, así que se suma el desfase del cliente.
    let mut win = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let mut origin = POINT { x: 0, y: 0 };
    unsafe {
        GetWindowRect(hwnd, &mut win);
        ClientToScreen(hwnd, &mut origin);
    }
    let (dx, dy) = (origin.x - win.left, origin.y - win.top);
    unsafe {
        let mut total: windows_sys::Win32::Graphics::Gdi::HRGN = std::ptr::null_mut();
        for poly in polygons {
            let pts: Vec<POINT> = poly
                .into_iter()
                .map(|(x, y)| POINT {
                    x: x + dx,
                    y: y + dy,
                })
                .collect();
            let rgn = CreatePolygonRgn(pts.as_ptr(), pts.len() as i32, WINDING);
            if rgn.is_null() {
                continue;
            }
            if total.is_null() {
                total = rgn;
            } else {
                CombineRgn(total, total, rgn, RGN_OR);
                DeleteObject(rgn);
            }
        }
        if total.is_null() {
            return;
        }
        // La región de Windows recorta por píxeles enteros (bordes en escalera). Se
        // agranda 2 px hacia fuera: el borde suavizado que pinta la interfaz cae
        // dentro, sobre fondo transparente, y se ve limpio.
        let grown = CreateRectRgn(0, 0, 0, 0);
        let tmp = CreateRectRgn(0, 0, 0, 0);
        CombineRgn(grown, total, std::ptr::null_mut(), RGN_COPY);
        for (ox, oy) in [
            (2, 0),
            (-2, 0),
            (0, 2),
            (0, -2),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            CombineRgn(tmp, total, std::ptr::null_mut(), RGN_COPY);
            OffsetRgn(tmp, ox, oy);
            CombineRgn(grown, grown, tmp, RGN_OR);
        }
        DeleteObject(tmp);
        DeleteObject(total);
        // Con éxito, Windows pasa a ser dueño de la región (no hay que borrarla).
        SetWindowRgn(hwnd, grown, 1);
    }
}
