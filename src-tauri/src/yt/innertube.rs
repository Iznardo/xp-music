//! Transporte a la API interna de YouTube Music (Innertube).
//!
//! ytmapi-rs se usa solo para lo que hace bien: autenticación con cookies,
//! `SAPISIDHASH`, versión de cliente y `X-Goog-AuthUser` (parche en vendor/).
//! Las respuestas se devuelven como JSON crudo y las interpreta `parse.rs`
//! con un parser tolerante (spike/FINDINGS.md §5).

use std::borrow::Cow;

use serde_json::{Map, Value};
use ytmapi_rs::auth::BrowserToken;
use ytmapi_rs::parse::{ParseFrom, ProcessedResult};
use ytmapi_rs::query::{PostMethod, PostQuery, Query};

use crate::error::{AppError, AppResult, ErrorCode};

pub type Yt = ytmapi_rs::YtMusic<BrowserToken>;

/// Consulta POST genérica: `path` (p. ej. `browse`) + cuerpo JSON (sin `context`,
/// que añade ytmapi-rs).
pub struct Raw {
    path: &'static str,
    body: Map<String, Value>,
    params: Vec<(&'static str, String)>,
}

impl Raw {
    pub fn new(path: &'static str, body: Value) -> Self {
        let body = match body {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        Self {
            path,
            body,
            params: Vec::new(),
        }
    }

    pub fn browse(browse_id: &str) -> Self {
        Self::new("browse", serde_json::json!({ "browseId": browse_id }))
    }

    /// Página siguiente. Los tokens llevan el prefijo de `parse::list_continuation`:
    /// `c:` va en el cuerpo (formato moderno) y `n:` como parámetros `ctoken` (antiguo).
    pub fn continuation(path: &'static str, token: &str) -> Self {
        match token.split_once(':') {
            Some(("n", t)) => Self {
                path,
                body: Map::new(),
                params: vec![
                    ("ctoken", t.to_owned()),
                    ("continuation", t.to_owned()),
                    ("type", "next".to_owned()),
                ],
            },
            Some(("c", t)) => Self::new(path, serde_json::json!({ "continuation": t })),
            _ => Self::new(path, serde_json::json!({ "continuation": token })),
        }
    }
}

/// Salida: el JSON tal cual.
#[derive(Debug)]
pub struct RawJson(pub Value);

impl ParseFrom<Raw> for RawJson {
    fn parse_from(p: ProcessedResult<Raw>) -> ytmapi_rs::Result<Self> {
        let v = serde_json::from_str(&p.source)
            .map_err(|e| ytmapi_rs::Error::from(std::io::Error::from(e)))?;
        Ok(RawJson(v))
    }
}

impl Query<BrowserToken> for Raw {
    type Output = RawJson;
    type Method = PostMethod;
}

impl PostQuery for Raw {
    fn header(&self) -> Map<String, Value> {
        self.body.clone()
    }

    fn params(&self) -> Vec<(&str, Cow<'_, str>)> {
        self.params
            .iter()
            .map(|(k, v)| (*k, Cow::from(v.as_str())))
            .collect()
    }

    fn path(&self) -> &str {
        self.path
    }
}

/// Ejecuta una consulta cruda y traduce los errores al contrato de la app.
pub async fn run(yt: &Yt, q: Raw) -> AppResult<Value> {
    let path = q.path;
    match yt.query(q).await {
        Ok(RawJson(v)) => Ok(v),
        Err(e) => Err(map_error(path, &e)),
    }
}

/// Clasifica un error de ytmapi-rs sin filtrar detalles (pueden incluir cabeceras).
pub fn map_error(what: &str, e: &ytmapi_rs::Error) -> AppError {
    let text = e.to_string();
    tracing::warn!(endpoint = what, error = %crate::logging::redact(&text), "error de YouTube Music");
    let code = |c: &str| {
        text.contains(&format!("code {c}"))
            || text.contains(&format!("{c} recieved"))
            || text.contains(&format!("{c} received"))
    };
    if code("401") || code("403") {
        AppError::new(
            ErrorCode::AuthExpired,
            tr!(
                "The YouTube Music session has expired. Import it again.",
                "La sesión de YouTube Music ha caducado. Vuelve a importarla."
            ),
        )
    } else if code("429") {
        AppError::new(
            ErrorCode::RateLimited,
            tr!(
                "YouTube Music asks you to wait a moment before continuing.",
                "YouTube Music pide esperar un poco antes de seguir."
            ),
        )
    } else if code("404") {
        AppError::new(
            ErrorCode::NotFound,
            tr!(
                "Not found on YouTube Music.",
                "No se encontró en YouTube Music."
            ),
        )
    } else if text.to_ascii_lowercase().contains("error sending request")
        || text.contains("timed out")
    {
        AppError::new(
            ErrorCode::Network,
            tr!(
                "Cannot reach YouTube Music.",
                "No hay conexión con YouTube Music."
            ),
        )
    } else {
        AppError::new(
            ErrorCode::Internal,
            tr!(
                "YouTube Music returned an unexpected response.",
                "YouTube Music respondió algo inesperado."
            ),
        )
    }
}
