// Reserved: structured error mapping for handler edges. Kept intentionally even
// where not yet wired in, so new fallible handlers have a ready `?` target.
#![allow(dead_code)]

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Error type for the HTTP boundary. Internal/integration code uses
/// `anyhow::Result`; we convert into this only at handler edges.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            other => {
                tracing::error!(error = %other, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
