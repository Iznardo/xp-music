//! Sonidos de la interfaz (FR-004-16), sintetizados: no son los de Windows XP,
//! solo comparten el carácter (clics suaves y campanillas cortas).

use serde::Deserialize;
use specta::Type;

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    /// Clic al navegar.
    Navigate,
    /// Aviso suave (mensajes informativos).
    Notify,
    /// Error.
    Error,
    /// Pregunta / advertencia.
    Exclamation,
}

/// Una nota con ataque corto y caída exponencial, con dos armónicos.
fn note(
    out: &mut [f32],
    rate: f32,
    start: f32,
    freq: f32,
    decay: f32,
    amp: f32,
    harmonics: (f32, f32),
) {
    let first = (start * rate) as usize;
    for (k, s) in out.iter_mut().enumerate().skip(first) {
        let t = (k - first) as f32 / rate;
        let attack = (t / 0.004).min(1.0);
        let env = attack * (-t / decay).exp();
        let w = std::f32::consts::TAU * freq * t;
        *s += amp * env * (w.sin() + harmonics.0 * (2.0 * w).sin() + harmonics.1 * (3.0 * w).sin());
    }
}

/// Muestras intercaladas para `channels` canales a `rate` Hz.
pub fn synth(sound: UiSound, rate: u32, channels: u16) -> Vec<f32> {
    let r = rate as f32;
    let secs = match sound {
        UiSound::Navigate => 0.05,
        UiSound::Notify => 0.45,
        UiSound::Error => 0.55,
        UiSound::Exclamation => 0.7,
    };
    let mut mono = vec![0.0f32; (secs * r) as usize];
    match sound {
        UiSound::Navigate => {
            // Clic: un golpe agudo muy corto con ruido filtrado.
            note(&mut mono, r, 0.0, 2200.0, 0.004, 0.10, (0.0, 0.0));
            let mut seed = 0x2545_f491_u32;
            let mut lp = 0.0f32;
            for (k, s) in mono.iter_mut().enumerate() {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                let noise = (seed as f32 / u32::MAX as f32) * 2.0 - 1.0;
                lp += 0.35 * (noise - lp);
                *s += 0.09 * lp * (-(k as f32 / r) / 0.006).exp();
            }
        }
        UiSound::Notify => {
            note(&mut mono, r, 0.0, 880.0, 0.09, 0.16, (0.3, 0.1));
            note(&mut mono, r, 0.13, 1318.5, 0.11, 0.16, (0.3, 0.1));
        }
        UiSound::Error => {
            note(&mut mono, r, 0.0, 659.3, 0.10, 0.18, (0.15, 0.25));
            note(&mut mono, r, 0.16, 440.0, 0.14, 0.18, (0.15, 0.25));
        }
        UiSound::Exclamation => {
            for (i, f) in [1046.5, 1318.5, 1568.0].into_iter().enumerate() {
                note(&mut mono, r, i as f32 * 0.035, f, 0.2, 0.09, (0.2, 0.05));
            }
        }
    }
    // Fundido final para evitar el chasquido del corte.
    let fade = (0.01 * r) as usize;
    let len = mono.len();
    for (k, s) in mono.iter_mut().enumerate().skip(len.saturating_sub(fade)) {
        *s *= (len - k) as f32 / fade as f32;
    }
    let ch = channels.max(1) as usize;
    let mut out = Vec::with_capacity(mono.len() * ch);
    for s in mono {
        let s = s.clamp(-1.0, 1.0);
        out.extend(std::iter::repeat_n(s, ch.min(2)));
        out.extend(std::iter::repeat_n(0.0, ch.saturating_sub(2)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sonidos_cortos_y_sin_saturar() {
        for s in [
            UiSound::Navigate,
            UiSound::Notify,
            UiSound::Error,
            UiSound::Exclamation,
        ] {
            let v = synth(s, 48_000, 2);
            assert!(
                !v.is_empty() && v.len() < 48_000 * 2,
                "{s:?} debe durar menos de 1 s"
            );
            let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.02 && peak <= 0.6, "{s:?}: pico {peak}");
            assert!(
                v.last().unwrap().abs() < 1e-3,
                "{s:?} debe terminar en silencio"
            );
        }
    }

    #[test]
    fn respeta_canales() {
        let v = synth(UiSound::Notify, 44_100, 8);
        assert_eq!(v.len() % 8, 0);
        // Solo los dos primeros canales llevan señal.
        assert!(v.chunks(8).all(|f| f[2..].iter().all(|x| *x == 0.0)));
    }
}
