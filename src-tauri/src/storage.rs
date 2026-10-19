//! SQLite local con migraciones versionadas (FR-001-03).

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::{AppError, AppResult};

/// Migraciones en orden. Nunca editar una ya publicada: añadir otra al final.
fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // 0001 · ajustes (001)
        M::up("CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);"),
    ])
}

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        migrations().to_latest(&mut conn).map_err(|e| {
            tracing::error!(error = %e, "migración fallida");
            AppError::new(
                crate::error::ErrorCode::Storage,
                "No se pudo actualizar la base de datos local.",
            )
        })?;
        Ok(Self(Mutex::new(conn)))
    }

    #[cfg(test)]
    pub fn open_in_memory() -> AppResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        migrations().to_latest(&mut conn).expect("migraciones");
        Ok(Self(Mutex::new(conn)))
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migraciones_validas() {
        migrations().validate().unwrap();
    }
}
