//! Symphonia → rubato → mapping of songs to the output (FR-002-01/03).

use std::time::Duration;

use audioadapter_buffers::direct::InterleavedSlice;
use reqwest::blocking::Client;
use rubato::{Fft, FixedSync, Resampler};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::{Hint, Probe};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

use super::resolver::ResolvedStream;
use super::source::{FileSource, HttpRangeSource, SourceError, SourceProbe};
use crate::domain::TrackId;

/// What the engine plays: a YouTube stream already resolved or a file on disk.
#[derive(Clone, Debug, PartialEq)]
pub enum MediaStream {
    Remote(ResolvedStream),
    Local {
        track_id: TrackId,
        path: std::path::PathBuf,
        duration_ms: Option<u32>,
    },
}

impl MediaStream {
    pub fn track_id(&self) -> &TrackId {
        match self {
            Self::Remote(s) => &s.track_id,
            Self::Local { track_id, .. } => track_id,
        }
    }

    pub fn duration_ms(&self) -> Option<u32> {
        match self {
            Self::Remote(s) => s.duration_ms,
            Self::Local { duration_ms, .. } => *duration_ms,
        }
    }
}

/// Why decoding failed.
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
            tracing::warn!(error = %other, "decoding error");
            DecodeFailure::Format
        }
    }
}

/// Where to measure how much audio is available ahead of the read position.
enum Buffered {
    Http(SourceProbe),
    Local,
}

/// Duration stated by the container (local files without a duration tag).
pub(crate) fn track_duration_ms(track: &symphonia::core::formats::Track) -> Option<u32> {
    let time = match (track.time_base, track.duration) {
        (Some(tb), Some(d)) => tb.calc_duration(d)?,
        _ => {
            let rate = track.codec_params.as_ref()?.audio()?.sample_rate?;
            let frames = track.num_frames?;
            return u32::try_from(frames * 1000 / rate.max(1) as u64).ok();
        }
    };
    u32::try_from(time.as_millis()).ok()
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
    probe: Buffered,
    /// Bytes per millisecond, to estimate how much is in the buffer.
    bytes_per_ms: f64,
    pub duration_ms: Option<u32>,
}

/// Format detection for YouTube's streams: MP4 only, without the trailing-metadata readers
/// the default probe has for local files. ID3v1 is read from the last 128 bytes, and during
/// a fast start those are beyond the head the first URL serves: opening the stream would
/// wait for yt-dlp's full URL (and its `available_at`), delaying the start by seconds.
fn remote_probe() -> &'static Probe {
    static PROBE: std::sync::OnceLock<Probe> = std::sync::OnceLock::new();
    PROBE.get_or_init(|| {
        let mut probe = Probe::new();
        probe.register_format::<symphonia::default::formats::IsoMp4Reader<'_>>();
        probe
    })
}

impl Decoding {
    /// Blocking (network or disk): open from a helper thread, not from the engine.
    pub fn open(
        client: Client,
        stream: &MediaStream,
        start_ms: u32,
        out_rate: u32,
        out_ch: u16,
    ) -> Result<Self, DecodeFailure> {
        let mut hint = Hint::new();
        let (mss, probe, byte_len, duration_ms) = match stream {
            MediaStream::Remote(stream) => {
                let source = HttpRangeSource::open(
                    client,
                    &stream.url,
                    stream.user_agent.as_deref(),
                    stream.content_length,
                    stream.available_at,
                    stream.upgrade.clone(),
                )?;
                let probe = Buffered::Http(source.probe());
                hint.with_extension("m4a");
                let mss = MediaSourceStream::new(Box::new(source), Default::default());
                (mss, probe, stream.content_length, stream.duration_ms)
            }
            MediaStream::Local {
                path, duration_ms, ..
            } => {
                let source = FileSource::open(path).map_err(|e| {
                    tracing::warn!(error = %e, "could not open the local file");
                    DecodeFailure::Format
                })?;
                let len = source.len();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    hint.with_extension(&ext.to_ascii_lowercase());
                }
                let mss = MediaSourceStream::new(Box::new(source), Default::default());
                (mss, Buffered::Local, Some(len), *duration_ms)
            }
        };
        let formats = match stream {
            MediaStream::Remote(_) => remote_probe(),
            MediaStream::Local { .. } => symphonia::default::get_probe(),
        };
        let format = formats
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

        // Without a known duration (untagged local file), the container's.
        let duration_ms = duration_ms.or_else(|| track_duration_ms(track));
        let bytes_per_ms = match (byte_len, duration_ms) {
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

    /// Blocking if the region has not been downloaded.
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

    /// Estimate of how much is downloaded ahead, in ms. A file on disk is all there.
    pub fn buffered_ms(&self) -> u32 {
        match &self.probe {
            Buffered::Http(p) => {
                (p.buffered_ahead() as f64 / self.bytes_per_ms).min(u32::MAX as f64) as u32
            }
            Buffered::Local => self.duration_ms.unwrap_or(u32::MAX),
        }
    }

    /// Decodes the next packet and appends to `out` the samples already
    /// resampled and adapted to the output channels. Returns `false` at the
    /// end of the stream.
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
                tracing::debug!(error = %e, "corrupt packet, skipping");
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
                tracing::error!(error = %e, "could not create the resampler");
                DecodeFailure::Format
            })?;
            self.resampler = Some(r);
        }
        let rs = self.resampler.as_mut().expect("resampler");
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
                        tracing::error!(error = %e, "resampling failed");
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

    /// Leftovers not resampled at the end (< 25 ms, inaudible).
    fn flush_pending(&mut self, out: &mut Vec<f32>) {
        if self.src_ch > 0 && !self.pending.is_empty() {
            map_channels(&self.pending, self.src_ch, self.out_ch, out);
            self.pending.clear();
        }
    }
}

/// Mono → all channels; stereo → first two, the rest silent.
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

/// HTTP client shared by all sources (keep-alive).
pub fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .build()
        .expect("HTTP client")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::local::fixtures::{mp3_with_id3, temp_dir, wav};

    fn local(path: std::path::PathBuf, duration_ms: Option<u32>) -> MediaStream {
        MediaStream::Local {
            track_id: TrackId::local(&path),
            path,
            duration_ms,
        }
    }

    fn decode_all(dec: &mut Decoding) -> usize {
        let mut out = Vec::new();
        while dec.next_chunk(&mut out).unwrap() {}
        out.len()
    }

    #[test]
    fn decodes_and_seeks_a_local_file() {
        let dir = temp_dir("decode-local");
        let path = dir.join("tone.WAV");
        std::fs::write(&path, wav(44_100, 2, 88_200)).unwrap();

        // No duration tag: the container's is used. 44.1 kHz → 48 kHz resampled.
        let mut dec =
            Decoding::open(http_client(), &local(path.clone(), None), 0, 48_000, 2).expect("opens");
        assert_eq!(dec.duration_ms, Some(2000));
        assert_eq!(dec.buffered_ms(), 2000, "a file on disk is all buffered");
        let whole = decode_all(&mut dec) / 2;
        assert!(whole.abs_diff(96_000) < 2_000, "frames: {whole}");

        dec.seek(1500).unwrap();
        let rest = decode_all(&mut dec) / 2;
        assert!(
            rest.abs_diff(24_000) < 2_000,
            "frames after the seek: {rest}"
        );

        let started = Decoding::open(http_client(), &local(path, None), 1000, 48_000, 1).unwrap();
        let mut started = started;
        let half = decode_all(&mut started);
        assert!(half.abs_diff(48_000) < 2_000, "mono, from 1 s: {half}");
    }

    #[test]
    fn decodes_mp3_behind_an_id3_tag() {
        let dir = temp_dir("decode-mp3");
        let path = dir.join("song.mp3");
        std::fs::write(&path, mp3_with_id3("T", "A", "B", 1, true)).unwrap();
        let mut dec =
            Decoding::open(http_client(), &local(path, Some(1000)), 0, 44_100, 2).expect("opens");
        assert!(decode_all(&mut dec) > 0);
    }

    #[test]
    fn missing_or_bogus_files_fail_cleanly() {
        let dir = temp_dir("decode-bad");
        let bogus = dir.join("bogus.flac");
        std::fs::write(&bogus, b"definitely not flac").unwrap();
        for path in [dir.join("missing.mp3"), bogus] {
            let r = Decoding::open(http_client(), &local(path, None), 0, 48_000, 2);
            assert!(matches!(r, Err(DecodeFailure::Format)));
        }
    }

    #[test]
    fn channel_mapping() {
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
