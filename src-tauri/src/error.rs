//! Contrato de errores: todo comando devuelve `Result<T, AppError>`.
//!
//! `message` está en español, es apto para mostrar al usuario y nunca contiene secretos.

use serde::Serialize;
use specta::Type;

#[allow(dead_code)] // Contrato completo; varios códigos se usan a partir de 002/003.
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
        tracing::error!(error = %e, "error de base de datos");
        Self::new(
            ErrorCode::Storage,
            "No se pudo acceder a la base de datos local.",
        )
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        tracing::error!(error = %e, "error de serialización");
        Self::internal("Datos locales con formato no válido.")
    }
}

pub type AppResult<T> = Result<T, AppError>;
