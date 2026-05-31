use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use tracing::{error, warn};
use utoipa::ToSchema;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Internal error")]
    Internal,
    #[error("Not implemented")]
    NotImplemented,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(msg) => {
                warn!(status = ?StatusCode::BAD_REQUEST, "Bad request");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            AppError::NotFound(msg) => {
                warn!(status = ?StatusCode::NOT_FOUND, "Resource not found");
                (StatusCode::NOT_FOUND, msg.clone())
            }
            AppError::Unauthorized(msg) => {
                warn!(status = ?StatusCode::UNAUTHORIZED, "Unauthorized");
                (StatusCode::UNAUTHORIZED, msg.clone())
            }
            AppError::Forbidden(msg) => {
                warn!(status = ?StatusCode::FORBIDDEN, "Forbidden");
                (StatusCode::FORBIDDEN, msg.clone())
            }
            AppError::Internal => {
                error!("Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal error".to_string(),
                )
            }
            AppError::NotImplemented => {
                warn!(status = ?StatusCode::NOT_IMPLEMENTED, "Not implemented");
                (StatusCode::NOT_IMPLEMENTED, "Not implemented".to_string())
            }
        };

        let body = ErrorResponse { message };

        (status, Json(body)).into_response()
    }
}
