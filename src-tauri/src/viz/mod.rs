//! Visualizaciones (specs/005): analizador sobre la derivación de PCM del motor.
//!
//! Solo funciona mientras la UI lo pide (`viz_set_active`): con la vista oculta
//! no se copia audio ni se calcula nada (NFR-005-02).

pub mod bands;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ringbuf::traits::{Consumer, Observer};
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::player::service::PlayerService;
use bands::{Analyzer, FFT_SIZE};

const FRAME: Duration = Duration::from_millis(33);

#[derive(Serialize, Type, Event, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "viz://frame")]
pub struct VizFrame {
    /// 64 valores 0..1, de graves a agudos.
    pub bands: Vec<f32>,
    pub peaks: Vec<f32>,
    /// 256 muestras -1..1.
    pub wave: Vec<f32>,
    /// Nivel RMS suavizado 0..1.
    pub level: f32,
}

pub struct VizService {
    app: AppHandle,
    player: Arc<PlayerService>,
    active: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

impl VizService {
    pub fn new(app: AppHandle, player: Arc<PlayerService>) -> Self {
        Self {
            app,
            player,
            active: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
        }
    }

    pub fn set_active(&self, on: bool) {
        let mut worker = self.worker.lock().unwrap();
        if on {
            if self.active.swap(true, Ordering::AcqRel) {
                return;
            }
            self.player.set_tap(true);
            let active = Arc::clone(&self.active);
            let player = Arc::clone(&self.player);
            let app = self.app.clone();
            *worker = thread::Builder::new()
                .name("viz-analyzer".into())
                .spawn(move || run(active, player, app))
                .ok();
        } else {
            self.active.store(false, Ordering::Release);
            self.player.set_tap(false);
            if let Some(h) = worker.take() {
                let _ = h.join();
            }
        }
    }
}

fn run(active: Arc<AtomicBool>, player: Arc<PlayerService>, app: AppHandle) {
    let mut tap = player.take_tap();
    let mut analyzer = Analyzer::new(tap.as_ref().map_or(48_000, |t| t.sample_rate));
    // Ventana deslizante mono de FFT_SIZE muestras (ring + copia lineal).
    let mut ring = vec![0.0f32; FFT_SIZE];
    let mut write = 0usize;
    let mut linear = vec![0.0f32; FFT_SIZE];
    let mut chunk = vec![0.0f32; 8192];
    let mut was_silent = false;

    // Lo viejo del tap no interesa.
    if let Some(t) = tap.as_mut() {
        t.consumer.clear();
    }

    while active.load(Ordering::Acquire) {
        let started = Instant::now();
        // El dispositivo puede cambiar: entonces hay derivación nueva.
        if let Some(new) = player.take_tap() {
            analyzer = Analyzer::new(new.sample_rate);
            tap = Some(new);
        }
        let mut got = 0usize;
        if let Some(t) = tap.as_mut() {
            let ch = t.channels.max(1) as usize;
            while t.consumer.occupied_len() >= ch {
                let usable = (chunk.len() / ch) * ch;
                let n = t.consumer.pop_slice(&mut chunk[..usable]);
                for frame in chunk[..n].chunks_exact(ch) {
                    let m = if ch >= 2 {
                        (frame[0] + frame[1]) * 0.5
                    } else {
                        frame[0]
                    };
                    ring[write] = m;
                    write = (write + 1) % FFT_SIZE;
                }
                got += n / ch;
            }
        }
        if got > 0 {
            linear[..FFT_SIZE - write].copy_from_slice(&ring[write..]);
            linear[FFT_SIZE - write..].copy_from_slice(&ring[..write]);
            analyzer.analyze(&linear);
        } else {
            analyzer.decay();
        }

        // En silencio sostenido no se emite nada (ahorro de IPC).
        let silent = analyzer.is_silent();
        if !(silent && was_silent) {
            let frame = VizFrame {
                bands: analyzer.bands.to_vec(),
                peaks: analyzer.peaks.to_vec(),
                wave: analyzer.wave.to_vec(),
                level: analyzer.level,
            };
            let _ = frame.emit(&app);
        }
        was_silent = silent;

        if let Some(left) = FRAME.checked_sub(started.elapsed()) {
            thread::sleep(left);
        }
    }
}
