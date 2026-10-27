//! Audio engine thread (specs/002/plan.md, "Contract: engine").
//!
//! Receives `EngineCmd`, decodes into the output ring and emits
//! `EngineEvent`. Opening a track (network) is done in helper threads so as not
//! to block commands. A registry of *segments* (start frame → track)
//! tells what is really playing even when the ring already contains the next
//! song (gapless transition).

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, select, unbounded};
use reqwest::blocking::Client;

use super::decoder::{DecodeFailure, Decoding, MediaStream, http_client};
use super::output::{Output, PcmTap, perceptual_amplitude};
use crate::domain::TrackId;
use crate::error::AppError;

const POSITION_INTERVAL: Duration = Duration::from_millis(250);
/// Minimum free samples in the ring to decode another packet.
const MIN_FREE_SAMPLES: usize = 4096;

#[derive(Debug)]
pub enum EngineCmd {
    Load {
        stream: MediaStream,
        start_ms: u32,
        autoplay: bool,
    },
    Preload {
        stream: MediaStream,
    },
    Play,
    Pause,
    Stop,
    Seek(u32),
    SetVolume {
        volume: f32,
        muted: bool,
    },

    SetGainDb(f32),
    SetDevice(Option<String>),

    SetTap(bool),
    PlaySound(super::sounds::UiSound),
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Idle,
    Loading,
    Buffering,
    Playing,
    Paused,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EngineEvent {
    State(EngineState),
    /// A track starts *playing*. `gapless` = it came in on its own from the preload.
    TrackStarted {
        track_id: TrackId,
        duration_ms: Option<u32>,
        gapless: bool,
    },
    Position {
        track_id: TrackId,
        position_ms: u32,
        buffered_ms: u32,
    },
    /// It finished playing and there was no preload to continue with.
    Ended {
        track_id: TrackId,
    },
    /// The download failed midway; `expired` = expired URL (re-resolve).
    StreamFailed {
        track_id: TrackId,
        position_ms: u32,
        expired: bool,
    },
    PreloadFailed {
        track_id: TrackId,
    },
    Error(AppError),
}

/// Track being decoded.
struct Active {
    track_id: TrackId,
    dec: Decoding,
    eof: bool,
}

/// From frame `start_frame` (output counter) on, `track_id` plays
/// starting at `start_ms`.
#[derive(Debug, Clone)]
struct Segment {
    start_frame: u64,
    track_id: TrackId,
    start_ms: u32,
    duration_ms: Option<u32>,
    gapless: bool,
}

enum Opened {
    Load {
        generation: u64,
        track_id: TrackId,
        start_ms: u32,
        result: Result<Decoding, DecodeFailure>,
    },
    Preload {
        track_id: TrackId,
        result: Result<Decoding, DecodeFailure>,
    },
}

pub struct EngineHandle {
    tx: Sender<EngineCmd>,
    tap: Arc<Mutex<Option<PcmTap>>>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
}

impl EngineHandle {
    pub fn spawn(events: Sender<EngineEvent>) -> Self {
        let (tx, rx) = unbounded();
        let tap = Arc::new(Mutex::new(None));
        let tap_slot = Arc::clone(&tap);
        let thread = thread::Builder::new()
            .name("audio-engine".into())
            .spawn(move || Engine::new(rx, events, tap_slot).run())
            .expect("audio engine thread");
        Self {
            tx,
            tap,
            thread: Mutex::new(Some(thread)),
        }
    }

    pub fn send(&self, cmd: EngineCmd) {
        if self.tx.send(cmd).is_err() {
            tracing::error!("the audio engine is not running");
        }
    }

    /// Takes the PCM tap (005). It changes if the device changes.
    pub fn take_tap(&self) -> Option<PcmTap> {
        self.tap.lock().unwrap().take()
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(EngineCmd::Shutdown);
        if let Some(t) = self.thread.lock().unwrap().take() {
            let _ = t.join();
        }
    }
}

struct Engine {
    rx: Receiver<EngineCmd>,
    events: Sender<EngineEvent>,
    opened_tx: Sender<Opened>,
    opened_rx: Receiver<Opened>,
    tap_slot: Arc<Mutex<Option<PcmTap>>>,
    client: Client,

    output: Option<Output>,
    device_id: Option<String>,
    volume: f32,
    muted: bool,
    gain_db: f32,
    tap_enabled: bool,

    active: Option<Active>,
    next: Option<Active>,
    segments: VecDeque<Segment>,
    carry: Vec<f32>,
    scratch: Vec<f32>,

    state: EngineState,
    want_playing: bool,
    generation: u64,
    reported_segment: Option<(u64, TrackId)>,
    ended_reported: bool,
    last_position: Instant,
}

impl Engine {
    fn new(
        rx: Receiver<EngineCmd>,
        events: Sender<EngineEvent>,
        tap_slot: Arc<Mutex<Option<PcmTap>>>,
    ) -> Self {
        let (opened_tx, opened_rx) = unbounded();
        Self {
            rx,
            events,
            opened_tx,
            opened_rx,
            tap_slot,
            client: http_client(),
            output: None,
            device_id: None,
            volume: 0.8,
            muted: false,
            gain_db: 0.0,
            tap_enabled: false,
            active: None,
            next: None,
            segments: VecDeque::new(),
            carry: Vec::new(),
            scratch: Vec::new(),
            state: EngineState::Idle,
            want_playing: false,
            generation: 0,
            reported_segment: None,
            ended_reported: false,
            last_position: Instant::now(),
        }
    }

    fn emit(&self, e: EngineEvent) {
        let _ = self.events.send(e);
    }

    fn set_state(&mut self, s: EngineState) {
        if self.state != s {
            self.state = s;
            self.emit(EngineEvent::State(s));
        }
    }

    fn run(mut self) {
        loop {
            // With pending work there is no waiting; otherwise sleep until a command.
            let busy = self.can_decode();
            let timeout = if busy {
                Duration::ZERO
            } else {
                Duration::from_millis(20)
            };
            select! {
                recv(self.rx) -> cmd => match cmd {
                    Ok(EngineCmd::Shutdown) | Err(_) => break,
                    Ok(cmd) => self.handle(cmd),
                },
                recv(self.opened_rx) -> msg => if let Ok(m) = msg { self.on_opened(m) },
                default(timeout) => {}
            }
            self.check_device();
            if self.can_decode() {
                self.decode_some();
            }
            self.track_playback();
        }
        tracing::info!("audio engine stopped");
    }

    // ------------------------------------------------------------ commands

    fn handle(&mut self, cmd: EngineCmd) {
        match cmd {
            EngineCmd::Load {
                stream,
                start_ms,
                autoplay,
            } => self.load(stream, start_ms, autoplay),
            EngineCmd::Preload { stream } => self.preload(stream),
            EngineCmd::Play => {
                self.want_playing = true;
                if let Some(o) = &self.output {
                    o.shared.paused.store(false, Ordering::Relaxed);
                }
                if self.active.is_some() || !self.segments.is_empty() {
                    self.set_state(EngineState::Playing);
                }
            }
            EngineCmd::Pause => {
                self.want_playing = false;
                if let Some(o) = &self.output {
                    o.shared.paused.store(true, Ordering::Relaxed);
                }
                if self.state != EngineState::Idle {
                    self.set_state(EngineState::Paused);
                }
            }
            EngineCmd::Stop => {
                self.generation += 1;
                self.want_playing = false;
                self.active = None;
                self.next = None;
                self.clear_output();
                self.set_state(EngineState::Idle);
            }
            EngineCmd::Seek(ms) => self.seek(ms),
            EngineCmd::SetVolume { volume, muted } => {
                self.volume = volume;
                self.muted = muted;
                self.apply_amplitude();
            }
            EngineCmd::SetGainDb(db) => {
                self.gain_db = db;
                self.apply_amplitude();
            }
            EngineCmd::SetDevice(id) => {
                self.device_id = id;
                self.reopen_output();
            }
            EngineCmd::PlaySound(s) => {
                if self.ensure_output()
                    && let Some(o) = self.output.as_mut()
                {
                    let samples = super::sounds::synth(s, o.rate, o.channels);
                    o.play_fx(&samples);
                }
            }
            EngineCmd::SetTap(on) => {
                self.tap_enabled = on;
                if let Some(o) = &self.output {
                    o.shared.tap_enabled.store(on, Ordering::Relaxed);
                }
            }
            EngineCmd::Shutdown => unreachable!(),
        }
    }

    fn apply_amplitude(&self) {
        if let Some(o) = &self.output {
            o.shared
                .set_amplitude(perceptual_amplitude(self.volume, self.muted, self.gain_db));
        }
    }

    fn ensure_output(&mut self) -> bool {
        if self.output.is_some() {
            return true;
        }
        match Output::open(self.device_id.as_deref(), &self.tap_slot) {
            Ok(o) => {
                o.shared.paused.store(!self.want_playing, Ordering::Relaxed);
                o.shared
                    .tap_enabled
                    .store(self.tap_enabled, Ordering::Relaxed);
                self.output = Some(o);
                self.apply_amplitude();
                true
            }
            Err(e) => {
                self.emit(EngineEvent::Error(e));
                false
            }
        }
    }

    fn load(&mut self, stream: MediaStream, start_ms: u32, autoplay: bool) {
        self.generation += 1;
        self.want_playing = autoplay;
        if !self.ensure_output() {
            self.set_state(EngineState::Idle);
            return;
        }
        // Already preloaded? (Next button right after the preload)
        if start_ms == 0
            && self
                .next
                .as_ref()
                .is_some_and(|n| &n.track_id == stream.track_id())
        {
            let next = self.next.take().expect("preload");
            self.start_active(next, 0, false);
            return;
        }
        self.set_state(EngineState::Loading);
        let (out_rate, out_ch) = {
            let o = self.output.as_ref().expect("output");
            (o.rate, o.channels)
        };
        let tx = self.opened_tx.clone();
        let client = self.client.clone();
        let generation = self.generation;
        thread::spawn(move || {
            let result = Decoding::open(client, &stream, start_ms, out_rate, out_ch);
            let _ = tx.send(Opened::Load {
                generation,
                track_id: stream.track_id().clone(),
                start_ms,
                result,
            });
        });
    }

    fn preload(&mut self, stream: MediaStream) {
        if !self.ensure_output() {
            return;
        }
        let (out_rate, out_ch) = {
            let o = self.output.as_ref().expect("output");
            (o.rate, o.channels)
        };
        let tx = self.opened_tx.clone();
        let client = self.client.clone();
        thread::spawn(move || {
            let result = Decoding::open(client, &stream, 0, out_rate, out_ch);
            let _ = tx.send(Opened::Preload {
                track_id: stream.track_id().clone(),
                result,
            });
        });
    }

    fn on_opened(&mut self, msg: Opened) {
        match msg {
            Opened::Load {
                generation,
                track_id,
                start_ms,
                result,
            } => {
                if generation != self.generation {
                    return; // Arrived late: the user already asked for something else.
                }
                match result {
                    Ok(dec) => {
                        // `want_playing` was set by `load` (or a later pause): don't override it.
                        self.start_active(
                            Active {
                                track_id,
                                dec,
                                eof: false,
                            },
                            start_ms,
                            false,
                        );
                    }
                    Err(f) => {
                        self.set_state(EngineState::Idle);
                        self.emit(EngineEvent::StreamFailed {
                            track_id,
                            position_ms: start_ms,
                            expired: f == DecodeFailure::Expired,
                        });
                    }
                }
            }
            Opened::Preload { track_id, result } => match result {
                Ok(dec) => {
                    tracing::debug!(track = %track_id.0, "track preloaded");
                    self.next = Some(Active {
                        track_id,
                        dec,
                        eof: false,
                    });
                }
                Err(_) => self.emit(EngineEvent::PreloadFailed { track_id }),
            },
        }
    }

    /// Replaces whatever is playing with `active` (empties the ring).
    fn start_active(&mut self, active: Active, start_ms: u32, gapless: bool) {
        self.clear_output();
        let start_frame = self.output.as_ref().map_or(0, |o| o.pushed_frames());
        self.segments.push_back(Segment {
            start_frame,
            track_id: active.track_id.clone(),
            start_ms,
            duration_ms: active.dec.duration_ms,
            gapless,
        });
        self.active = Some(active);
        self.ended_reported = false;
        if let Some(o) = &self.output {
            o.shared.paused.store(!self.want_playing, Ordering::Relaxed);
        }
        self.set_state(if self.want_playing {
            EngineState::Buffering
        } else {
            EngineState::Paused
        });
    }

    fn clear_output(&mut self) {
        self.carry.clear();
        self.segments.clear();
        self.reported_segment = None;
        if let Some(o) = &mut self.output {
            o.flush();
        }
    }

    fn seek(&mut self, ms: u32) {
        let Some(active) = &mut self.active else {
            // The track has already finished decoding: reload from the start of the segment.
            return;
        };
        let track_id = active.track_id.clone();
        let duration_ms = active.dec.duration_ms;
        match active.dec.seek(ms) {
            Ok(()) => {
                active.eof = false;
                self.clear_output();
                let start_frame = self.output.as_ref().map_or(0, |o| o.pushed_frames());
                self.segments.push_back(Segment {
                    start_frame,
                    track_id,
                    start_ms: ms,
                    duration_ms,
                    gapless: false,
                });
                // The new segment is the same song: it is not announced as a track change.
                self.reported_segment =
                    Some((start_frame, self.segments.back().unwrap().track_id.clone()));
                if self.want_playing {
                    self.set_state(EngineState::Buffering);
                }
            }
            Err(f) => {
                self.active = None;
                self.ended_reported = true;
                self.emit(EngineEvent::StreamFailed {
                    track_id,
                    position_ms: ms,
                    expired: f == DecodeFailure::Expired,
                });
            }
        }
    }

    fn reopen_output(&mut self) {
        let position = self.current_position();
        self.output = None;
        if !self.ensure_output() {
            return;
        }
        // The sample rate may have changed: reopen the track at the same position.
        if let (Some(active), Some((track_id, ms))) = (&self.active, position)
            && active.track_id == track_id
        {
            self.seek(ms);
        }
        self.next = None;
    }

    fn check_device(&mut self) {
        if self
            .output
            .as_ref()
            .is_some_and(|o| o.shared.device_lost.load(Ordering::Acquire))
        {
            tracing::warn!("audio device lost; switching to the default one");
            self.device_id = None;
            self.reopen_output();
        }
    }

    // ------------------------------------------------------------ decoding

    fn can_decode(&self) -> bool {
        self.output
            .as_ref()
            .is_some_and(|o| o.free_samples() >= MIN_FREE_SAMPLES)
            && (self.active.as_ref().is_some_and(|a| !a.eof) || !self.carry.is_empty())
    }

    fn decode_some(&mut self) {
        let Some(output) = self.output.as_mut() else {
            return;
        };
        if !self.carry.is_empty() {
            let n = output.push(&self.carry);
            self.carry.drain(..n);
            if !self.carry.is_empty() {
                return;
            }
        }
        let Some(active) = self.active.as_mut() else {
            return;
        };
        if active.eof {
            return;
        }
        self.scratch.clear();
        match active.dec.next_chunk(&mut self.scratch) {
            Ok(true) => {
                let n = output.push(&self.scratch);
                self.carry.extend_from_slice(&self.scratch[n..]);
            }
            Ok(false) => {
                active.eof = true;
                if let Some(next) = self.next.take() {
                    // Gapless: the next one starts right behind it in the ring.
                    let start_frame = output.pushed_frames();
                    self.segments.push_back(Segment {
                        start_frame,
                        track_id: next.track_id.clone(),
                        start_ms: 0,
                        duration_ms: next.dec.duration_ms,
                        gapless: true,
                    });
                    self.active = Some(next);
                }
            }
            Err(f) => {
                let track_id = active.track_id.clone();
                let position_ms = self.current_position().map_or(0, |(_, ms)| ms);
                self.active = None;
                // The service reloads at this position: an `Ended` must not arrive as well.
                self.ended_reported = true;
                self.emit(EngineEvent::StreamFailed {
                    track_id,
                    position_ms,
                    expired: f == DecodeFailure::Expired,
                });
            }
        }
    }

    // ------------------------------------------------------------ tracking

    /// Segment playing now and the position within it.
    fn current_position(&self) -> Option<(TrackId, u32)> {
        let o = self.output.as_ref()?;
        let played = o.played_frames();
        let seg = self
            .segments
            .iter()
            .rev()
            .find(|s| s.start_frame <= played)?;
        let ms = seg.start_ms as u64 + (played - seg.start_frame) * 1000 / o.rate as u64;
        Some((seg.track_id.clone(), ms.min(u32::MAX as u64) as u32))
    }

    fn track_playback(&mut self) {
        let Some(o) = self.output.as_ref() else {
            return;
        };
        let (played, queued, rate, channels) = (
            o.played_frames(),
            o.queued_samples(),
            o.rate.max(1) as u64,
            o.channels.max(1) as u64,
        );

        // Discard old segments (keeps the current one).
        while self.segments.len() > 1 && self.segments[1].start_frame <= played {
            self.segments.pop_front();
        }

        // Has a new segment started playing?
        if let Some(seg) = self
            .segments
            .front()
            .filter(|s| s.start_frame <= played)
            .cloned()
        {
            let key = (seg.start_frame, seg.track_id.clone());
            if self.reported_segment.as_ref() != Some(&key)
                && (played > seg.start_frame || seg.gapless)
            {
                self.reported_segment = Some(key);
                self.emit(EngineEvent::TrackStarted {
                    track_id: seg.track_id.clone(),
                    duration_ms: seg.duration_ms,
                    gapless: seg.gapless,
                });
            }
        }

        // State: Buffering ↔ Playing depending on whether there is audio in the ring.
        if self.want_playing && matches!(self.state, EngineState::Buffering | EngineState::Playing)
        {
            let starving = queued == 0 && self.active.as_ref().is_some_and(|a| !a.eof);
            self.set_state(if starving {
                EngineState::Buffering
            } else {
                EngineState::Playing
            });
        }

        // End: nothing is being decoded anymore and the ring is empty.
        let drained = queued == 0 && self.carry.is_empty();
        if drained
            && self.active.as_ref().is_none_or(|a| a.eof)
            && !self.ended_reported
            && let Some(seg) = self.segments.back().cloned()
            && played >= seg.start_frame
            && self.state != EngineState::Idle
        {
            self.ended_reported = true;
            self.active = None;
            self.segments.clear();
            self.reported_segment = None;
            self.set_state(EngineState::Idle);
            self.emit(EngineEvent::Ended {
                track_id: seg.track_id,
            });
            return;
        }

        if self.last_position.elapsed() >= POSITION_INTERVAL {
            self.last_position = Instant::now();
            if let Some((track_id, position_ms)) = self.current_position()
                && matches!(self.state, EngineState::Playing | EngineState::Buffering)
            {
                let ring_ms = (queued as u64 / channels) * 1000 / rate;
                let source_ms = self
                    .active
                    .as_ref()
                    .map_or(0, |a| a.dec.buffered_ms() as u64);
                let buffered_ms = (ring_ms + source_ms).min(u32::MAX as u64) as u32;
                self.emit(EngineEvent::Position {
                    track_id,
                    position_ms,
                    buffered_ms,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests with real audio play silently unless `XP_MUSIC_TEST_AUDIBLE=1`.
    fn test_muted() -> bool {
        std::env::var_os("XP_MUSIC_TEST_AUDIBLE").is_none()
    }
    use crate::audio::resolver::{
        FormatPreference, ResolvedStream, StreamResolver, ToolPaths, YtDlpResolver,
    };

    fn wait_for(
        rx: &Receiver<EngineEvent>,
        timeout: Duration,
        mut pred: impl FnMut(&EngineEvent) -> bool,
    ) -> Option<EngineEvent> {
        let deadline = Instant::now() + timeout;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(ev) => {
                    if pred(&ev) {
                        return Some(ev);
                    }
                    assert!(
                        !matches!(ev, EngineEvent::StreamFailed { .. } | EngineEvent::Error(_)),
                        "unexpected event: {ev:?}"
                    );
                }
                Err(_) => break,
            }
        }
        None
    }

    /// Local files through the real output (muted): start, seek, PCM for the
    /// visualization, gapless into a second file and the end.
    /// `cargo test --lib -- --ignored --nocapture engine_local`.
    #[test]
    #[ignore]
    fn engine_local_files_play_seek_and_chain() {
        use crate::local::fixtures::{temp_dir, wav};
        use ringbuf::traits::Observer;
        let dir = temp_dir("engine-local");
        let stream = |name: &str, secs: u32| {
            let path = dir.join(name);
            std::fs::write(&path, wav(44_100, 2, 44_100 * secs)).unwrap();
            MediaStream::Local {
                track_id: TrackId::local(&path),
                path,
                duration_ms: None,
            }
        };
        let (a, b) = (stream("a.wav", 4), stream("b.wav", 2));
        let (tx, rx) = unbounded();
        let engine = EngineHandle::spawn(tx);
        engine.send(EngineCmd::SetVolume {
            volume: 0.5,
            muted: test_muted(),
        });
        engine.send(EngineCmd::SetTap(true));
        let t0 = Instant::now();
        engine.send(EngineCmd::Load {
            stream: a.clone(),
            start_ms: 0,
            autoplay: true,
        });
        let started = wait_for(&rx, Duration::from_secs(5), |e| {
            matches!(e, EngineEvent::TrackStarted { .. })
        })
        .expect("did not start");
        println!("first sound: {} ms", t0.elapsed().as_millis());
        assert!(
            matches!(
                started,
                EngineEvent::TrackStarted {
                    duration_ms: Some(4000),
                    ..
                }
            ),
            "{started:?}"
        );

        engine.send(EngineCmd::Seek(2500));
        wait_for(
            &rx,
            Duration::from_secs(5),
            |e| matches!(e, EngineEvent::Position { position_ms, .. } if *position_ms >= 2600),
        )
        .expect("the seek does not play");
        let tap = engine.take_tap().expect("PCM tap for the visualization");
        assert!(tap.consumer.occupied_len() > 0, "no audio reaches the tap");

        engine.send(EngineCmd::Preload { stream: b.clone() });
        let mut ended_a = false;
        wait_for(&rx, Duration::from_secs(5), |e| {
            ended_a |= matches!(e, EngineEvent::Ended { track_id } if track_id == a.track_id());
            matches!(e, EngineEvent::TrackStarted { track_id, gapless: true, .. } if track_id == b.track_id())
        })
        .expect("the second file did not follow");
        assert!(!ended_a, "there was a gap between the files");
        wait_for(
            &rx,
            Duration::from_secs(5),
            |e| matches!(e, EngineEvent::Ended { track_id } if track_id == b.track_id()),
        )
        .expect("the end was not reported");
        engine.shutdown();
    }

    /// Visualization with real music (muted): audio reaches the tap even with
    /// the volume at 0 and beats are detected, also after releasing and
    /// retaking the tap. `cargo test --lib -- --ignored --nocapture viz_real`.
    #[test]
    #[ignore]
    fn viz_real_detects_beats() {
        use crate::viz::bands::{Analyzer, FFT_SIZE};
        use ringbuf::traits::{Consumer, Observer};
        let resolver = YtDlpResolver::new(ToolPaths::locate().expect("tools/"), None);
        let s = resolver
            .resolve(&TrackId("yQd4xQBlBa0".into()), FormatPreference::Aac128)
            .unwrap();
        let (tx, rx) = unbounded();
        let engine = EngineHandle::spawn(tx);
        engine.send(EngineCmd::SetVolume {
            volume: 0.0,
            muted: true,
        });
        engine.send(EngineCmd::SetTap(true));
        engine.send(EngineCmd::Load {
            stream: MediaStream::Remote(s),
            start_ms: 30_000,
            autoplay: true,
        });
        wait_for(&rx, Duration::from_secs(15), |e| {
            matches!(e, EngineEvent::TrackStarted { .. })
        })
        .expect("did not start playing");
        let mut tap = None;
        for _ in 0..50 {
            tap = engine.take_tap();
            if tap.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let mut tap = tap.expect("no PCM tap");
        let ch = tap.channels as usize;
        let mut a = Analyzer::new(tap.sample_rate);
        let mut ring = vec![0.0f32; FFT_SIZE];
        let mut buf = vec![0.0f32; 8192];
        let (mut beats, mut max_level) = (0, 0.0f32);
        for _ in 0..(8 * 30) {
            std::thread::sleep(Duration::from_millis(33));
            while tap.consumer.occupied_len() >= ch {
                let n = tap.consumer.pop_slice(&mut buf[..(8192 / ch) * ch]);
                for f in buf[..n].chunks_exact(ch) {
                    ring.rotate_left(1);
                    ring[FFT_SIZE - 1] = (f[0] + f[ch - 1]) * 0.5;
                }
            }
            a.analyze(&ring);
            max_level = max_level.max(a.level);
            if a.beat >= 1.0 {
                beats += 1;
            }
        }
        println!("8 s muted: max level {max_level:.2}, beats {beats}");
        engine.shutdown();
        assert!(
            max_level > 0.1,
            "no audio reaches the visualization with volume 0"
        );
        assert!(beats >= 6, "too few beats detected: {beats}");
    }

    /// Real audio through the speakers (30 % volume):
    /// `cargo test --lib -- --ignored --nocapture engine_real`.
    #[test]
    #[ignore]
    fn engine_real_starts_seeks_and_chains_without_gap() {
        let resolver = YtDlpResolver::new(ToolPaths::locate().expect("tools/"), None);
        let a = resolver
            .resolve(&TrackId("yQd4xQBlBa0".into()), FormatPreference::Aac128)
            .unwrap();
        let b = resolver
            .resolve(&TrackId("IoNOzOaWgTE".into()), FormatPreference::Aac128)
            .unwrap();
        let dur = a.duration_ms.expect("duration");

        let (tx, rx) = unbounded();
        let engine = EngineHandle::spawn(tx);
        engine.send(EngineCmd::SetVolume {
            volume: 0.67,
            muted: test_muted(),
        }); // ≈ 30 % amplitude

        // 1) First sound with the URL already resolved (same as cached): target < 1 s.
        let t0 = Instant::now();
        engine.send(EngineCmd::Load {
            stream: MediaStream::Remote(a.clone()),
            start_ms: 0,
            autoplay: true,
        });
        wait_for(&rx, Duration::from_secs(10), |e| {
            matches!(e, EngineEvent::TrackStarted { .. })
        })
        .expect("did not start playing");
        println!("first sound (cached URL): {} ms", t0.elapsed().as_millis());

        wait_for(
            &rx,
            Duration::from_secs(10),
            |e| matches!(e, EngineEvent::Position { position_ms, .. } if *position_ms >= 1500),
        )
        .expect("the position does not advance");

        // 2) Seek to 8 s before the end.
        let target = dur - 8_000;
        let t1 = Instant::now();
        engine.send(EngineCmd::Seek(target));
        wait_for(&rx, Duration::from_secs(10), |e| {
            matches!(e, EngineEvent::Position { position_ms, .. } if *position_ms >= target + 200)
        })
        .expect("the seek does not play");
        println!(
            "seek to {} %: {} ms",
            target * 100 / dur,
            t1.elapsed().as_millis()
        );

        // 3) Preload and gapless transition: the first track must not send `Ended`.
        engine.send(EngineCmd::Preload {
            stream: MediaStream::Remote(b.clone()),
        });
        let mut ended_a = false;
        let started_b = wait_for(&rx, Duration::from_secs(20), |e| {
            if matches!(e, EngineEvent::Ended { .. }) {
                ended_a = true;
            }
            matches!(e, EngineEvent::TrackStarted { track_id, gapless: true, .. } if *track_id == b.track_id)
        });
        assert!(
            started_b.is_some(),
            "the second track did not start on its own"
        );
        assert!(
            !ended_a,
            "`Ended` arrived before the transition: there was a gap"
        );
        println!("gapless transition to {}", b.track_id.0);

        wait_for(&rx, Duration::from_secs(5), |e| {
            matches!(e, EngineEvent::Position { track_id, position_ms, .. } if *track_id == b.track_id && *position_ms >= 1000)
        })
        .expect("the second track does not advance");
        engine.shutdown();
    }

    /// AC-002-03: the URL "expires" (403) and playback recovers by reloading at the same position.
    /// `cargo test --lib -- --ignored --nocapture engine_real_url`.
    #[test]
    #[ignore]
    fn engine_real_expired_url_recovers() {
        use crate::audio::source::test_server;

        let resolver = YtDlpResolver::new(ToolPaths::locate().expect("tools/"), None);
        let real = resolver
            .resolve(&TrackId("yQd4xQBlBa0".into()), FormatPreference::Aac128)
            .unwrap();
        // Full download by ranges (without Range, YouTube throttles the speed).
        let mut bytes = Vec::new();
        let mut src = crate::audio::source::HttpRangeSource::open(
            http_client(),
            &real.url,
            None,
            real.content_length,
            real.available_at,
            None,
        )
        .unwrap();
        std::io::Read::read_to_end(&mut src, &mut bytes).unwrap();
        let data = Arc::new(bytes);
        let dur = real.duration_ms.unwrap();

        // Request 0 = length, 1 = first block; then everything is 403.
        let failing = ResolvedStream {
            url: test_server::serve(Arc::clone(&data), Some(2)),
            content_length: None,
            ..real.clone()
        };
        let healthy = ResolvedStream {
            url: test_server::serve(Arc::clone(&data), None),
            content_length: None,
            ..real
        };

        let (tx, rx) = unbounded();
        let engine = EngineHandle::spawn(tx);
        engine.send(EngineCmd::SetVolume {
            volume: 0.67,
            muted: test_muted(),
        });
        engine.send(EngineCmd::Load {
            stream: MediaStream::Remote(failing),
            start_ms: 0,
            autoplay: true,
        });
        wait_for(&rx, Duration::from_secs(10), |e| {
            matches!(e, EngineEvent::TrackStarted { .. })
        })
        .expect("not playing");
        thread::sleep(Duration::from_secs(2));

        // Seeking to a region that is not downloaded forces a block request: 403.
        let target = dur / 2;
        engine.send(EngineCmd::Seek(target));
        let failed = loop {
            let e = rx
                .recv_timeout(Duration::from_secs(10))
                .expect("the failure did not arrive");
            if matches!(e, EngineEvent::StreamFailed { .. }) {
                break e;
            }
        };
        let EngineEvent::StreamFailed {
            position_ms,
            expired,
            ..
        } = failed
        else {
            unreachable!()
        };
        assert!(expired, "a 403 must be flagged as expired");
        let t0 = Instant::now();

        // What the service would do: re-resolve and load at the same position.
        engine.send(EngineCmd::Load {
            stream: MediaStream::Remote(healthy),
            start_ms: position_ms,
            autoplay: true,
        });
        wait_for(&rx, Duration::from_secs(10), |e| {
            matches!(e, EngineEvent::TrackStarted { .. })
        })
        .expect("did not recover");
        let gap = t0.elapsed();
        println!(
            "recovered in {} ms (position {} s)",
            gap.as_millis(),
            position_ms / 1000
        );
        assert!(gap < Duration::from_secs(2), "gap too long: {gap:?}");
        engine.shutdown();
    }
}
