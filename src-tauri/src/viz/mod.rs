//! Visualizations (specs/005): analyzer over the engine's PCM tap.
//!
//! It only runs while the UI asks for it (`viz_set_active`): with the view hidden
//! no audio is copied and nothing is computed (NFR-005-02).

pub mod bands;
pub mod custom;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use ringbuf::traits::{Consumer, Observer};
use serde::Serialize;
use specta::Type;
use tauri::AppHandle;
use tauri_specta::Event;

use crate::audio::output::PcmTap;
use crate::player::service::PlayerService;
use bands::{Analyzer, FFT_SIZE};

const FRAME: Duration = Duration::from_millis(33);

#[derive(Serialize, Type, Event, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "viz://frame")]
pub struct VizFrame {
    /// 64 values 0..255 (0..1 quantized), from bass to treble. One byte is enough to
    /// draw, and the JSON sent 30 times per second takes ~3 times less.
    pub bands: Vec<u8>,
    pub peaks: Vec<u8>,
    /// 256 samples -127..127 (-1..1 quantized).
    pub wave: Vec<i8>,
    /// Smoothed RMS level 0..1.
    pub level: f32,
    /// Rhythm pulse 0..1 (1 on every bass beat, falls in ~0.2 s).
    pub beat: f32,
    /// Bass normalized to the song 0..1.
    pub bass: f32,
}

pub struct VizService {
    app: AppHandle,
    player: Arc<PlayerService>,
    active: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
    /// The PCM tap is only delivered once per opened output: it is kept
    /// here between activations (switching preset, leaving and returning to Now
    /// Playing). Without this, the second time the analyzer ran out of audio.
    tap: Arc<Mutex<Option<PcmTap>>>,
}

impl VizService {
    pub fn new(app: AppHandle, player: Arc<PlayerService>) -> Self {
        Self {
            app,
            player,
            active: Arc::new(AtomicBool::new(false)),
            worker: Mutex::new(None),
            tap: Arc::new(Mutex::new(None)),
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
            let stored = Arc::clone(&self.tap);
            *worker = thread::Builder::new()
                .name("viz-analyzer".into())
                .spawn(move || run(active, player, app, stored))
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

fn frame_of(analyzer: &Analyzer) -> VizFrame {
    let u = |x: &f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    VizFrame {
        bands: analyzer.bands.iter().map(u).collect(),
        peaks: analyzer.peaks.iter().map(u).collect(),
        wave: analyzer
            .wave
            .iter()
            .map(|x| (x.clamp(-1.0, 1.0) * 127.0).round() as i8)
            .collect(),
        level: analyzer.level,
        beat: analyzer.beat,
        bass: analyzer.bass,
    }
}

fn run(
    active: Arc<AtomicBool>,
    player: Arc<PlayerService>,
    app: AppHandle,
    stored: Arc<Mutex<Option<PcmTap>>>,
) {
    let mut tap = player.take_tap().or_else(|| stored.lock().unwrap().take());
    let mut analyzer = Analyzer::new(tap.as_ref().map_or(48_000, |t| t.sample_rate));
    // Mono sliding window of FFT_SIZE samples (ring + linear copy).
    let mut ring = vec![0.0f32; FFT_SIZE];
    let mut write = 0usize;
    let mut linear = vec![0.0f32; FFT_SIZE];
    let mut chunk = vec![0.0f32; 8192];
    let mut was_silent = false;
    // Diagnostics: one line 5 s after activation (does audio arrive? are there beats?).
    let activated = Instant::now();
    let (mut reported, mut samples_seen, mut beats) = (false, 0usize, 0u32);

    // Old tap data is of no interest.
    if let Some(t) = tap.as_mut() {
        t.consumer.clear();
    }

    while active.load(Ordering::Acquire) {
        let started = Instant::now();
        // The device may change: then there is a new tap.
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
        samples_seen += got;
        if got > 0 {
            linear[..FFT_SIZE - write].copy_from_slice(&ring[write..]);
            linear[FFT_SIZE - write..].copy_from_slice(&ring[..write]);
            analyzer.analyze(&linear);
        } else {
            analyzer.decay();
        }

        if analyzer.beat >= 1.0 {
            beats += 1;
        }
        if !reported && activated.elapsed() >= Duration::from_secs(5) {
            reported = true;
            tracing::info!(
                with_audio = tap.is_some(),
                samples = samples_seen,
                beats = beats,
                "visualization active"
            );
        }

        // During sustained silence nothing is emitted (saves IPC).
        let silent = analyzer.is_silent();
        if !(silent && was_silent) {
            let _ = frame_of(&analyzer).emit(&app);
        }
        was_silent = silent;

        if let Some(left) = FRAME.checked_sub(started.elapsed()) {
            thread::sleep(left);
        }
    }
    // Handed back for the next activation.
    *stored.lock().unwrap() = tap;
}

#[cfg(test)]
mod size_tests {
    use super::*;

    /// Size of a frame as it travels to the interface (30 per second).
    #[test]
    fn frame_size() {
        let mut a = bands::Analyzer::new(48_000);
        let x: Vec<f32> = (0..bands::FFT_SIZE)
            .map(|i| (i as f32 * 0.07).sin() * 0.6 + (i as f32 * 0.31).sin() * 0.2)
            .collect();
        for _ in 0..10 {
            a.analyze(&x);
        }
        let f = frame_of(&a);
        let json = serde_json::to_string(&f).unwrap();
        println!(
            "frame: {} bytes ({} KB/s at 30 fps)",
            json.len(),
            json.len() * 30 / 1024
        );
    }
}
