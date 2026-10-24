//! Rolling-file logging with secret redaction (FR-001-05, principle VI).
//!
//! Every line goes through [`redact`] before reaching disk or the console, so
//! a careless `debug!` with headers does not leak cookies.

use std::borrow::Cow;
use std::io::{self, Write};
use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

const MASK: &str = "[REDACTED]";

/// Headers whose value is hidden until the end of the line.
const HEADER_KEYS: &[&str] = &["cookie:", "authorization:", "x-goog-authuser:"];

/// Markers whose value is hidden until the next separator.
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

/// Hides cookies, authentication headers and tokens in a log line.
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

/// `Write` that redacts each chunk before passing it to the real writer.
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

/// Starts logging. The `WorkerGuard` must live as long as the app is open.
pub fn init(log_dir: &Path) -> WorkerGuard {
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("xp-music")
        .filename_suffix("log")
        .max_log_files(7)
        .build(log_dir)
        .expect("could not create the log file");
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
    fn redacts_full_cookie_header() {
        let line = "GET /browse Cookie: SID=abc; HSID=def; PREF=x\nnext";
        let r = redact(line);
        assert!(
            !r.contains("abc") && !r.contains("def") && !r.contains("PREF=x"),
            "{r}"
        );
        assert!(r.ends_with("\nnext"));
    }

    #[test]
    fn redacts_sapisidhash_and_loose_cookies() {
        let line = r#"headers: Authorization: SAPISIDHASH 1700000000_abcdef, url="x?SAPISID=zzz;__Secure-3PSID=yyy""#;
        let r = redact(line);
        for secret in ["1700000000_abcdef", "zzz", "yyy"] {
            assert!(!r.contains(secret), "{secret} still in: {r}");
        }
    }

    #[test]
    fn redacts_github_token() {
        let r = redact("token=github_pat_11ABCDEF done");
        assert!(!r.contains("11ABCDEF"));
        assert!(r.ends_with(" done"));
    }

    #[test]
    fn leaves_normal_lines_untouched() {
        let line = "playing yQd4xQBlBa0 at 48000 Hz";
        assert!(matches!(redact(line), Cow::Borrowed(_)));
    }

    #[test]
    fn writer_redacts() {
        let mut out = Vec::new();
        RedactingWriter(&mut out)
            .write_all(b"cookie: SID=secret\n")
            .unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(!s.contains("secret"), "{s}");
    }
}
