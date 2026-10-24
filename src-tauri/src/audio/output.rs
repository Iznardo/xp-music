//! Audio output with cpal (FR-002-04/08/10, NFR-002-04).
//!
//! The stream is always running; pause, volume and flushing are controlled with
//! atomics. The real-time callback does not block, allocate memory or take any
//! mutex: it only reads atomics and moves samples between SPSC ring buffers.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use crate::error::{AppError, ErrorCode};
use crate::player::types::AudioDevice;

/// Cushion between decoder and sound card: 0.5 s.
const RING_SECONDS: f32 = 0.5;
/// What the tap keeps for the analyzer (005): 1 s.
const TAP_SECONDS: f32 = 1.0;

/// State shared with the callback.
pub struct OutputShared {
    volume_bits: AtomicU32,
    pub paused: AtomicBool,
    flush_requested: AtomicBool,
    /// Frames sent to the sound card since the output was opened.
    pub played_frames: AtomicU64,
    /// Frames written to the ring by the engine (updated by the engine).
    pub pushed_frames: AtomicU64,
    pub underruns: AtomicU64,
    pub tap_enabled: AtomicBool,
    pub device_lost: AtomicBool,
}

impl OutputShared {
    fn new() -> Self {
        Self {
            volume_bits: AtomicU32::new(1.0f32.to_bits()),
            paused: AtomicBool::new(true),
            flush_requested: AtomicBool::new(false),
            played_frames: AtomicU64::new(0),
            pushed_frames: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            tap_enabled: AtomicBool::new(false),
            device_lost: AtomicBool::new(false),
        }
    }

    /// Final linear amplitude (perceptual volume × normalization gain).
    pub fn set_amplitude(&self, amp: f32) {
        self.volume_bits.store(amp.to_bits(), Ordering::Relaxed);
    }

    fn amplitude(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }
}

/// Perceptual curve: the ear perceives loudness logarithmically.
pub fn perceptual_amplitude(volume: f32, muted: bool, gain_db: f32) -> f32 {
    if muted {
        return 0.0;
    }
    let v = volume.clamp(0.0, 1.0);
    v * v * v * 10f32.powf(gain_db.clamp(-10.0, 10.0) / 20.0)
}

/// PCM tap for the analyzer (005).
pub struct PcmTap {
    pub consumer: HeapCons<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct Output {
    _stream: cpal::Stream,
    prod: HeapProd<f32>,
    /// UI sounds: mixed on top of the music (or of silence when paused).
    fx: HeapProd<f32>,
    pub shared: Arc<OutputShared>,
    pub rate: u32,
    pub channels: u16,
}

impl Output {
    /// Opens the `device_id` device (or the default one). `tap_slot` receives
    /// the new PCM tap.
    pub fn open(
        device_id: Option<&str>,
        tap_slot: &Mutex<Option<PcmTap>>,
    ) -> Result<Self, AppError> {
        let host = cpal::default_host();
        let device = device_id
            .and_then(|id| id.parse().ok().and_then(|id| host.device_by_id(&id)))
            .or_else(|| host.default_output_device())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::AudioDevice,
                    tr!(
                        "No audio device is available.",
                        "No hay ningún dispositivo de audio disponible."
                    ),
                )
            })?;
        let device_name = device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| "?".into());
        let config = device.default_output_config().map_err(|e| {
            tracing::error!(error = %e, "no output configuration");
            AppError::new(
                ErrorCode::AudioDevice,
                tr!(
                    "Could not open the audio device.",
                    "No se pudo abrir el dispositivo de audio."
                ),
            )
        })?;
        if config.sample_format() != cpal::SampleFormat::F32 {
            tracing::error!(format = ?config.sample_format(), "unsupported output format");
            return Err(AppError::new(
                ErrorCode::AudioDevice,
                tr!(
                    "The audio device uses an unsupported format.",
                    "El dispositivo de audio usa un formato no soportado."
                ),
            ));
        }
        let config: cpal::StreamConfig = config.into();
        let rate = config.sample_rate;
        let channels = config.channels;
        let ch = channels as usize;

        let ring_len = ((rate as f32 * RING_SECONDS) as usize * ch).max(1024);
        let (prod, mut cons) = HeapRb::<f32>::new(ring_len).split();
        let (mut tap_prod, tap_cons) =
            HeapRb::<f32>::new(((rate as f32 * TAP_SECONDS) as usize * ch).max(1024)).split();
        let (fx, mut fx_cons) = HeapRb::<f32>::new((rate as usize * ch).max(1024)).split();

        let shared = Arc::new(OutputShared::new());
        let cb = Arc::clone(&shared);
        let err_shared = Arc::clone(&shared);
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if cb.flush_requested.load(Ordering::Acquire) {
                        cons.clear();
                        cb.played_frames
                            .store(cb.pushed_frames.load(Ordering::Acquire), Ordering::Release);
                        cb.flush_requested.store(false, Ordering::Release);
                    }
                    let n = if cb.paused.load(Ordering::Relaxed) {
                        0
                    } else {
                        cons.pop_slice(data)
                    };
                    // The visualization sees the signal before the volume, so it reacts
                    // just as strongly with low volume (or muted).
                    if n > 0 && cb.tap_enabled.load(Ordering::Relaxed) {
                        tap_prod.push_slice(&data[..n]);
                    }
                    let amp = cb.amplitude();
                    if amp != 1.0 {
                        for s in &mut data[..n] {
                            *s *= amp;
                        }
                    }
                    data[n..].fill(0.0);
                    if n > 0 {
                        cb.played_frames
                            .fetch_add((n / ch) as u64, Ordering::AcqRel);
                    }
                    if n < data.len() && n > 0 {
                        cb.underruns.fetch_add(1, Ordering::Relaxed);
                    }
                    // UI effects, bypassing the music volume.
                    for s in data.iter_mut() {
                        match fx_cons.try_pop() {
                            Some(x) => *s += x,
                            None => break,
                        }
                    }
                },
                move |err| {
                    tracing::warn!(error = %err, "audio stream error");
                    if matches!(
                        err.kind(),
                        cpal::ErrorKind::DeviceNotAvailable | cpal::ErrorKind::DeviceChanged
                    ) {
                        err_shared.device_lost.store(true, Ordering::Release);
                    }
                },
                None,
            )
            .map_err(|e| {
                tracing::error!(error = %e, "could not create the stream");
                AppError::new(
                    ErrorCode::AudioDevice,
                    tr!(
                        "Could not open the audio device.",
                        "No se pudo abrir el dispositivo de audio."
                    ),
                )
            })?;
        stream.play().map_err(|e| {
            tracing::error!(error = %e, "could not start the stream");
            AppError::new(
                ErrorCode::AudioDevice,
                tr!(
                    "Could not start audio output.",
                    "No se pudo iniciar la salida de audio."
                ),
            )
        })?;

        *tap_slot.lock().unwrap() = Some(PcmTap {
            consumer: tap_cons,
            sample_rate: rate,
            channels,
        });
        tracing::info!(device = %device_name, rate, channels, "audio output opened");
        Ok(Self {
            _stream: stream,
            prod,
            fx,
            shared,
            rate,
            channels,
        })
    }

    /// Free space in the ring, in samples.
    pub fn free_samples(&self) -> usize {
        self.prod.vacant_len()
    }

    /// Samples in the ring still waiting to be played.
    pub fn queued_samples(&self) -> usize {
        self.prod.occupied_len()
    }

    /// Queues a UI sound (already in the output format). If it does not fit, it is truncated.
    pub fn play_fx(&mut self, samples: &[f32]) {
        self.fx.push_slice(samples);
    }

    /// Writes what fits and returns how many samples were written.
    pub fn push(&mut self, samples: &[f32]) -> usize {
        let n = self.prod.push_slice(samples);
        self.shared
            .pushed_frames
            .fetch_add((n / self.channels as usize) as u64, Ordering::AcqRel);
        n
    }

    /// Empties the ring (seek, track change) and waits for the callback to confirm.
    pub fn flush(&mut self) {
        self.shared.flush_requested.store(true, Ordering::Release);
        let start = Instant::now();
        while self.shared.flush_requested.load(Ordering::Acquire)
            && start.elapsed() < Duration::from_millis(200)
        {
            std::thread::sleep(Duration::from_millis(2));
        }
        if self.shared.flush_requested.swap(false, Ordering::AcqRel) {
            // The callback did not run (device stopped): align the counters here.
            tracing::debug!("flush without callback confirmation");
            self.shared.played_frames.store(
                self.shared.pushed_frames.load(Ordering::Acquire),
                Ordering::Release,
            );
        }
    }

    pub fn played_frames(&self) -> u64 {
        self.shared.played_frames.load(Ordering::Acquire)
    }

    pub fn pushed_frames(&self) -> u64 {
        self.shared.pushed_frames.load(Ordering::Acquire)
    }
}

pub fn list_devices() -> Vec<AudioDevice> {
    let host = cpal::default_host();
    let default_id = host
        .default_output_device()
        .and_then(|d| d.id().ok())
        .map(|id| id.to_string());
    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    devices
        .filter_map(|d| {
            let id = d.id().ok()?.to_string();
            let name = d
                .description()
                .map(|x| x.name().to_owned())
                .unwrap_or_else(|_| id.clone());
            Some(AudioDevice {
                is_default: Some(&id) == default_id.as_ref(),
                id,
                name,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real output test: requires an ALSA/Pulse/PipeWire device.
    #[test]
    #[ignore = "requires an audio device"]
    fn real_audio_output() {
        let tap = Mutex::new(None);
        let mut output = Output::open(None, &tap).unwrap();
        output.shared.tap_enabled.store(true, Ordering::Relaxed);
        output.shared.paused.store(false, Ordering::Relaxed);
        let samples: Vec<f32> = (0..output.rate / 10)
            .flat_map(|frame| {
                let sample = (frame as f32 * 440.0 * std::f32::consts::TAU / output.rate as f32)
                    .sin()
                    * 0.01;
                std::iter::repeat_n(sample, output.channels as usize)
            })
            .collect();
        assert_eq!(output.push(&samples), samples.len());
        let deadline = Instant::now() + Duration::from_secs(5);
        while output.played_frames() == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(output.played_frames() > 0, "the callback must consume PCM");
        assert!(
            tap.lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .consumer
                .occupied_len()
                > 0
        );
        assert!(!output.shared.device_lost.load(Ordering::Relaxed));
    }

    #[test]
    fn volume_curve() {
        assert_eq!(perceptual_amplitude(1.0, false, 0.0), 1.0);
        assert_eq!(perceptual_amplitude(0.0, false, 0.0), 0.0);
        assert_eq!(perceptual_amplitude(1.0, true, 0.0), 0.0);
        assert!((perceptual_amplitude(0.5, false, 0.0) - 0.125).abs() < 1e-6);
        // +6 dB ≈ ×2, and the gain is limited to ±10 dB.
        assert!((perceptual_amplitude(1.0, false, 6.0) - 1.995).abs() < 0.01);
        assert!((perceptual_amplitude(1.0, false, 40.0) - 3.162).abs() < 0.01);
    }
}
