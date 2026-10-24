//! Local SQLite with versioned migrations (FR-001-03).

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::{AppError, AppResult};

/// Migrations in order. Never edit one that has been released: append a new one at the end.
fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // 0001 · settings (001)
        M::up("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);"),
        // 0002 · persisted queue and resolution cache (002)
        M::up(
            "CREATE TABLE queue_state (id INTEGER PRIMARY KEY CHECK (id = 1), json TEXT NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE resolve_cache (track_id TEXT NOT NULL, itag INTEGER NOT NULL, json TEXT NOT NULL,
                                         expires_at INTEGER NOT NULL, PRIMARY KEY (track_id, itag));",
        ),
        // 0003 · YouTube Music API and thumbnail cache (003)
        M::up(
            "CREATE TABLE api_cache (key TEXT PRIMARY KEY, json TEXT NOT NULL, fetched_at INTEGER NOT NULL);
             CREATE TABLE thumb_index (name TEXT PRIMARY KEY, url TEXT NOT NULL, mime TEXT NOT NULL,
                                       bytes INTEGER NOT NULL, last_access INTEGER NOT NULL);",
        ),
    ])
}

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrations().to_latest(&mut conn).map_err(|e| {
            tracing::error!(error = %e, "migration failed");
            AppError::new(
                crate::error::ErrorCode::Storage,
                tr!(
                    "Could not update the local database.",
                    "No se pudo actualizar la base de datos local."
                ),
            )
        })?;
        Ok(Self(Mutex::new(conn)))
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        migrations().to_latest(&mut conn).expect("migrations");
        Ok(Self(Mutex::new(conn)))
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Standalone value from the `settings` table (internal timestamps, etc.).
    pub fn kv_get(&self, key: &str) -> AppResult<Option<String>> {
        use rusqlite::OptionalExtension;
        Ok(self
            .conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn kv_set(&self, key: &str, value: &str) -> AppResult<()> {
        self.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        migrations().validate().unwrap();
    }
}
