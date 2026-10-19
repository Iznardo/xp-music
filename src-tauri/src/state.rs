//! Estado global del backend, registrado con `app.manage(...)`.

use std::path::PathBuf;

use crate::storage::Db;

pub struct AppState {
    pub db: std::sync::Arc<Db>,
}

/// Carpetas de la app según specs/001: datos en `%APPDATA%\xp-music`, logs en
/// `%LOCALAPPDATA%\xp-music\logs`.
pub struct AppDirs {
    pub data: PathBuf,
    pub logs: PathBuf,
    pub thumbs: PathBuf,
}

impl AppDirs {
    pub fn resolve() -> std::io::Result<Self> {
        let base = |d: Option<PathBuf>| {
            d.ok_or_else(|| std::io::Error::other("carpeta de usuario no disponible"))
        };
        let dirs = Self {
            data: base(dirs::config_dir())?.join("xp-music"),
            logs: base(dirs::data_local_dir())?.join("xp-music").join("logs"),
            thumbs: base(dirs::data_local_dir())?
                .join("xp-music")
                .join("thumbs"),
        };
        std::fs::create_dir_all(&dirs.data)?;
        std::fs::create_dir_all(&dirs.logs)?;
        Ok(dirs)
    }
}
