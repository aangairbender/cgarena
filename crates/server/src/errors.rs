use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use validator::ValidationErrors;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Not found")]
    NotFound,

    #[error("Validation failed: {0}")]
    ValidationFailed(#[from] ValidationErrors),

    #[error("Already exists")]
    AlreadyExists,

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn get_status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            AppError::AlreadyExists => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn get_error_code(&self) -> &'static str {
        match self {
            AppError::NotFound => "not_found",
            AppError::ValidationFailed(_) => "validation_failed",
            AppError::AlreadyExists => "already_exists",
            AppError::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for AppError {
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

#[derive(Serialize)]
struct ErrorResponse {
    error_code: &'static str,
    message: String,
}
