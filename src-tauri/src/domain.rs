//! Tipos de dominio compartidos entre backend y frontend (contrato de specs/001/plan.md).
//!
//! Los tipos TS se generan desde aquí con tauri-specta; no se escriben a mano.

use serde::{Deserialize, Serialize};
use specta::Type;

/// `videoId` de YouTube (11 caracteres).
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrackId(pub String);

/// Id de playlist tal como lo devuelve la biblioteca (con prefijo `VL`).
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlaylistId(pub String);

/// `browseId` de álbum o artista.
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
    /// Necesario para editar playlists.
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

#[allow(dead_code)] // Se usa a partir de 003 (paginación).
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub continuation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_en_camel_case() {
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
}
