//! Error contract: every command returns `Result<T, AppError>`.
//!
//! `message` is localized (see `tr!`), fit to show to the user, and never contains secrets.

use serde::Serialize;
use specta::Type;

#[allow(dead_code)] // Full contract; several codes are used from 002/003 onwards.
#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    NotAuthenticated,
    AuthExpired,
    Network,
    RateLimited,
    ResolveFailed,
    StreamExpired,
    Decode,
    AudioDevice,
    NotFound,
    InvalidInput,
    Storage,
    Internal,
}

#[derive(thiserror::Error, Serialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub recoverable: bool,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let recoverable = matches!(
            code,
            ErrorCode::Network
                | ErrorCode::RateLimited
                | ErrorCode::ResolveFailed
                | ErrorCode::StreamExpired
        );
        Self {
            code,
            message: message.into(),
            recoverable,
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        tracing::error!(error = %e, "database error");
        Self::new(
            ErrorCode::Storage,
            tr!(
                "Could not access the local database.",
                "No se pudo acceder a la base de datos local."
            ),
        )
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        tracing::error!(error = %e, "serialization error");
        Self::internal(tr!(
            "Local data has an invalid format.",
            "Datos locales con formato no válido."
        ))
    }
}

pub type AppResult<T> = Result<T, AppError>;
