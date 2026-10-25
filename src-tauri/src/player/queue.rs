//! Playback queue: pure logic, no audio or I/O (FR-002-11..13).
//!
//! `tracks` keeps the original order and `order` the playback order
//! (same as the original except when shuffled). All public indices
//! refer to the playback order, which is what the user sees.

use serde::{Deserialize, Serialize};

use super::types::{InsertPosition, QueueSnapshot, QueueSource, RepeatMode};
use crate::domain::{LikeStatus, Track, TrackId};

/// "Previous" with more than this much time played restarts the song.
pub const RESTART_THRESHOLD_MS: u32 = 3_000;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Queue {
    tracks: Vec<Track>,
    order: Vec<usize>,
    pos: Option<usize>,
    shuffle: bool,
    repeat: RepeatMode,
    autoplay: bool,
    source: Option<QueueSource>,
    seed: u64,
}

/// Result of "previous".
#[derive(Debug, PartialEq, Eq)]
pub enum Previous {
    Restart,
    Moved,
}

impl Queue {
    pub fn new() -> Self {
        Self {
            autoplay: true,
            seed: 0x9E37_79B9_7F4A_7C15,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[allow(dead_code)] // Mandatory companion of `len` (clippy).
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn current(&self) -> Option<&Track> {
        self.pos.map(|p| &self.tracks[self.order[p]])
    }

    pub fn current_index(&self) -> Option<usize> {
        self.pos
    }

    pub fn repeat(&self) -> RepeatMode {
        self.repeat
    }

    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    pub fn autoplay(&self) -> bool {
        self.autoplay
    }

    #[allow(dead_code)] // Will be used by context-aware autoplay (003).
    pub fn source(&self) -> Option<&QueueSource> {
        self.source.as_ref()
    }

    pub fn tracks_in_order(&self) -> impl Iterator<Item = &Track> {
        self.order.iter().map(|&i| &self.tracks[i])
    }

    /// Replaces the queue and positions at `start` (index in the original order).
    pub fn set(&mut self, tracks: Vec<Track>, start: usize, source: QueueSource) {
        let start = start.min(tracks.len().saturating_sub(1));
        self.order = (0..tracks.len()).collect();
        self.pos = (!tracks.is_empty()).then_some(start);
        self.tracks = tracks;
        self.source = Some(source);
        if self.shuffle {
            self.reshuffle_keeping_current();
        }
    }

    pub fn play_index(&mut self, index: usize) -> bool {
        if index < self.order.len() {
            self.pos = Some(index);
            true
        } else {
            false
        }
    }

    pub fn add(&mut self, tracks: Vec<Track>, at: InsertPosition) {
        let first_new = self.tracks.len();
        let new_idx: Vec<usize> = (first_new..first_new + tracks.len()).collect();
        self.tracks.extend(tracks);
        let insert_at = match (at, self.pos) {
            (InsertPosition::Next, Some(p)) => p + 1,
            _ => self.order.len(),
        };
        self.order.splice(insert_at..insert_at, new_idx);
        if self.pos.is_none() && !self.order.is_empty() {
            self.pos = Some(0);
        }
    }

    /// Removes the track at `index`. Returns `true` if it was the current one: in that
    /// case the current one becomes the next (or none if it was the last).
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.order.len() {
            return false;
        }
        let removed = self.order.remove(index);
        self.tracks.remove(removed);
        for i in &mut self.order {
            if *i > removed {
                *i -= 1;
            }
        }
        match self.pos {
            Some(p) if p == index => {
                self.pos = (index < self.order.len()).then_some(index);
                true
            }
            Some(p) if p > index => {
                self.pos = Some(p - 1);
                false
            }
            _ => false,
        }
    }

    pub fn move_item(&mut self, from: usize, to: usize) {
        if from >= self.order.len() || to >= self.order.len() || from == to {
            return;
        }
        let item = self.order.remove(from);
        self.order.insert(to, item);
        if let Some(p) = self.pos {
            self.pos = Some(if p == from {
                to
            } else if from < p && to >= p {
                p - 1
            } else if from > p && to <= p {
                p + 1
            } else {
                p
            });
        }
    }

    /// Sets the "Like" of a song in all its occurrences; `true` if it was present.
    pub fn set_like(&mut self, id: &TrackId, status: LikeStatus) -> bool {
        let mut found = false;
        for t in self.tracks.iter_mut().filter(|t| &t.id == id) {
            t.like_status = Some(status);
            found = true;
        }
        found
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.order.clear();
        self.pos = None;
        self.source = None;
    }

    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    pub fn set_autoplay(&mut self, enabled: bool) {
        self.autoplay = enabled;
    }

    /// Turns shuffle on or off. When turned on, the current track becomes the
    /// first and the rest is shuffled; when turned off, the original order is
    /// restored keeping the current track.
    pub fn set_shuffle(&mut self, enabled: bool) {
        if enabled == self.shuffle {
            return;
        }
        self.shuffle = enabled;
        if enabled {
            self.reshuffle_keeping_current();
        } else {
            let current = self.pos.map(|p| self.order[p]);
            self.order.sort_unstable();
            self.pos = current.and_then(|c| self.order.iter().position(|&i| i == c));
        }
    }

    fn reshuffle_keeping_current(&mut self) {
        let current = self.pos.map(|p| self.order[p]);
        let mut rest: Vec<usize> = self
            .order
            .iter()
            .copied()
            .filter(|&i| Some(i) != current)
            .collect();
        // Fisher-Yates with our own xorshift: deterministic with the seed (tests).
        for i in (1..rest.len()).rev() {
            let j = (self.next_rand() % (i as u64 + 1)) as usize;
            rest.swap(i, j);
        }
        self.order = current.into_iter().chain(rest).collect();
        self.pos = current.map(|_| 0);
    }

    fn next_rand(&mut self) -> u64 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.seed = x;
        x
    }

    #[cfg(test)]
    fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Index that would play after the current one, without moving. `auto` = natural
    /// end of the song (honors "repeat one"); `false` = Next button.
    pub fn peek_next(&self, auto: bool) -> Option<usize> {
        let p = self.pos?;
        if auto && self.repeat == RepeatMode::One {
            return Some(p);
        }
        if p + 1 < self.order.len() {
            Some(p + 1)
        } else if self.repeat != RepeatMode::Off {
            Some(0)
        } else {
            None
        }
    }

    pub fn track_at(&self, index: usize) -> Option<&Track> {
        self.order.get(index).map(|&i| &self.tracks[i])
    }

    /// Advances. Returns `false` if the end was reached (the service decides whether
    /// to fall back to autoplay).
    pub fn advance(&mut self, auto: bool) -> bool {
        match self.peek_next(auto) {
            Some(n) => {
                self.pos = Some(n);
                true
            }
            None => false,
        }
    }

    pub fn previous(&mut self, position_ms: u32) -> Previous {
        let Some(p) = self.pos else {
            return Previous::Restart;
        };
        if position_ms > RESTART_THRESHOLD_MS {
            return Previous::Restart;
        }
        if p > 0 {
            self.pos = Some(p - 1);
            Previous::Moved
        } else if self.repeat == RepeatMode::All && self.order.len() > 1 {
            self.pos = Some(self.order.len() - 1);
            Previous::Moved
        } else {
            Previous::Restart
        }
    }

    pub fn snapshot(&self) -> QueueSnapshot {
        QueueSnapshot {
            tracks: self.tracks_in_order().cloned().collect(),
            current_index: self.pos.map(|p| p as u32),
            source: self.source.clone(),
            autoplay: self.autoplay,
            shuffle: self.shuffle,
            repeat: self.repeat,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TrackId;

    fn tr(id: &str) -> Track {
        Track {
            id: TrackId(id.into()),
            title: id.into(),
            artists: vec![],
            album: None,
            duration_ms: None,
            thumbnails: vec![],
            explicit: false,
            like_status: None,
            set_video_id: None,
        }
    }

    fn ids(q: &Queue) -> Vec<String> {
        q.tracks_in_order().map(|t| t.id.0.clone()).collect()
    }

    fn cur(q: &Queue) -> &str {
        &q.current().unwrap().id.0
    }

    fn queue(n: usize, start: usize) -> Queue {
        let mut q = Queue::new();
        q.set(
            (0..n).map(|i| tr(&format!("t{i}"))).collect(),
            start,
            QueueSource::Manual,
        );
        q
    }

    #[test]
    fn set_and_basic_advance() {
        let mut q = queue(3, 1);
        assert_eq!(cur(&q), "t1");
        assert!(q.advance(false));
        assert_eq!(cur(&q), "t2");
        assert!(
            !q.advance(false),
            "at the end without repeat it does not advance"
        );
        assert_eq!(cur(&q), "t2");
    }

    #[test]
    fn set_empty_has_no_current() {
        let mut q = Queue::new();
        q.set(vec![], 5, QueueSource::Manual);
        assert!(q.current().is_none());
        assert!(!q.advance(false));
    }

    #[test]
    fn repeat_all_wraps_to_start() {
        let mut q = queue(2, 1);
        q.set_repeat(RepeatMode::All);
        assert!(q.advance(true));
        assert_eq!(cur(&q), "t0");
    }

    #[test]
    fn repeat_one_only_on_natural_end() {
        let mut q = queue(3, 0);
        q.set_repeat(RepeatMode::One);
        assert_eq!(q.peek_next(true), Some(0));
        assert!(q.advance(true));
        assert_eq!(cur(&q), "t0");
        assert!(q.advance(false), "the Next button does advance");
        assert_eq!(cur(&q), "t1");
    }

    #[test]
    fn previous_restarts_or_goes_back() {
        let mut q = queue(3, 1);
        assert_eq!(q.previous(10_000), Previous::Restart);
        assert_eq!(cur(&q), "t1");
        assert_eq!(q.previous(1_000), Previous::Moved);
        assert_eq!(cur(&q), "t0");
        assert_eq!(
            q.previous(0),
            Previous::Restart,
            "on the first without repeat it restarts"
        );
        q.set_repeat(RepeatMode::All);
        assert_eq!(q.previous(0), Previous::Moved);
        assert_eq!(cur(&q), "t2");
    }

    #[test]
    fn add_next_and_at_end() {
        let mut q = queue(3, 0);
        q.add(vec![tr("n")], InsertPosition::Next);
        q.add(vec![tr("e")], InsertPosition::End);
        assert_eq!(ids(&q), ["t0", "n", "t1", "t2", "e"]);
        assert_eq!(cur(&q), "t0");
    }

    #[test]
    fn add_to_empty_queue_sets_current() {
        let mut q = Queue::new();
        q.add(vec![tr("a"), tr("b")], InsertPosition::Next);
        assert_eq!(cur(&q), "a");
    }

    #[test]
    fn remove_before_after_and_current() {
        let mut q = queue(4, 2);
        assert!(!q.remove(0));
        assert_eq!(cur(&q), "t2");
        assert!(
            !q.remove(2),
            "removing a later one does not change the current"
        );
        assert_eq!(ids(&q), ["t1", "t2"]);
        assert!(
            q.remove(1),
            "removing the current (last) one leaves no current"
        );
        assert!(q.current().is_none());
        let mut q = queue(3, 0);
        assert!(q.remove(0));
        assert_eq!(cur(&q), "t1", "removing the current moves to the next");
    }

    #[test]
    fn move_adjusts_current() {
        let mut q = queue(4, 1);
        q.move_item(1, 3);
        assert_eq!(ids(&q), ["t0", "t2", "t3", "t1"]);
        assert_eq!(cur(&q), "t1");
        q.move_item(0, 3);
        assert_eq!(cur(&q), "t1");
        assert_eq!(q.current_index(), Some(2));
        q.move_item(3, 0);
        assert_eq!(ids(&q), ["t0", "t2", "t3", "t1"]);
        assert_eq!(q.current_index(), Some(3));
    }

    #[test]
    fn shuffle_keeps_current_first_and_all_tracks() {
        let mut q = queue(20, 7).with_seed(42);
        q.set_shuffle(true);
        assert_eq!(cur(&q), "t7");
        assert_eq!(q.current_index(), Some(0));
        let mut sorted = ids(&q);
        sorted.sort();
        let mut expected: Vec<String> = (0..20).map(|i| format!("t{i}")).collect();
        expected.sort();
        assert_eq!(sorted, expected);
        assert_ne!(ids(&q)[1..], expected[..19], "the order must change");
    }

    #[test]
    fn unshuffle_restores_original_order() {
        let mut q = queue(10, 4).with_seed(7);
        q.set_shuffle(true);
        q.advance(false);
        let playing = cur(&q).to_owned();
        q.set_shuffle(false);
        assert_eq!(
            ids(&q),
            (0..10).map(|i| format!("t{i}")).collect::<Vec<_>>()
        );
        assert_eq!(cur(&q), playing);
    }

    #[test]
    fn snapshot_in_playback_order() {
        let mut q = queue(3, 2).with_seed(1);
        q.set_shuffle(true);
        let s = q.snapshot();
        assert_eq!(s.current_index, Some(0));
        assert_eq!(s.tracks[0].id.0, "t2");
        assert!(s.shuffle);
    }

    #[test]
    fn serializes_for_persistence() {
        let q = queue(3, 1);
        let json = serde_json::to_string(&q).unwrap();
        let back: Queue = serde_json::from_str(&json).unwrap();
        assert_eq!(cur(&back), "t1");
    }

    #[test]
    fn mixes_local_files_and_youtube_tracks() {
        let local = |name: &str| Track {
            id: TrackId::local(std::path::Path::new(&format!(r"D:\Music\{name}.flac"))),
            ..tr(name)
        };
        let mut q = Queue::new().with_seed(3);
        q.set(
            vec![local("a"), tr("yt1"), local("b")],
            0,
            QueueSource::Local,
        );
        q.add(vec![tr("yt2")], InsertPosition::Next);
        assert_eq!(ids(&q)[1], "yt2");
        assert!(q.current().unwrap().id.is_local());
        assert!(q.advance(false));
        assert!(!q.current().unwrap().id.is_local());

        q.set_shuffle(true);
        q.set_shuffle(false);
        let json = serde_json::to_string(&q).unwrap();
        let back: Queue = serde_json::from_str(&json).unwrap();
        let locals: Vec<_> = back
            .tracks_in_order()
            .filter_map(|t| t.id.local_path())
            .collect();
        assert_eq!(
            locals,
            [
                std::path::PathBuf::from(r"D:\Music\a.flac"),
                std::path::PathBuf::from(r"D:\Music\b.flac")
            ],
            "local files survive persistence with their paths"
        );
        assert_eq!(back.snapshot().source, Some(QueueSource::Local));
    }

    #[test]
    fn like_marks_all_occurrences() {
        let mut q = Queue::new();
        let a = tr("a");
        q.set(vec![a.clone(), tr("b"), a], 0, QueueSource::Manual);
        assert!(q.set_like(&TrackId("a".into()), LikeStatus::Like));
        let likes: Vec<_> = q.tracks_in_order().map(|x| x.like_status).collect();
        assert_eq!(
            likes,
            [Some(LikeStatus::Like), None, Some(LikeStatus::Like)]
        );
        assert!(!q.set_like(&TrackId("zzz".into()), LikeStatus::Like));
    }
}
