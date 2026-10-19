//! Ajustes persistentes (FR-001-04).
//!
//! Se guardan como un único JSON en la tabla `settings` (clave `app`). Al
//! cargar se fusiona sobre los valores por defecto, así que un JSON antiguo
//! sin los campos nuevos sigue siendo válido.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri_specta::Event;

use crate::error::AppResult;
use crate::storage::Db;

const KEY: &str = "app";

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Blue,
    Olive,
    Silver,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioQuality {
    /// itag 140
    #[default]
    Aac128,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VizPreset {
    #[default]
    Bars,
    Waves,
    Ambience,
    Battery,
    Cover,
}

#[derive(Serialize, Deserialize, Type, Event, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[tauri_specta(event_name = "settings://changed")]
pub struct Settings {
    pub theme: Theme,
    /// 0.0..=1.0
    pub volume: f32,
    pub normalize_volume: bool,
    pub audio_quality: AudioQuality,
    /// `None` = dispositivo predeterminado.
    pub output_device: Option<String>,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub launch_at_login: bool,
    pub report_history: bool,
    /// `None` = sin visualización.
    pub visualization: Option<VizPreset>,
    pub xp_sounds: bool,
    pub notifications: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Blue,
            volume: 0.8,
            normalize_volume: true,
            audio_quality: AudioQuality::Aac128,
            output_device: None,
            close_to_tray: true,
            start_minimized: false,
            launch_at_login: false,
            report_history: true,
            visualization: Some(VizPreset::Bars),
            xp_sounds: true,
            notifications: true,
        }
    }
}

impl Settings {
    /// Ajusta valores fuera de rango en lugar de rechazarlos.
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

    /// Fusiona el JSON guardado sobre los valores por defecto; si está corrupto,
    /// devuelve los predeterminados.
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
                tracing::warn!(error = %e, "ajustes corruptos; se usan los predeterminados");
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
    fn sin_guardar_devuelve_predeterminados() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(Settings::load(&db).unwrap(), Settings::default());
    }

    #[test]
    fn guarda_y_carga() {
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
    fn volumen_fuera_de_rango_se_recorta() {
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
    fn json_antiguo_sin_campos_nuevos_es_valido() {
        let s = Settings::from_stored_json(r#"{"theme":"Silver"}"#);
        assert_eq!(s.theme, Theme::Silver);
        assert_eq!(s.volume, 0.8);
    }

    #[test]
    fn json_corrupto_da_predeterminados() {
        assert_eq!(
            Settings::from_stored_json("{no es json"),
            Settings::default()
        );
        assert_eq!(
            Settings::from_stored_json(r#"{"theme":"Rosa"}"#),
            Settings::default()
        );
    }
}
