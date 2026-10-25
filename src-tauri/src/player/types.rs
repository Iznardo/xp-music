//! Player types shared with the frontend (contract from specs/002/plan.md).

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::domain::{BrowseId, PlaylistId, Track, TrackId};
use crate::error::AppError;

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Idle,
    Loading,
    Buffering,
    Playing,
    Paused,
    Error,
}

/// Where the queue came from (for autoplay and the UI).
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
pub enum QueueSource {
    Playlist(PlaylistId),
    Album(BrowseId),
    Radio(TrackId),
    Search(String),
    Library,
    /// "My music" (files on disk).
    Local,
    Manual,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertPosition {
    Next,
    End,
}

#[derive(Serialize, Type, Event, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "queue://changed")]
pub struct QueueSnapshot {
    /// In effective playback order (already shuffled if `shuffle`).
    pub tracks: Vec<Track>,
    pub current_index: Option<u32>,
    pub source: Option<QueueSource>,
    pub autoplay: bool,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

#[derive(Serialize, Type, Event, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "player://state")]
pub struct PlayerState {
    pub status: PlaybackStatus,
    pub track: Option<Track>,
    pub position_ms: u32,
    pub duration_ms: Option<u32>,
    pub volume: f32,
    pub muted: bool,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub error: Option<AppError>,
}

#[derive(Serialize, Type, Event, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "player://position")]
pub struct PositionUpdate {
    pub position_ms: u32,
    pub buffered_ms: u32,
}

#[derive(Serialize, Type, Event, Clone, Debug)]
#[tauri_specta(event_name = "player://error")]
pub struct PlayerError(pub AppError);

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}
