//! Symphonia → rubato → mapeo de canciones a la salida (FR-002-01/03).

use std::time::Duration;

use audioadapter_buffers::direct::InterleavedSlice;
use reqwest::blocking::Client;
use rubato::{Fft, FixedSync, Resampler};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

use super::resolver::ResolvedStream;
use super::source::{HttpRangeSource, SourceError, SourceProbe};

/// Por qué falló la decodificación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeFailure {
    Expired,
    Network,
    Format,
}

impl From<SourceError> for DecodeFailure {
    fn from(e: SourceError) -> Self {
        match e {
            SourceError::Expired => Self::Expired,
            SourceError::Network => Self::Network,
        }
    }
}

fn classify(e: SymError) -> DecodeFailure {
    match e {
        SymError::IoError(io) => SourceError::from_io(&io).into(),
        other => {
            tracing::warn!(error = %other, "error de decodificación");
            DecodeFailure::Format
        }
    }
}

pub struct Decoding {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track: u32,
    out_rate: u32,
    out_ch: usize,
    resampler: Option<Fft<f32>>,
    src_ch: usize,
    pending: Vec<f32>,
    decoded: Vec<f32>,
    resampled: Vec<f32>,
    probe: SourceProbe,
    /// Bytes por milisegundo, para estimar cuánto hay en el búfer.
    bytes_per_ms: f64,
    pub duration_ms: Option<u32>,
}

impl Decoding {
    /// Bloqueante (red): abrir desde un hilo auxiliar, no desde el motor.
    pub fn open(
        client: Client,
        stream: &ResolvedStream,
        start_ms: u32,
        out_rate: u32,
        out_ch: u16,
    ) -> Result<Self, DecodeFailure> {
        let source = HttpRangeSource::open(
            client,
            &stream.url,
            stream.user_agent.as_deref(),
            stream.content_length,
            stream.upgrade.clone(),
        )?;
        let probe = source.probe();
        let mss = MediaSourceStream::new(Box::new(source), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("m4a");
        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(classify)?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(DecodeFailure::Format)?;
        let track_id = track.id;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or(DecodeFailure::Format)?
            .clone();
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())
            .map_err(classify)?;

        let duration_ms = stream.duration_ms;
        let bytes_per_ms = match (stream.content_length, duration_ms) {
            (Some(len), Some(d)) if d > 0 => len as f64 / d as f64,
            _ => 16.0, // ~128 kbps
        };
        let mut dec = Self {
            format,
            decoder,
            track: track_id,
            out_rate,
            out_ch: out_ch as usize,
            resampler: None,
            src_ch: 0,
            pending: Vec::new(),
            decoded: Vec::new(),
            resampled: Vec::new(),
            probe,
            bytes_per_ms,
            duration_ms,
        };
        if start_ms > 0 {
            dec.seek(start_ms)?;
        }
        Ok(dec)
    }

    /// Bloqueante si la zona no está descargada.
    pub fn seek(&mut self, ms: u32) -> Result<(), DecodeFailure> {
        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::from_millis_u64(ms as u64),
                    track_id: Some(self.track),
                },
            )
            .map_err(classify)?;
        self.decoder.reset();
        if let Some(r) = &mut self.resampler {
            r.reset();
        }
        self.pending.clear();
        Ok(())
    }

    /// Estimación de lo descargado por delante, en ms.
    pub fn buffered_ms(&self) -> u32 {
        (self.probe.buffered_ahead() as f64 / self.bytes_per_ms).min(u32::MAX as f64) as u32
    }

    /// Decodifica el siguiente paquete y añade a `out` las muestras ya
    /// remuestreadas y adaptadas a los canales de salida. Devuelve `false` al
    /// final del stream.
    pub fn next_chunk(&mut self, out: &mut Vec<f32>) -> Result<bool, DecodeFailure> {
        let packet = loop {
            match self.format.next_packet() {
                Ok(Some(p)) if p.track_id == self.track => break p,
                Ok(Some(_)) => continue,
                Ok(None) => {
                    self.flush_pending(out);
                    return Ok(false);
                }
                Err(e) => return Err(classify(e)),
            }
        };
        let buf = match self.decoder.decode(&packet) {
            Ok(b) => b,
            Err(SymError::DecodeError(e)) => {
                tracing::debug!(error = %e, "paquete corrupto, se salta");
                return Ok(true);
            }
            Err(e) => return Err(classify(e)),
        };
        let in_rate = buf.spec().rate();
        self.src_ch = buf.spec().channels().count();
        self.decoded.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut self.decoded);

        if in_rate == self.out_rate {
            map_channels(&self.decoded, self.src_ch, self.out_ch, out);
            return Ok(true);
        }
        if self.resampler.is_none() {
            let r = Fft::<f32>::new(
                in_rate as usize,
                self.out_rate as usize,
                1024,
                self.src_ch,
                FixedSync::Input,
            )
            .map_err(|e| {
                tracing::error!(error = %e, "no se pudo crear el remuestreador");
                DecodeFailure::Format
            })?;
            self.resampler = Some(r);
        }
        let rs = self.resampler.as_mut().expect("remuestreador");
        self.pending.extend_from_slice(&self.decoded);
        loop {
            let need = rs.input_frames_next();
            if self.pending.len() / self.src_ch < need {
                break;
            }
            let max_out = rs.output_frames_max();
            self.resampled.resize(max_out * self.src_ch, 0.0);
            let input =
                InterleavedSlice::new(&self.pending[..need * self.src_ch], self.src_ch, need)
                    .map_err(|_| DecodeFailure::Format)?;
            let mut output = InterleavedSlice::new_mut(&mut self.resampled, self.src_ch, max_out)
                .map_err(|_| DecodeFailure::Format)?;
            let (used, produced) =
                rs.process_into_buffer(&input, &mut output, None)
                    .map_err(|e| {
                        tracing::error!(error = %e, "fallo remuestreando");
                        DecodeFailure::Format
                    })?;
            self.pending.drain(..used * self.src_ch);
            map_channels(
                &self.resampled[..produced * self.src_ch],
                self.src_ch,
                self.out_ch,
                out,
            );
        }
        Ok(true)
    }

    /// Restos sin remuestrear al final (< 25 ms, inaudible).
    fn flush_pending(&mut self, out: &mut Vec<f32>) {
        if self.src_ch > 0 && !self.pending.is_empty() {
            map_channels(&self.pending, self.src_ch, self.out_ch, out);
            self.pending.clear();
        }
    }
}

/// Mono → todos los canales; estéreo → primeros dos, el resto en silencio.
fn map_channels(src: &[f32], src_ch: usize, out_ch: usize, out: &mut Vec<f32>) {
    if src_ch == out_ch {
        out.extend_from_slice(src);
        return;
    }
    for frame in src.chunks_exact(src_ch) {
        for c in 0..out_ch {
            out.push(if c < src_ch {
                frame[c]
            } else if src_ch == 1 {
                frame[0]
            } else {
                0.0
            });
        }
    }
}

/// Cliente HTTP compartido por todas las fuentes (keep-alive).
pub fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .build()
        .expect("cliente HTTP")
}

#[cfg(test)]
mod tests {
    use super::map_channels;

    #[test]
    fn mapeo_de_canales() {
        let mut out = Vec::new();
        map_channels(&[0.1, 0.2], 1, 2, &mut out);
        assert_eq!(out, [0.1, 0.1, 0.2, 0.2]);
        out.clear();
        map_channels(&[0.1, 0.2], 2, 4, &mut out);
        assert_eq!(out, [0.1, 0.2, 0.0, 0.0]);
        out.clear();
        map_channels(&[0.1, 0.2, 0.3, 0.4], 2, 2, &mut out);
        assert_eq!(out, [0.1, 0.2, 0.3, 0.4]);
    }
}
