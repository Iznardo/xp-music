//! Salida de audio con cpal (FR-002-04/08/10, NFR-002-04).
//!
//! El stream está siempre en marcha; pausa, volumen y vaciado se controlan con
//! atómicos. El callback de tiempo real no bloquea, no reserva memoria ni toma
//! mutex: solo lee atómicos y mueve muestras entre ring buffers SPSC.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::{HeapCons, HeapProd, HeapRb};

use crate::error::{AppError, ErrorCode};
use crate::player::types::AudioDevice;

/// Colchón entre decodificador y tarjeta: 0,5 s.
const RING_SECONDS: f32 = 0.5;
/// Lo que guarda la derivación para el analizador (005): 1 s.
const TAP_SECONDS: f32 = 1.0;

/// Estado compartido con el callback.
pub struct OutputShared {
    volume_bits: AtomicU32,
    pub paused: AtomicBool,
    flush_requested: AtomicBool,
    /// Frames enviados a la tarjeta desde que se abrió la salida.
    pub played_frames: AtomicU64,
    /// Frames escritos en el ring por el motor (lo actualiza el motor).
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

    /// Amplitud lineal final (volumen perceptual × ganancia de normalización).
    pub fn set_amplitude(&self, amp: f32) {
        self.volume_bits.store(amp.to_bits(), Ordering::Relaxed);
    }

    fn amplitude(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }
}

/// Curva perceptual: el oído percibe el volumen de forma logarítmica.
pub fn perceptual_amplitude(volume: f32, muted: bool, gain_db: f32) -> f32 {
    if muted {
        return 0.0;
    }
    let v = volume.clamp(0.0, 1.0);
    v * v * v * 10f32.powf(gain_db.clamp(-10.0, 10.0) / 20.0)
}

/// Derivación de PCM para el analizador (005).
#[allow(dead_code)] // La consume el analizador de 005.
pub struct PcmTap {
    pub consumer: HeapCons<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub struct Output {
    _stream: cpal::Stream,
    prod: HeapProd<f32>,
    pub shared: Arc<OutputShared>,
    pub rate: u32,
    pub channels: u16,
}

impl Output {
    /// Abre el dispositivo `device_id` (o el predeterminado). `tap_slot` recibe
    /// la nueva derivación de PCM.
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
                    "No hay ningún dispositivo de audio disponible.",
                )
            })?;
        let device_name = device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| "?".into());
        let config = device.default_output_config().map_err(|e| {
            tracing::error!(error = %e, "sin configuración de salida");
            AppError::new(
                ErrorCode::AudioDevice,
                "No se pudo abrir el dispositivo de audio.",
            )
        })?;
        if config.sample_format() != cpal::SampleFormat::F32 {
            tracing::error!(format = ?config.sample_format(), "formato de salida no soportado");
            return Err(AppError::new(
                ErrorCode::AudioDevice,
                "El dispositivo de audio usa un formato no soportado.",
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
                    if cb.paused.load(Ordering::Relaxed) {
                        data.fill(0.0);
                        return;
                    }
                    let n = cons.pop_slice(data);
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
                        if cb.tap_enabled.load(Ordering::Relaxed) {
                            tap_prod.push_slice(&data[..n]);
                        }
                    }
                    if n < data.len() && n > 0 {
                        cb.underruns.fetch_add(1, Ordering::Relaxed);
                    }
                },
                move |err| {
                    tracing::warn!(error = %err, "error en el stream de audio");
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
                tracing::error!(error = %e, "no se pudo crear el stream");
                AppError::new(
                    ErrorCode::AudioDevice,
                    "No se pudo abrir el dispositivo de audio.",
                )
            })?;
        stream.play().map_err(|e| {
            tracing::error!(error = %e, "no se pudo iniciar el stream");
            AppError::new(
                ErrorCode::AudioDevice,
                "No se pudo iniciar la salida de audio.",
            )
        })?;

        *tap_slot.lock().unwrap() = Some(PcmTap {
            consumer: tap_cons,
            sample_rate: rate,
            channels,
        });
        tracing::info!(device = %device_name, rate, channels, "salida de audio abierta");
        Ok(Self {
            _stream: stream,
            prod,
            shared,
            rate,
            channels,
        })
    }

    /// Espacio libre en el ring, en muestras.
    pub fn free_samples(&self) -> usize {
        self.prod.vacant_len()
    }

    /// Muestras pendientes de sonar en el ring.
    pub fn queued_samples(&self) -> usize {
        self.prod.occupied_len()
    }

    /// Escribe lo que quepa y devuelve cuántas muestras se escribieron.
    pub fn push(&mut self, samples: &[f32]) -> usize {
        let n = self.prod.push_slice(samples);
        self.shared
            .pushed_frames
            .fetch_add((n / self.channels as usize) as u64, Ordering::AcqRel);
        n
    }

    /// Vacía el ring (seek, cambio de pista) y espera a que el callback lo confirme.
    pub fn flush(&mut self) {
        self.shared.flush_requested.store(true, Ordering::Release);
        let start = Instant::now();
        while self.shared.flush_requested.load(Ordering::Acquire)
            && start.elapsed() < Duration::from_millis(200)
        {
            std::thread::sleep(Duration::from_millis(2));
        }
        if self.shared.flush_requested.swap(false, Ordering::AcqRel) {
            // El callback no corrió (dispositivo parado): se alinean los contadores aquí.
            tracing::debug!("flush sin confirmación del callback");
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

    #[test]
    fn curva_de_volumen() {
        assert_eq!(perceptual_amplitude(1.0, false, 0.0), 1.0);
        assert_eq!(perceptual_amplitude(0.0, false, 0.0), 0.0);
        assert_eq!(perceptual_amplitude(1.0, true, 0.0), 0.0);
        assert!((perceptual_amplitude(0.5, false, 0.0) - 0.125).abs() < 1e-6);
        // +6 dB ≈ ×2, y la ganancia se limita a ±10 dB.
        assert!((perceptual_amplitude(1.0, false, 6.0) - 1.995).abs() < 0.01);
        assert!((perceptual_amplitude(1.0, false, 40.0) - 3.162).abs() < 0.01);
    }
}
