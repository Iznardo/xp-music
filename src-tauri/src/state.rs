//! Global backend state, registered with `app.manage(...)`.

use std::path::PathBuf;

use crate::storage::Db;

pub struct AppState {
    pub db: std::sync::Arc<Db>,
}

/// App folders per specs/001: data in `%APPDATA%\xp-music`, logs in
/// `%LOCALAPPDATA%\xp-music\logs`.
pub struct AppDirs {
    pub data: PathBuf,
    pub logs: PathBuf,
    pub thumbs: PathBuf,
}

impl AppDirs {
    pub fn resolve() -> std::io::Result<Self> {
        let base = |d: Option<PathBuf>| {
            d.ok_or_else(|| std::io::Error::other("user directory not available"))
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
