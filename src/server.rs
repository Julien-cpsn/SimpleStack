use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use sqlx::SqlitePool;

/// Shared application state, cloned cheaply into every handler/middleware.
#[derive(Clone)]
pub struct ServerState {
    pub db: SqlitePool,
    /// Set the `Secure` flag on the session cookie (enable when served over HTTPS).
    pub cookie_secure: bool,
}

#[derive(Debug)]
pub enum ServerError {
    Unauthorized,
    Forbidden,
    BadRequest(String),
    Conflict(String),
    Internal(anyhow::Error),
}

impl ServerState {
    pub fn new(db: SqlitePool, cookie_secure: bool) -> Self {
        Self {
            db,
            cookie_secure,
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ServerError::Unauthorized => (StatusCode::UNAUTHORIZED, "authentication required".to_string()),
            ServerError::Forbidden => (StatusCode::FORBIDDEN, "insufficient permissions".to_string()),
            ServerError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ServerError::Conflict(m) => (StatusCode::CONFLICT, m),
            ServerError::Internal(e) => {
                tracing::error!("internal error: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for ServerError {
    fn from(e: E) -> Self {
        ServerError::Internal(e.into())
    }
}