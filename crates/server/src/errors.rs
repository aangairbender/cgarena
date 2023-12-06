use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use validator::ValidationErrors;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Not found")]
    NotFound,

    #[error("Validation failed: {0}")]
    ValidationFailed(#[from] ValidationErrors),

    #[error("Already exists")]
    AlreadyExists,

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl Error {
    fn get_status_code(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            Error::AlreadyExists => StatusCode::CONFLICT,
            Error::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn get_error_code(&self) -> &'static str {
        match self {
            Error::NotFound => "not_found",
            Error::ValidationFailed(_) => "validation_failed",
            Error::AlreadyExists => "already_exists",
            Error::Internal(_) => "internal_error",
        }
    }
}

impl IntoResponse for Error {
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
