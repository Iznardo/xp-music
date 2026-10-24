//! Minimal localisation for the strings the backend shows to the user
//! (tray menu, taskbar buttons, error messages, window titles).
//!
//! English is the base language. Every message is written inline as an
//! English/Spanish pair with [`tr!`]; the active language is a process-wide
//! value set from the `language` setting (`None` = follow the system locale).

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Es,
}

impl Language {
    /// `es`, `es-ES`, `es_MX.UTF-8`… are Spanish; anything else is English.
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag
            .trim()
            .split(['-', '_', '.', '@', ':'])
            .next()
            .unwrap_or("");
        if primary.eq_ignore_ascii_case("es") {
            Language::Es
        } else {
            Language::En
        }
    }

    /// Language of the operating system's UI locale.
    pub fn system() -> Self {
        system_locale().map_or(Language::En, |tag| Self::from_tag(&tag))
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(Language::En as u8);

pub fn current() -> Language {
    if CURRENT.load(Ordering::Relaxed) == Language::Es as u8 {
        Language::Es
    } else {
        Language::En
    }
}

/// Applies the `language` setting. Returns `true` if the active language changed.
pub fn set(choice: Option<Language>) -> bool {
    let lang = choice.unwrap_or_else(Language::system);
    CURRENT.swap(lang as u8, Ordering::Relaxed) != lang as u8
}

/// `tr!("English", "Español")` → `String` in the active language.
/// With format arguments (`tr!("Failed: {e}", "Falló: {e}", e = e)`) both
/// templates are formatted with them.
macro_rules! tr {
    ($en:literal, $es:literal) => {
        match $crate::i18n::current() {
            $crate::i18n::Language::Es => $es.to_owned(),
            $crate::i18n::Language::En => $en.to_owned(),
        }
    };
    ($en:literal, $es:literal, $($args:tt)+) => {
        match $crate::i18n::current() {
            $crate::i18n::Language::Es => format!($es, $($args)+),
            $crate::i18n::Language::En => format!($en, $($args)+),
        }
    };
}

#[cfg(windows)]
fn system_locale() -> Option<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // SAFETY: plain query with no arguments.
    let langid = unsafe { GetUserDefaultUILanguage() };
    // The low 10 bits are the primary language; 0x0a is LANG_SPANISH.
    Some(if langid & 0x3ff == 0x0a { "es" } else { "en" }.to_owned())
}

#[cfg(target_os = "macos")]
fn system_locale() -> Option<String> {
    // GUI apps do not inherit `LANG`; ask the system for the preferred language.
    let out = std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .ok()?;
    // Output looks like `(\n    "es-ES",\n    "en-US"\n)`.
    String::from_utf8_lossy(&out.stdout)
        .split('"')
        .nth(1)
        .map(str::to_owned)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn system_locale() -> Option<String> {
    // POSIX precedence: LC_ALL, then LC_MESSAGES, then LANG.
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanish_tags_are_detected() {
        for tag in ["es", "es-ES", "es_MX.UTF-8", "ES-419", " es_AR "] {
            assert_eq!(Language::from_tag(tag), Language::Es, "{tag}");
        }
    }

    #[test]
    fn other_tags_fall_back_to_english() {
        for tag in [
            "",
            "en-US",
            "en_GB.UTF-8",
            "est",
            "et-EE",
            "C",
            "POSIX",
            "fr-FR",
        ] {
            assert_eq!(Language::from_tag(tag), Language::En, "{tag}");
        }
    }

    #[test]
    fn tr_follows_the_active_language() {
        set(Some(Language::Es));
        assert_eq!(tr!("Hello", "Hola"), "Hola");
        assert_eq!(tr!("Hi {n}", "Hola {n}", n = 3), "Hola 3");
        set(Some(Language::En));
        assert_eq!(tr!("Hello", "Hola"), "Hello");
        assert_eq!(tr!("Hi {n}", "Hola {n}", n = 3), "Hi 3");
    }
}
