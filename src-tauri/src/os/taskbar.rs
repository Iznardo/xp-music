//! Botones en la miniatura de la barra de tareas, como WMP (FR-006-05).
//! Anterior · Reproducir/Pausa · Siguiente, con iconos dibujados en código.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
    DeleteObject, HGDIOBJ,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{
    DefSubclassProc, ITaskbarList3, SetWindowSubclass, THB_FLAGS, THB_ICON, THB_TOOLTIP,
    THBF_ENABLED, THUMBBUTTON, TaskbarList,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, HICON, ICONINFO, RegisterWindowMessageW, WM_COMMAND,
};
use windows::core::w;

const THBN_CLICKED: usize = 0x1800;
const SUBCLASS_ID: usize = 0x5850_4d55; // "XPMU"

pub const BTN_PREV: u32 = 1;
pub const BTN_TOGGLE: u32 = 2;
pub const BTN_NEXT: u32 = 3;

/// Iconos como punteros crudos (HICON no es Send).
struct Icons {
    prev: usize,
    play: usize,
    pause: usize,
    next: usize,
}

static ICONS: OnceLock<Icons> = OnceLock::new();
static ON_CLICK: OnceLock<Box<dyn Fn(u32) + Send + Sync>> = OnceLock::new();
static PLAYING: AtomicBool = AtomicBool::new(false);
static CURRENT: std::sync::Mutex<usize> = std::sync::Mutex::new(0);

/// Qué hacer al pulsar un botón (una sola vez al arrancar).
pub fn set_handler(f: impl Fn(u32) + Send + Sync + 'static) {
    let _ = ON_CLICK.set(Box::new(f));
}

/// Engancha la ventana principal (cada vez que se crea). Hilo de UI.
pub fn attach(hwnd_raw: isize) {
    let hwnd = HWND(hwnd_raw as *mut _);
    *CURRENT.lock().unwrap() = hwnd_raw as usize;
    // SAFETY: hwnd es la ventana principal viva; el procedimiento es estático.
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(subclass), SUBCLASS_ID, 0);
    }
    add_buttons(hwnd);
}

/// Refleja reproducir/pausa en el botón central. Hilo de UI.
pub fn set_playing(playing: bool) {
    if PLAYING.swap(playing, Ordering::AcqRel) == playing {
        return;
    }
    let raw = *CURRENT.lock().unwrap();
    if raw == 0 {
        return;
    }
    let hwnd = HWND(raw as *mut _);
    let Some(icons) = icons() else { return };
    let b = button(
        BTN_TOGGLE,
        if playing { icons.pause } else { icons.play },
        if playing { "Pausa" } else { "Reproducir" },
    );
    // SAFETY: COM ya está inicializado en el hilo de UI (lo usa WebView2).
    unsafe {
        if let Ok(tb) =
            CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER)
        {
            let _ = tb.HrInit();
            let _ = tb.ThumbBarUpdateButtons(hwnd, &[b]);
        }
    }
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
    // SAFETY: cadena estática terminada en 0.
    let created = *TASKBAR_CREATED
        .get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarButtonCreated")) });
    if msg == created {
        add_buttons(hwnd);
    } else if msg == WM_COMMAND && (wparam.0 >> 16) & 0xffff == THBN_CLICKED {
        if let Some(f) = ON_CLICK.get() {
            f((wparam.0 & 0xffff) as u32);
        }
        return LRESULT(0);
    }
    // SAFETY: reenvío estándar de una subclase.
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

fn add_buttons(hwnd: HWND) {
    let Some(icons) = icons() else { return };
    let playing = PLAYING.load(Ordering::Acquire);
    let buttons = [
        button(BTN_PREV, icons.prev, "Anterior"),
        button(
            BTN_TOGGLE,
            if playing { icons.pause } else { icons.play },
            if playing { "Pausa" } else { "Reproducir" },
        ),
        button(BTN_NEXT, icons.next, "Siguiente"),
    ];
    // SAFETY: COM inicializado en el hilo de UI; si la barra aún no está lista, falla sin más
    // y se reintenta al llegar "TaskbarButtonCreated".
    unsafe {
        if let Ok(tb) =
            CoCreateInstance::<_, ITaskbarList3>(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            && tb.HrInit().is_ok()
            && tb.ThumbBarAddButtons(hwnd, &buttons).is_err()
        {
            let _ = tb.ThumbBarUpdateButtons(hwnd, &buttons);
        }
    }
}

fn button(id: u32, icon: usize, tip: &str) -> THUMBBUTTON {
    let mut sz = [0u16; 260];
    for (d, s) in sz.iter_mut().zip(tip.encode_utf16()) {
        *d = s;
    }
    THUMBBUTTON {
        dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
        iId: id,
        iBitmap: 0,
        hIcon: HICON(icon as *mut _),
        szTip: sz,
        dwFlags: THBF_ENABLED,
    }
}

fn icons() -> Option<&'static Icons> {
    if let Some(i) = ICONS.get() {
        return Some(i);
    }
    let make = |f: fn(f32, f32) -> bool| make_icon(f).map(|h| h.0 as usize);
    let icons = Icons {
        prev: make(glyph_prev)?,
        play: make(glyph_play)?,
        pause: make(glyph_pause)?,
        next: make(glyph_next)?,
    };
    Some(ICONS.get_or_init(|| icons))
}

// ------------------------------------------------------------------ glifos 16×16

fn glyph_play(x: f32, y: f32) -> bool {
    x >= 4.5 && (y - 8.0).abs() <= (12.5 - x) * 0.62
}

fn glyph_pause(x: f32, y: f32) -> bool {
    (3.0..=13.0).contains(&y) && ((4.0..=6.5).contains(&x) || (9.5..=12.0).contains(&x))
}

fn glyph_prev(x: f32, y: f32) -> bool {
    let bar = (3.0..=4.8).contains(&x) && (4.0..=12.0).contains(&y);
    let tri = (5.5..=12.5).contains(&x) && (y - 8.0).abs() <= (x - 5.5) * 0.57;
    bar || tri
}

fn glyph_next(x: f32, y: f32) -> bool {
    glyph_prev(16.0 - x, y)
}

/// Icono 16×16 de 32 bits: glifo blanco con contorno azul oscuro.
fn make_icon(shape: fn(f32, f32) -> bool) -> Option<HICON> {
    const N: i32 = 16;
    let inside = |x: i32, y: i32| {
        (0..N).contains(&x) && (0..N).contains(&y) && shape(x as f32 + 0.5, y as f32 + 0.5)
    };
    let mut px = vec![0u32; (N * N) as usize];
    for y in 0..N {
        for x in 0..N {
            let v = if inside(x, y) {
                0xFFF8_FAFF // ARGB blanco azulado
            } else if (-1..=1).any(|dy| (-1..=1).any(|dx| inside(x + dx, y + dy))) {
                0xE01A_2C55 // contorno
            } else {
                0
            };
            px[(y * N + x) as usize] = v;
        }
    }
    // SAFETY: llamadas GDI con estructuras válidas; los bitmaps se liberan tras crear el icono.
    unsafe {
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: N,
                biHeight: -N,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let color = CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        std::ptr::copy_nonoverlapping(px.as_ptr(), bits as *mut u32, px.len());
        let mask_bits = [0u8; (N * N / 8) as usize];
        let mask = CreateBitmap(N, N, 1, 1, Some(mask_bits.as_ptr() as *const _));
        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(HGDIOBJ(color.0));
        let _ = DeleteObject(HGDIOBJ(mask.0));
        icon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glifos_razonables() {
        let count = |f: fn(f32, f32) -> bool| {
            (0..16)
                .flat_map(|y| (0..16).map(move |x| (x, y)))
                .filter(|&(x, y)| f(x as f32 + 0.5, y as f32 + 0.5))
                .count()
        };
        for (name, f) in [
            ("play", glyph_play as fn(f32, f32) -> bool),
            ("pause", glyph_pause),
            ("prev", glyph_prev),
            ("next", glyph_next),
        ] {
            let n = count(f);
            assert!((20..120).contains(&n), "{name}: {n} píxeles");
        }
        assert_eq!(
            count(glyph_prev),
            count(glyph_next),
            "siguiente es el espejo de anterior"
        );
    }

    #[test]
    fn crea_iconos() {
        assert!(make_icon(glyph_play).is_some());
    }
}
