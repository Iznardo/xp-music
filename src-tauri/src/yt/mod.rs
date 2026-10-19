//! Integración con YouTube Music (specs/003).

pub mod innertube;
pub mod parse;
pub mod playback;
pub mod service;
pub mod session;
pub mod thumbs;

#[cfg(test)]
mod dev_dump {
    //! Herramienta de desarrollo: vuelca respuestas reales a
    //! `%USERPROFILE%\.xp-music\dumps\` (fuera del repo; contienen datos personales).
    //! `cargo test --lib -- --ignored --nocapture volcar_respuestas`
    use super::innertube::{Raw, Yt, run};
    use serde_json::json;

    pub async fn client() -> Yt {
        let dir = dirs::home_dir().unwrap().join(".xp-music");
        let cookie = std::fs::read_to_string(dir.join("cookie.txt")).expect("cookie.txt");
        let account: u32 = std::env::var("XP_MUSIC_ACCOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let http = ytmapi_rs::Client::new().unwrap();
        let token = ytmapi_rs::auth::BrowserToken::from_str(cookie.trim(), &http)
            .await
            .unwrap()
            .with_auth_user(account);
        ytmapi_rs::YtMusicBuilder::new_with_client(http)
            .with_auth_token(token)
            .build()
            .unwrap()
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore]
    async fn volcar_respuestas() {
        let yt = client().await;
        let out = dirs::home_dir().unwrap().join(".xp-music").join("dumps");
        std::fs::create_dir_all(&out).unwrap();
        let queries: Vec<(&str, Raw)> = vec![
            ("account_menu", Raw::new("account/account_menu", json!({}))),
            ("home", Raw::browse("FEmusic_home")),
            ("library_playlists", Raw::browse("FEmusic_liked_playlists")),
            ("liked_songs", Raw::browse("VLLM")),
            ("library_songs", Raw::browse("FEmusic_liked_videos")),
            ("library_albums", Raw::browse("FEmusic_liked_albums")),
            (
                "library_artists",
                Raw::browse("FEmusic_library_corpus_track_artists"),
            ),
            ("history", Raw::browse("FEmusic_history")),
            (
                "playlist_bachata",
                Raw::browse("VLPLEcMXldRXRhKU8AfZOALMpD6Fz9G_oE9p"),
            ),
            (
                "playlist_95",
                Raw::browse("VLPLEcMXldRXRhKOzEA87R59bPcpsqBPNN-X"),
            ),
            (
                "search",
                Raw::new("search", json!({ "query": "aventura obsesion" })),
            ),
            (
                "suggestions",
                Raw::new("music/get_search_suggestions", json!({ "input": "avent" })),
            ),
            (
                "radio",
                Raw::new(
                    "next",
                    json!({ "videoId": "yQd4xQBlBa0", "playlistId": "RDAMVMyQd4xQBlBa0", "isAudioOnly": true, "enablePersistentPlaylistPanel": true }),
                ),
            ),
            (
                "player",
                Raw::new("player", json!({ "videoId": "yQd4xQBlBa0" })),
            ),
            ("album", Raw::browse("MPREb_WixYTw0z5H2")),
            ("artist", Raw::browse("UCDaQK4nrsvz27YzufrhJfgg")),
            (
                "search_songs",
                Raw::new(
                    "search",
                    json!({ "query": "aventura", "params": "EgWKAQIIAWoMEA4QChADEAQQCRAF" }),
                ),
            ),
            (
                "search_playlists",
                Raw::new(
                    "search",
                    json!({ "query": "bachata", "params": "EgeKAQQoABagwQDhAKEAMQBBAJEAU%3D" }),
                ),
            ),
            (
                "search_artists",
                Raw::new(
                    "search",
                    json!({ "query": "aventura", "params": "EgWKAQIgAWoMEA4QChADEAQQCRAF" }),
                ),
            ),
        ];
        for (name, q) in queries {
            match run(&yt, q).await {
                Ok(v) => {
                    std::fs::write(
                        out.join(format!("{name}.json")),
                        serde_json::to_string_pretty(&v).unwrap(),
                    )
                    .unwrap();
                    println!("ok    {name}");
                }
                Err(e) => println!("ERROR {name}: {e}"),
            }
        }
    }
}
