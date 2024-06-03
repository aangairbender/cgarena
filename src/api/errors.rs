use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use validator::ValidationErrors;

use crate::server::ArenaError;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,

    #[error("Validation failed: {0}")]
    ValidationFailed(#[from] ValidationErrors),

    #[error("Already exists")]
    AlreadyExists,

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    fn get_status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::AlreadyExists => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn get_error_code(&self) -> &'static str {
        match self {
            ApiError::NotFound => "not_found",
            ApiError::ValidationFailed(_) => "validation_failed",
            ApiError::AlreadyExists => "already_exists",
            ApiError::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status_code = self.get_status_code();
        let error_code = self.get_error_code();
        let body = ErrorResponse {
            error_code,
            message: self.to_string(),
        };
        (status_code, Json(body)).into_response()
    }
}

impl From<ArenaError> for ApiError {
    fn from(value: ArenaError) -> Self {
        match value {
            ArenaError::AlreadyExists => ApiError::AlreadyExists,
            ArenaError::NotFound => ApiError::NotFound,
        }
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error_code: &'static str,
    message: String,
}
