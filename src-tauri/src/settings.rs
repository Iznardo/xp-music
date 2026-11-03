//! Persistent settings (FR-001-04).
//!
//! Stored as a single JSON in the `settings` table (key `app`). On load it is
//! merged over the defaults, so an old JSON without the new fields is still
//! valid.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::error::AppResult;
use crate::i18n::Language;
use crate::storage::Db;

const KEY: &str = "app";

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Blue,
    Olive,
    Silver,
}

/// Light or dark variant of the color scheme; combines with any `Theme`.
#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Appearance {
    /// Follow the system preference (`prefers-color-scheme`).
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioQuality {
    /// itag 140
    #[default]
    Aac128,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VizPreset {
    Bars,
    Waves,
    Ambience,
    Battery,
    /// Rays bursting from the center with a trail (WMP 9 "Ambience: Water" style).
    #[default]
    Water,
    /// Psychedelic plasma with swirls.
    Alchemy,
    /// Hypnotic spiral tunnel.
    Spiral,
    Cover,
}

#[derive(Serialize, Deserialize, Type, Event, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "settings://changed")]
pub struct Settings {
    pub theme: Theme,
    pub appearance: Appearance,
    /// `None` = follow the system language.
    pub language: Option<Language>,
    /// 0.0..=1.0
    pub volume: f32,
    pub normalize_volume: bool,
    pub audio_quality: AudioQuality,
    /// `None` = default device.
    pub output_device: Option<String>,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub launch_at_login: bool,
    pub report_history: bool,
    /// `None` = no visualization.
    pub visualization: Option<VizPreset>,
    /// Name of a user visualization (`visualizations/<name>.frag`). It wins over
    /// `visualization` while the file exists.
    pub custom_visualization: Option<String>,
    pub xp_sounds: bool,
    pub notifications: bool,
    /// Pause when the Windows session is locked (006).
    pub pause_on_lock: bool,
    /// Show the current song as a Discord activity. Off by default: it tells others what you listen to.
    pub discord_presence: bool,
    /// Global shortcuts (006). Format `Ctrl+Alt+P`; `None` = no shortcut.
    pub hotkeys: HotkeySettings,
    /// Folders of "My music" (local files), absolute paths.
    pub local_folders: Vec<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySettings {
    pub play_pause: Option<String>,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub show: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Blue,
            appearance: Appearance::System,
            language: None,
            volume: 0.8,
            normalize_volume: true,
            audio_quality: AudioQuality::Aac128,
            output_device: None,
            // GNOME may not show AppIndicators; do not leave the app invisible. On macOS
            // closing the window quits, as on Linux.
            close_to_tray: cfg!(windows),
            start_minimized: false,
            launch_at_login: false,
            report_history: true,
            visualization: Some(VizPreset::Water),
            custom_visualization: None,
            xp_sounds: true,
            notifications: true,
            pause_on_lock: false,
            discord_presence: false,
            hotkeys: HotkeySettings::default(),
            local_folders: Vec::new(),
        }
    }
}

impl Settings {
    /// Clamps out-of-range values instead of rejecting them.
    fn sanitized(mut self) -> Self {
        self.volume = if self.volume.is_finite() {
            self.volume.clamp(0.0, 1.0)
        } else {
            0.8
        };
        self
    }

    pub fn load(db: &Db) -> AppResult<Self> {
        let conn = db.conn();
        let raw: Option<String> = conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [KEY], |r| {
                r.get(0)
            })
            .map(Some)
            .or_else(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    Ok(None)
                } else {
                    Err(e)
                }
            })?;
        Ok(match raw {
            Some(json) => Self::from_stored_json(&json),
            None => Settings::default(),
        })
    }

    /// Merges the stored JSON over the defaults; if it is corrupt, returns the
    /// defaults.
    fn from_stored_json(json: &str) -> Self {
        let merged = serde_json::from_str::<serde_json::Value>(json).and_then(|stored| {
            let mut base = serde_json::to_value(Settings::default())?;
            if let (Some(base), serde_json::Value::Object(stored)) = (base.as_object_mut(), stored)
            {
                base.extend(stored);
            }
            serde_json::from_value::<Settings>(base)
        });
        merged
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "corrupt settings; using defaults");
                Settings::default()
            })
            .sanitized()
    }

    pub fn save(self, db: &Db) -> AppResult<Self> {
        let s = self.sanitized();
        db.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (KEY, serde_json::to_string(&s)?),
        )?;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsaved_returns_defaults() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(Settings::load(&db).unwrap(), Settings::default());
    }

    #[test]
    fn saves_and_loads() {
        let db = Db::open_in_memory().unwrap();
        let s = Settings {
            theme: Theme::Olive,
            volume: 0.3,
            ..Default::default()
        };
        s.clone().save(&db).unwrap();
        assert_eq!(Settings::load(&db).unwrap(), s);
    }

    #[test]
    fn out_of_range_volume_is_clamped() {
        let db = Db::open_in_memory().unwrap();
        let s = Settings {
            volume: 7.0,
            ..Default::default()
        }
        .save(&db)
        .unwrap();
        assert_eq!(s.volume, 1.0);
    }

    #[test]
    fn old_json_without_new_fields_is_valid() {
        let s = Settings::from_stored_json(r#"{"theme":"Silver"}"#);
        assert_eq!(s.theme, Theme::Silver);
        assert_eq!(s.appearance, Appearance::System);
        assert_eq!(s.volume, 0.8);
    }

    #[test]
    fn corrupt_json_gives_defaults() {
        assert_eq!(Settings::from_stored_json("{not json"), Settings::default());
        assert_eq!(
            Settings::from_stored_json(r#"{"theme":"Pink"}"#),
            Settings::default()
        );
    }
}
