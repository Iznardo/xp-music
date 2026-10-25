//! Domain types shared between backend and frontend (contract from specs/001/plan.md).
//!
//! The TS types are generated from here with tauri-specta; they are not written by hand.

use serde::{Deserialize, Serialize};
use specta::Type;

/// YouTube `videoId` (11 characters) or a file on disk as `local:<absolute path>`.
/// A videoId never contains `:`, so the two cannot collide.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrackId(pub String);

const LOCAL_PREFIX: &str = "local:";

impl TrackId {
    pub fn local(path: &std::path::Path) -> Self {
        Self(format!("{LOCAL_PREFIX}{}", path.display()))
    }

    pub fn is_local(&self) -> bool {
        self.0.starts_with(LOCAL_PREFIX)
    }

    /// The file of a local track (`None` for YouTube ones).
    pub fn local_path(&self) -> Option<std::path::PathBuf> {
        self.0.strip_prefix(LOCAL_PREFIX).map(Into::into)
    }
}

/// Playlist id as returned by the library (with the `VL` prefix).
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlaylistId(pub String);

/// Album or artist `browseId`.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrowseId(pub String);

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtistRef {
    pub id: Option<BrowseId>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumRef {
    pub id: Option<BrowseId>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnail {
    pub url: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LikeStatus {
    Like,
    Dislike,
    Indifferent,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub title: String,
    pub artists: Vec<ArtistRef>,
    pub album: Option<AlbumRef>,
    pub duration_ms: Option<u32>,
    pub thumbnails: Vec<Thumbnail>,
    pub explicit: bool,
    pub like_status: Option<LikeStatus>,
    /// Needed to edit playlists.
    pub set_video_id: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistSummary {
    pub id: PlaylistId,
    pub title: String,
    pub author: Option<String>,
    pub track_count: Option<u32>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub continuation: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumSummary {
    pub id: BrowseId,
    pub title: String,
    pub artists: Vec<ArtistRef>,
    /// Year or type ("2002", "Single • 2019"), as given by YouTube.
    pub subtitle: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtistSummary {
    pub id: BrowseId,
    pub name: String,
    pub subtitle: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

/// Item of a Home, artist or search section.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
pub enum MediaItem {
    Track(Track),
    Playlist(PlaylistSummary),
    Album(AlbumSummary),
    Artist(ArtistSummary),
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub title: String,
    pub items: Vec<MediaItem>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDetail {
    pub summary: PlaylistSummary,
    pub description: Option<String>,
    /// "103 views • 41 tracks • 2 hours, 46 minutes"
    pub stats: Option<String>,
    pub owned: bool,
    pub tracks: Page<Track>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDetail {
    pub summary: AlbumSummary,
    /// Playlist with the album's tracks (to play it in full).
    pub playlist_id: Option<PlaylistId>,
    pub stats: Option<String>,
    pub description: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ArtistDetail {
    pub summary: ArtistSummary,
    pub description: Option<String>,
    pub top_songs: Vec<Track>,
    /// Albums, singles, videos, similar artists… in YouTube's order.
    pub sections: Vec<Section>,
    /// Song to start the artist's radio from.
    pub radio_seed: Option<TrackId>,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchFilter {
    All,
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Privacy {
    Private,
    Unlisted,
    Public,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_camel_case() {
        let t = Track {
            id: TrackId("yQd4xQBlBa0".into()),
            title: "Obsesion".into(),
            artists: vec![ArtistRef {
                id: None,
                name: "Aventura".into(),
            }],
            album: None,
            duration_ms: Some(255_000),
            thumbnails: vec![],
            explicit: false,
            like_status: Some(LikeStatus::Like),
            set_video_id: None,
        };
        let json = serde_json::to_value(&t).unwrap();
        assert_eq!(json["id"], "yQd4xQBlBa0");
        assert_eq!(json["durationMs"], 255_000);
        assert_eq!(json["likeStatus"], "Like");
        assert!(json.get("setVideoId").is_some());
    }

    #[test]
    fn local_ids_round_trip_and_never_look_like_youtube() {
        let path = std::path::Path::new(r"C:\Music\Café del Mar\01 - Intro.flac");
        let id = TrackId::local(path);
        assert!(id.is_local());
        assert_eq!(id.local_path().as_deref(), Some(path));
        let yt = TrackId("yQd4xQBlBa0".into());
        assert!(!yt.is_local());
        assert_eq!(yt.local_path(), None);
    }
}
