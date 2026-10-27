//! Fast start (specs/008): YouTube's internal API with the Android client
//! returns a direct audio URL in ~0.3 s, but without a "PO token" YouTube only
//! serves the first ~1 MB through it (≈ 65 s at 128 kbps). That is enough to
//! start playing while yt-dlp obtains the full URL (~2 s), which the source
//! uses from the second block on (see `Upgrade` and `source.rs`).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::resolver::{FormatPreference, ResolvedStream};
use crate::domain::TrackId;
use crate::error::{AppError, AppResult, ErrorCode};

const ENDPOINT: &str = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
const CLIENT_VERSION: &str = "20.10.38";
const UA: &str = "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip";
/// Bytes that YouTube serves without a PO token (measured: up to ~1.06 MB).
pub const HEAD_LIMIT: u64 = 1 << 20;

fn client() -> &'static reqwest::blocking::Client {
    static C: std::sync::OnceLock<reqwest::blocking::Client> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .expect("HTTP client")
    })
}

fn fail(why: &str) -> AppError {
    tracing::debug!(why, "fast start not available");
    AppError::new(
        ErrorCode::ResolveFailed,
        tr!(
            "Fast start is not available.",
            "Arranque rápido no disponible."
        ),
    )
}

/// URL of the start of the audio. Any failure = use yt-dlp only.
pub fn resolve_head(id: &TrackId, pref: FormatPreference) -> AppResult<ResolvedStream> {
    let body = json!({
        "videoId": id.0,
        "context": { "client": {
            "clientName": "ANDROID",
            "clientVersion": CLIENT_VERSION,
            "androidSdkVersion": 34,
            "hl": "es",
            "gl": "ES",
        }},
        "contentCheckOk": true,
        "racyCheckOk": true,
    });
    let v: Value = client()
        .post(ENDPOINT)
        .header("User-Agent", UA)
        .json(&body)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|_| fail("network"))?;
    parse(id, pref, &v)
}

fn parse(id: &TrackId, pref: FormatPreference, v: &Value) -> AppResult<ResolvedStream> {
    if v["playabilityStatus"]["status"] != "OK" {
        return Err(fail("not playable with this client"));
    }
    let itag = pref.itag();
    let f = v["streamingData"]["adaptiveFormats"]
        .as_array()
        // The dynamic range compression (DRC) variant is a different file: not usable.
        .and_then(|a| {
            a.iter().find(|f| {
                f["itag"].as_u64() == Some(itag as u64)
                    && f["isDrc"].as_bool() != Some(true)
                    && f["xtags"].is_null()
            })
        })
        .ok_or_else(|| fail("format not found"))?;
    // An encrypted signature (`signatureCipher`) is no good: that is yt-dlp's job.
    let url = f["url"]
        .as_str()
        .ok_or_else(|| fail("encrypted URL"))?
        .to_owned();
    let num = |x: &Value| x.as_str().and_then(|s| s.parse::<u64>().ok());
    let expires_at = url
        .split(['?', '&'])
        .find_map(|kv| kv.strip_prefix("expire="))
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + 3600
        });
    Ok(ResolvedStream {
        track_id: id.clone(),
        url,
        itag,
        content_length: num(&f["contentLength"]),
        duration_ms: num(&f["approxDurationMs"]).map(|d| d as u32),
        expires_at,
        available_at: None,
        user_agent: Some(UA.to_owned()),
        upgrade: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_response_with_direct_url() {
        let v = json!({
            "playabilityStatus": { "status": "OK" },
            "streamingData": { "adaptiveFormats": [
                { "itag": 251, "url": "https://x/opus" },
                { "itag": 140, "url": "https://x/aac?expire=1999999999&a=1", "contentLength": "5818745", "approxDurationMs": "359000" }
            ]}
        });
        let s = parse(&TrackId("abc".into()), FormatPreference::Aac128, &v).unwrap();
        assert_eq!(s.url, "https://x/aac?expire=1999999999&a=1");
        assert_eq!(s.content_length, Some(5_818_745));
        assert_eq!(s.duration_ms, Some(359_000));
        assert_eq!(s.expires_at, 1_999_999_999);
    }

    #[test]
    fn rejects_encrypted_signature_and_unplayable() {
        let encrypted = json!({
            "playabilityStatus": { "status": "OK" },
            "streamingData": { "adaptiveFormats": [{ "itag": 140, "signatureCipher": "s=..." }] }
        });
        assert!(parse(&TrackId("a".into()), FormatPreference::Aac128, &encrypted).is_err());
        let bot = json!({ "playabilityStatus": { "status": "LOGIN_REQUIRED" } });
        assert!(parse(&TrackId("a".into()), FormatPreference::Aac128, &bot).is_err());
    }

    /// Real network: `cargo test --lib -- --ignored --nocapture fast_start_real`.
    #[test]
    #[ignore]
    fn fast_start_real() {
        let t = std::time::Instant::now();
        let s = resolve_head(&TrackId("kJQP7kiw5Fk".into()), FormatPreference::Aac128).unwrap();
        println!(
            "{} ms, {:?} bytes",
            t.elapsed().as_millis(),
            s.content_length
        );
    }

    /// With real YouTube: a fast start produces audio before yt-dlp's full URL exists.
    /// Format detection must not read past the head (it used to probe for ID3v1 at the
    /// end of the file and wait for the full URL, delaying the start by seconds).
    /// `cargo test --lib -- --ignored --nocapture fast_start_plays_before_the_full_url`.
    #[test]
    #[ignore]
    fn fast_start_plays_before_the_full_url() {
        use crate::audio::decoder::{Decoding, MediaStream};
        use crate::audio::resolver::Upgrade;
        let t = std::time::Instant::now();
        let mut head =
            resolve_head(&TrackId("fJ9rUzIMcZQ".into()), FormatPreference::Aac128).unwrap();
        // The full URL never arrives: only the head can be used.
        head.upgrade = Some(Upgrade::default());
        let client = reqwest::blocking::Client::new();
        let mut dec = Decoding::open(client, &MediaStream::Remote(head), 0, 48_000, 2).unwrap();
        let mut out = Vec::new();
        dec.next_chunk(&mut out).unwrap();
        println!("first audio in {} ms", t.elapsed().as_millis());
        assert!(!out.is_empty());
        assert!(t.elapsed() < std::time::Duration::from_secs(5));
    }

    /// End to end with real YouTube: the start with the fast URL and the rest
    /// with yt-dlp's. `cargo test --lib -- --ignored --nocapture fast_start_full`.
    #[test]
    #[ignore]
    fn fast_start_full() {
        use crate::audio::resolver::{StreamResolver, ToolPaths, Upgrade, YtDlpResolver};
        use crate::audio::source::HttpRangeSource;
        use std::io::Read;
        let id = TrackId("fJ9rUzIMcZQ".into());
        let up = Upgrade::default();
        let (id2, up2) = (id.clone(), up.clone());
        std::thread::spawn(move || {
            let r = YtDlpResolver::new(ToolPaths::locate().unwrap(), None);
            up2.fill(r.resolve(&id2, FormatPreference::Aac128));
        });
        let t = std::time::Instant::now();
        let head = resolve_head(&id, FormatPreference::Aac128).unwrap();
        let client = reqwest::blocking::Client::new();
        let mut src = HttpRangeSource::open(
            client,
            &head.url,
            head.user_agent.as_deref(),
            head.content_length,
            head.available_at,
            Some(up),
        )
        .unwrap();
        let mut first = [0u8; 4096];
        src.read_exact(&mut first).unwrap();
        println!("first bytes in {} ms", t.elapsed().as_millis());
        let mut rest = Vec::new();
        src.read_to_end(&mut rest).unwrap();
        println!(
            "full song ({} bytes) in {} ms",
            rest.len() + 4096,
            t.elapsed().as_millis()
        );
        assert_eq!(rest.len() as u64 + 4096, head.content_length.unwrap());
    }
}
