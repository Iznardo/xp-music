//! User visualizations: `.frag` files in the `visualizations` folder. Each one is
//! the body of a GLSL `main()` that the WebView compiles on top of the shared
//! header (`COMMON` in `src/lib/viz/webgl.ts`). It is only text: no native code
//! runs, and a broken shader just falls back to Bars.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::error::{AppError, AppResult};

/// A shader is a few KB; this stops a stray huge file from flooding the IPC.
const MAX_BYTES: u64 = 64 * 1024;
const MAX_FILES: usize = 64;

#[derive(Serialize, Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CustomViz {
    /// File name without `.frag`.
    pub name: String,
    /// GLSL source, as is.
    pub source: String,
}

/// The user's visualizations folder.
pub fn dir() -> AppResult<PathBuf> {
    let dirs = crate::state::AppDirs::resolve().map_err(|e| AppError::internal(e.to_string()))?;
    let dir = dirs.data.join("visualizations");
    std::fs::create_dir_all(&dir).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(dir)
}

/// The `.frag` files in a folder (not its subfolders), by name.
pub fn list_in(base: &Path) -> Vec<CustomViz> {
    let Ok(entries) = std::fs::read_dir(base) else {
        return Vec::new();
    };
    let mut out: Vec<CustomViz> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let is_frag = path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("frag"));
            let meta = e.metadata().ok()?;
            if !is_frag || !meta.is_file() || meta.len() > MAX_BYTES {
                return None;
            }
            Some(CustomViz {
                name: path.file_stem()?.to_string_lossy().into_owned(),
                source: std::fs::read_to_string(&path).ok()?,
            })
        })
        .collect();
    out.sort_by_key(|v| v.name.to_lowercase());
    out.truncate(MAX_FILES);
    out
}

#[tauri::command]
#[specta::specta]
pub fn viz_custom_list() -> AppResult<Vec<CustomViz>> {
    let dir = dir()?;
    let list = list_in(&dir);
    tracing::info!(dir = %dir.display(), count = list.len(), "custom visualizations found");
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub fn viz_custom_open_dir() -> AppResult<()> {
    crate::os::open_dir(&dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_small_frag_files_by_name() {
        let d = std::env::temp_dir().join(format!("xpm-viz-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("b.frag"), "void main(){}").unwrap();
        std::fs::write(d.join("A.FRAG"), "x").unwrap();
        std::fs::write(d.join("note.txt"), "x").unwrap();
        std::fs::write(d.join("sub").join("c.frag"), "x").unwrap();
        std::fs::write(d.join("huge.frag"), vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        let l = list_in(&d);
        let names: Vec<_> = l.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["A", "b"]);
        assert_eq!(l[1].source, "void main(){}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
