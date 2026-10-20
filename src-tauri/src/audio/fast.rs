//! Arranque rápido (specs/008): la API interna de YouTube con el cliente de
//! Android devuelve en ~0,3 s una URL directa del audio, pero sin "PO token"
//! YouTube solo sirve con ella el primer ~1 MB (≈ 65 s a 128 kbps). Basta para
//! empezar a sonar mientras yt-dlp consigue la URL completa (~2 s), que la fuente
//! usa a partir del segundo bloque (ver `Upgrade` y `source.rs`).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::resolver::{FormatPreference, ResolvedStream};
use crate::domain::TrackId;
use crate::error::{AppError, AppResult, ErrorCode};

const ENDPOINT: &str = "https://www.youtube.com/youtubei/v1/player?prettyPrint=false";
const CLIENT_VERSION: &str = "20.10.38";
const UA: &str = "com.google.android.youtube/20.10.38 (Linux; U; Android 14) gzip";
/// Bytes que YouTube sirve sin PO token (medido: hasta ~1,06 MB).
pub const HEAD_LIMIT: u64 = 1 << 20;

fn client() -> &'static reqwest::blocking::Client {
    static C: std::sync::OnceLock<reqwest::blocking::Client> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(4))
            .build()
            .expect("cliente HTTP")
    })
}

fn fail(why: &str) -> AppError {
    tracing::debug!(why, "arranque rápido no disponible");
    AppError::new(ErrorCode::ResolveFailed, "Arranque rápido no disponible.")
}

/// URL del principio del audio. Cualquier fallo = usar solo yt-dlp.
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
        .map_err(|_| fail("red"))?;
    parse(id, pref, &v)
}

fn parse(id: &TrackId, pref: FormatPreference, v: &Value) -> AppResult<ResolvedStream> {
    if v["playabilityStatus"]["status"] != "OK" {
        return Err(fail("no reproducible con este cliente"));
    }
    let itag = pref.itag();
    let f = v["streamingData"]["adaptiveFormats"]
        .as_array()
        // La variante con compresión dinámica (DRC) es otro archivo: no vale.
        .and_then(|a| {
            a.iter().find(|f| {
                f["itag"].as_u64() == Some(itag as u64)
                    && f["isDrc"].as_bool() != Some(true)
                    && f["xtags"].is_null()
            })
        })
        .ok_or_else(|| fail("sin el formato"))?;
    // Con firma cifrada (`signatureCipher`) no sirve: eso es trabajo de yt-dlp.
    let url = f["url"]
        .as_str()
        .ok_or_else(|| fail("URL cifrada"))?
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
        user_agent: Some(UA.to_owned()),
        upgrade: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreta_respuesta_con_url_directa() {
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
    fn rechaza_firma_cifrada_y_no_reproducible() {
        let cifrada = json!({
            "playabilityStatus": { "status": "OK" },
            "streamingData": { "adaptiveFormats": [{ "itag": 140, "signatureCipher": "s=..." }] }
        });
        assert!(parse(&TrackId("a".into()), FormatPreference::Aac128, &cifrada).is_err());
        let bot = json!({ "playabilityStatus": { "status": "LOGIN_REQUIRED" } });
        assert!(parse(&TrackId("a".into()), FormatPreference::Aac128, &bot).is_err());
    }

    /// Red real: `cargo test --lib -- --ignored --nocapture arranque_rapido_real`.
    #[test]
    #[ignore]
    fn arranque_rapido_real() {
        let t = std::time::Instant::now();
        let s = resolve_head(&TrackId("kJQP7kiw5Fk".into()), FormatPreference::Aac128).unwrap();
        println!(
            "{} ms, {:?} bytes",
            t.elapsed().as_millis(),
            s.content_length
        );
    }

    /// Extremo a extremo con YouTube real: principio con la URL rápida y el resto
    /// con la de yt-dlp. `cargo test --lib -- --ignored --nocapture arranque_rapido_completo`.
    #[test]
    #[ignore]
    fn arranque_rapido_completo() {
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
            Some(up),
        )
        .unwrap();
        let mut first = [0u8; 4096];
        src.read_exact(&mut first).unwrap();
        println!("primeros bytes en {} ms", t.elapsed().as_millis());
        let mut rest = Vec::new();
        src.read_to_end(&mut rest).unwrap();
        println!(
            "canción completa ({} bytes) en {} ms",
            rest.len() + 4096,
            t.elapsed().as_millis()
        );
        assert_eq!(rest.len() as u64 + 4096, head.content_length.unwrap());
    }
}
