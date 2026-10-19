//! Logging a archivo rotativo con redacción de secretos (FR-001-05, principio VI).
//!
//! Toda línea pasa por [`redact`] antes de llegar al disco o a la consola, así
//! un `debug!` descuidado con cabeceras no filtra cookies.

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const MASK: &str = "[REDACTADO]";

/// Cabeceras cuyo valor se oculta hasta el final de la línea.
const HEADER_KEYS: &[&str] = &["cookie:", "authorization:", "x-goog-authuser:"];

/// Marcadores cuyo valor se oculta hasta el siguiente separador.
const VALUE_KEYS: &[&str] = &[
    "SAPISIDHASH ",
    "SAPISID=",
    "APISID=",
    "PSID=",
    "SID=",
    "HSID=",
    "SSID=",
    "PSIDTS=",
    "PSIDCC=",
    "LOGIN_INFO=",
    "Bearer ",
    "github_pat_",
];

fn is_value_end(c: char) -> bool {
    c.is_whitespace() || matches!(c, ';' | '"' | '\'' | ',' | '&')
}

/// Oculta cookies, cabeceras de autenticación y tokens en una línea de log.
pub fn redact(input: &str) -> Cow<'_, str> {
    let lower = input.to_ascii_lowercase();
    let mut spans: Vec<(usize, usize)> = Vec::new();

    for key in HEADER_KEYS {
        let mut from = 0;
        while let Some(i) = lower[from..].find(key) {
            let start = from + i + key.len();
            let end = input[start..].find('\n').map_or(input.len(), |n| start + n);
            spans.push((start, end));
            from = end;
        }
    }
    for key in VALUE_KEYS {
        let mut from = 0;
        while let Some(i) = input[from..].find(key) {
            let start = from + i + key.len();
            let end = input[start..]
                .find(is_value_end)
                .map_or(input.len(), |n| start + n);
            if end > start {
                spans.push((start, end));
            }
            from = end.max(start);
        }
    }
    if spans.is_empty() {
        return Cow::Borrowed(input);
    }

    spans.sort_unstable();
    let mut out = String::with_capacity(input.len());
    let mut pos = 0;
    for (start, end) in spans {
        if end <= pos {
            continue;
        }
        let start = start.max(pos);
        out.push_str(&input[pos..start]);
        out.push_str(MASK);
        pos = end;
    }
    out.push_str(&input[pos..]);
    Cow::Owned(out)
}

/// `Write` que redacta cada bloque antes de pasarlo al escritor real.
pub struct RedactingWriter<W>(W);

impl<W: Write> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match std::str::from_utf8(buf) {
            Ok(s) => self.0.write_all(redact(s).as_bytes())?,
            Err(_) => self.0.write_all(buf)?,
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub struct Redacting<M>(M);

impl<'a, M: MakeWriter<'a>> MakeWriter<'a> for Redacting<M> {
    type Writer = RedactingWriter<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingWriter(self.0.make_writer())
    }
}

/// Inicia el logging. El `WorkerGuard` debe vivir mientras la app esté abierta.
pub fn init(log_dir: &Path) -> WorkerGuard {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("xp-music")
        .filename_suffix("log")
        .max_log_files(7)
        .build(log_dir)
        .expect("no se pudo crear el archivo de log");
    let (file, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env("XP_MUSIC_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_ansi(false).with_writer(Redacting(file)));
    #[cfg(debug_assertions)]
    let registry = registry.with(fmt::layer().with_writer(Redacting(io::stderr)));
    registry.init();
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacta_cabecera_cookie_completa() {
        let line = "GET /browse Cookie: SID=abc; HSID=def; PREF=x\nsiguiente";
        let r = redact(line);
        assert!(
            !r.contains("abc") && !r.contains("def") && !r.contains("PREF=x"),
            "{r}"
        );
        assert!(r.ends_with("\nsiguiente"));
    }

    #[test]
    fn redacta_sapisidhash_y_cookies_sueltas() {
        let line = r#"headers: Authorization: SAPISIDHASH 1700000000_abcdef, url="x?SAPISID=zzz;__Secure-3PSID=yyy""#;
        let r = redact(line);
        for secret in ["1700000000_abcdef", "zzz", "yyy"] {
            assert!(!r.contains(secret), "{secret} sigue en: {r}");
        }
    }

    #[test]
    fn redacta_token_de_github() {
        let r = redact("token=github_pat_11ABCDEF hecho");
        assert!(!r.contains("11ABCDEF"));
        assert!(r.ends_with(" hecho"));
    }

    #[test]
    fn deja_intactas_las_lineas_normales() {
        let line = "reproduciendo yQd4xQBlBa0 a 48000 Hz";
        assert!(matches!(redact(line), Cow::Borrowed(_)));
    }

    #[test]
    fn el_writer_redacta() {
        let mut out = Vec::new();
        RedactingWriter(&mut out)
            .write_all(b"cookie: SID=secreto\n")
            .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(!s.contains("secreto"), "{s}");
    }
}
