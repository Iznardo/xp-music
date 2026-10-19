//! Cola de reproducción: lógica pura, sin audio ni E/S (FR-002-11..13).
//!
//! `tracks` guarda el orden original y `order` el orden de reproducción
//! (igual al original salvo con aleatorio). Todos los índices públicos se
//! refieren al orden de reproducción, que es lo que ve el usuario.

use serde::{Deserialize, Serialize};

use super::types::{InsertPosition, QueueSnapshot, QueueSource, RepeatMode};
use crate::domain::Track;

/// "Anterior" con más de este tiempo reproducido reinicia la canción.
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

/// Resultado de "anterior".
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

    #[allow(dead_code)] // Pareja obligada de `len` (clippy).
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

    #[allow(dead_code)] // La usará el autoplay con contexto (003).
    pub fn source(&self) -> Option<&QueueSource> {
        self.source.as_ref()
    }

    pub fn tracks_in_order(&self) -> impl Iterator<Item = &Track> {
        self.order.iter().map(|&i| &self.tracks[i])
    }

    /// Sustituye la cola y se coloca en `start` (índice del orden original).
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

    /// Quita la pista en `index`. Devuelve `true` si era la actual: en ese caso
    /// la actual pasa a ser la siguiente (o ninguna si era la última).
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

    /// Activa o desactiva el aleatorio. Al activarlo, la pista actual queda la
    /// primera y el resto se baraja; al desactivarlo se vuelve al orden original
    /// manteniendo la pista actual.
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
        // Fisher-Yates con xorshift propio: determinista con la semilla (tests).
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

    /// Índice que sonaría tras la actual, sin moverse. `auto` = fin natural de
    /// la canción (respeta "repetir una"); `false` = botón Siguiente.
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

    /// Avanza. Devuelve `false` si se llegó al final (el servicio decide si
    /// tirar de autoplay).
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
    fn set_y_avance_basico() {
        let mut q = queue(3, 1);
        assert_eq!(cur(&q), "t1");
        assert!(q.advance(false));
        assert_eq!(cur(&q), "t2");
        assert!(!q.advance(false), "al final sin repetir no avanza");
        assert_eq!(cur(&q), "t2");
    }

    #[test]
    fn set_vacio_no_tiene_actual() {
        let mut q = Queue::new();
        q.set(vec![], 5, QueueSource::Manual);
        assert!(q.current().is_none());
        assert!(!q.advance(false));
    }

    #[test]
    fn repetir_todo_vuelve_al_principio() {
        let mut q = queue(2, 1);
        q.set_repeat(RepeatMode::All);
        assert!(q.advance(true));
        assert_eq!(cur(&q), "t0");
    }

    #[test]
    fn repetir_una_solo_en_fin_natural() {
        let mut q = queue(3, 0);
        q.set_repeat(RepeatMode::One);
        assert_eq!(q.peek_next(true), Some(0));
        assert!(q.advance(true));
        assert_eq!(cur(&q), "t0");
        assert!(q.advance(false), "el botón Siguiente sí avanza");
        assert_eq!(cur(&q), "t1");
    }

    #[test]
    fn anterior_reinicia_o_retrocede() {
        let mut q = queue(3, 1);
        assert_eq!(q.previous(10_000), Previous::Restart);
        assert_eq!(cur(&q), "t1");
        assert_eq!(q.previous(1_000), Previous::Moved);
        assert_eq!(cur(&q), "t0");
        assert_eq!(
            q.previous(0),
            Previous::Restart,
            "en la primera sin repetir reinicia"
        );
        q.set_repeat(RepeatMode::All);
        assert_eq!(q.previous(0), Previous::Moved);
        assert_eq!(cur(&q), "t2");
    }

    #[test]
    fn anadir_siguiente_y_al_final() {
        let mut q = queue(3, 0);
        q.add(vec![tr("n")], InsertPosition::Next);
        q.add(vec![tr("e")], InsertPosition::End);
        assert_eq!(ids(&q), ["t0", "n", "t1", "t2", "e"]);
        assert_eq!(cur(&q), "t0");
    }

    #[test]
    fn anadir_a_cola_vacia_fija_actual() {
        let mut q = Queue::new();
        q.add(vec![tr("a"), tr("b")], InsertPosition::Next);
        assert_eq!(cur(&q), "a");
    }

    #[test]
    fn quitar_antes_despues_y_la_actual() {
        let mut q = queue(4, 2);
        assert!(!q.remove(0));
        assert_eq!(cur(&q), "t2");
        assert!(!q.remove(2), "quitar una posterior no cambia la actual");
        assert_eq!(ids(&q), ["t1", "t2"]);
        assert!(q.remove(1), "quitar la actual (última) la deja sin actual");
        assert!(q.current().is_none());
        let mut q = queue(3, 0);
        assert!(q.remove(0));
        assert_eq!(cur(&q), "t1", "quitar la actual pasa a la siguiente");
    }

    #[test]
    fn mover_ajusta_la_actual() {
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
    fn aleatorio_mantiene_actual_primera_y_todas_las_pistas() {
        let mut q = queue(20, 7).with_seed(42);
        q.set_shuffle(true);
        assert_eq!(cur(&q), "t7");
        assert_eq!(q.current_index(), Some(0));
        let mut sorted = ids(&q);
        sorted.sort();
        let mut expected: Vec<String> = (0..20).map(|i| format!("t{i}")).collect();
        expected.sort();
        assert_eq!(sorted, expected);
        assert_ne!(ids(&q)[1..], expected[..19], "debe cambiar el orden");
    }

    #[test]
    fn quitar_aleatorio_vuelve_al_orden_original() {
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
    fn snapshot_en_orden_de_reproduccion() {
        let mut q = queue(3, 2).with_seed(1);
        q.set_shuffle(true);
        let s = q.snapshot();
        assert_eq!(s.current_index, Some(0));
        assert_eq!(s.tracks[0].id.0, "t2");
        assert!(s.shuffle);
    }

    #[test]
    fn serializa_para_persistir() {
        let q = queue(3, 1);
        let json = serde_json::to_string(&q).unwrap();
        let back: Queue = serde_json::from_str(&json).unwrap();
        assert_eq!(cur(&back), "t1");
    }
}
