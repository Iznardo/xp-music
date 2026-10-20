//! Análisis espectral: ventana de Hann, FFT y 64 bandas logarítmicas (FR-005-01/02).
//! Lógica pura y sin asignaciones en `analyze` (buffers preasignados).

use std::sync::Arc;

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

pub const FFT_SIZE: usize = 2048;
pub const BANDS: usize = 64;
pub const WAVE: usize = 256;
const MIN_HZ: f32 = 20.0;
const MAX_HZ: f32 = 16_000.0;
/// Rango dinámico representado: -70 dB → 0, 0 dB → 1.
const FLOOR_DB: f32 = -70.0;
const ATTACK: f32 = 0.6;
const RELEASE: f32 = 0.15;
const PEAK_FALL: f32 = 0.01;

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<Complex32>,
    scratch: Vec<Complex32>,
    /// Rango de bins [inicio, fin) de cada banda.
    ranges: Vec<(usize, usize)>,
    pub bands: [f32; BANDS],
    pub peaks: [f32; BANDS],
    pub wave: [f32; WAVE],
    pub level: f32,
}

impl Analyzer {
    pub fn new(sample_rate: u32) -> Self {
        let fft = FftPlanner::new().plan_fft_forward(FFT_SIZE);
        let window = (0..FFT_SIZE)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (FFT_SIZE - 1) as f32).cos())
            .collect();
        let scratch = vec![Complex32::default(); fft.get_inplace_scratch_len()];
        Self {
            fft,
            window,
            buf: vec![Complex32::default(); FFT_SIZE],
            scratch,
            ranges: band_ranges(sample_rate),
            bands: [0.0; BANDS],
            peaks: [0.0; BANDS],
            wave: [0.0; WAVE],
            level: 0.0,
        }
    }

    /// Analiza las últimas `FFT_SIZE` muestras mono y actualiza bandas, picos,
    /// forma de onda y nivel con suavizado.
    pub fn analyze(&mut self, mono: &[f32]) {
        debug_assert_eq!(mono.len(), FFT_SIZE);
        let mut sum_sq = 0.0;
        for (i, (&x, w)) in mono.iter().zip(&self.window).enumerate() {
            self.buf[i] = Complex32::new(x * w, 0.0);
            sum_sq += x * x;
        }
        self.fft
            .process_with_scratch(&mut self.buf, &mut self.scratch);

        // Normalización: un seno de amplitud 1 con Hann da un pico de ~N/4.
        let norm = 4.0 / FFT_SIZE as f32;
        for (b, &(lo, hi)) in self.ranges.iter().enumerate() {
            let mag = self.buf[lo..hi]
                .iter()
                .map(|c| c.norm())
                .fold(0.0f32, f32::max)
                * norm;
            let db = 20.0 * mag.max(1e-9).log10();
            let target = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
            let v = self.bands[b];
            let k = if target > v { ATTACK } else { RELEASE };
            self.bands[b] = v + (target - v) * k;
            self.peaks[b] = (self.peaks[b] - PEAK_FALL).max(self.bands[b]);
        }

        // Forma de onda: las últimas WAVE*4 muestras, decimadas.
        let step = 4;
        let start = FFT_SIZE - WAVE * step;
        for (i, w) in self.wave.iter_mut().enumerate() {
            *w = mono[start + i * step];
        }
        let rms = (sum_sq / FFT_SIZE as f32).sqrt();
        self.level += ((rms * 2.0).min(1.0) - self.level) * 0.3;
    }

    /// Sin audio (pausa): todo cae suavemente a cero.
    pub fn decay(&mut self) {
        for b in 0..BANDS {
            self.bands[b] *= 1.0 - RELEASE;
            self.peaks[b] = (self.peaks[b] - PEAK_FALL).max(self.bands[b]);
        }
        self.wave = [0.0; WAVE];
        self.level *= 0.7;
    }

    pub fn is_silent(&self) -> bool {
        self.peaks.iter().all(|p| *p < 0.001) && self.level < 0.001
    }
}

/// Bandas logarítmicas de 20 Hz a 16 kHz. Las graves, más estrechas que un bin,
/// usan al menos el bin más cercano.
fn band_ranges(sample_rate: u32) -> Vec<(usize, usize)> {
    let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
    let max_bin = FFT_SIZE / 2;
    let ratio = (MAX_HZ / MIN_HZ).powf(1.0 / BANDS as f32);
    (0..BANDS)
        .map(|b| {
            let f_lo = MIN_HZ * ratio.powi(b as i32);
            let f_hi = f_lo * ratio;
            let lo = ((f_lo / bin_hz).round() as usize).clamp(1, max_bin - 1);
            let hi = ((f_hi / bin_hz).round() as usize).clamp(lo + 1, max_bin);
            (lo, hi)
        })
        .collect()
}

/// Frecuencia central de una banda (para tests y etiquetas).
#[cfg(test)]
pub fn band_center(b: usize) -> f32 {
    let ratio = (MAX_HZ / MIN_HZ).powf(1.0 / BANDS as f32);
    MIN_HZ * ratio.powf(b as f32 + 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: f32, amp: f32) -> Vec<f32> {
        (0..FFT_SIZE)
            .map(|i| amp * (std::f32::consts::TAU * freq * i as f32 / rate).sin())
            .collect()
    }

    fn settle(a: &mut Analyzer, x: &[f32]) {
        for _ in 0..20 {
            a.analyze(x);
        }
    }

    #[test]
    fn seno_de_1khz_activa_su_banda() {
        let mut a = Analyzer::new(48_000);
        settle(&mut a, &sine(1000.0, 48_000.0, 0.8));
        let target = (0..BANDS)
            .min_by(|&x, &y| {
                (band_center(x) - 1000.0)
                    .abs()
                    .total_cmp(&(band_center(y) - 1000.0).abs())
            })
            .unwrap();
        // La banda puede caer justo al lado por redondeo de bins: se mira su vecindad.
        let near = a.bands[target.saturating_sub(1)..=(target + 1).min(BANDS - 1)]
            .iter()
            .fold(0.0f32, |m, x| m.max(*x));
        assert!(near > 0.8, "banda de 1 kHz = {near}");
        for (b, v) in a.bands.iter().enumerate() {
            if (band_center(b) / 1000.0).log2().abs() > 2.0 {
                assert!(*v < 0.1, "banda {b} ({} Hz) = {v}", band_center(b));
            }
        }
    }

    #[test]
    fn silencio_da_cero_y_decay_lo_mantiene() {
        let mut a = Analyzer::new(44_100);
        settle(&mut a, &vec![0.0; FFT_SIZE]);
        assert!(a.bands.iter().all(|v| *v < 0.01));
        a.decay();
        assert!(a.is_silent());
    }

    #[test]
    fn ruido_blanco_reparte_energia() {
        let mut seed = 1u32;
        let noise: Vec<f32> = (0..FFT_SIZE)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
            })
            .collect();
        let mut a = Analyzer::new(48_000);
        settle(&mut a, &noise);
        // Del medio hacia arriba (bandas anchas) todas tienen energía.
        assert!(
            a.bands[24..].iter().all(|v| *v > 0.3),
            "{:?}",
            &a.bands[24..]
        );
        assert!(a.level > 0.3);
    }

    #[test]
    fn bandas_validas_y_crecientes() {
        let r = band_ranges(44_100);
        assert_eq!(r.len(), BANDS);
        assert!(r.iter().all(|(lo, hi)| lo < hi && *hi <= FFT_SIZE / 2));
        assert!(r.windows(2).all(|w| w[0].0 <= w[1].0));
    }
}
