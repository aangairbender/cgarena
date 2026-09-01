use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Not found")]
    NotFound,

    #[error("Validation failed: {0}")]
    ValidationFailed(anyhow::Error),

    #[error("Conflict: {0}")]
    Conflict(anyhow::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error(transparent)]
    Replay(#[from] crate::replay_viewer::ReplayError),
}

impl ApiError {
    fn get_status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Replay(error) => match error {
                crate::replay_viewer::ReplayError::MatchNotFound
                | crate::replay_viewer::ReplayError::SessionNotFound
                | crate::replay_viewer::ReplayError::AssetNotFound => StatusCode::NOT_FOUND,
                crate::replay_viewer::ReplayError::Unavailable => StatusCode::CONFLICT,
                crate::replay_viewer::ReplayError::InvalidArtifact(_) => {
                    StatusCode::UNPROCESSABLE_ENTITY
                }
                crate::replay_viewer::ReplayError::StartupFailed(_) => StatusCode::BAD_GATEWAY,
                crate::replay_viewer::ReplayError::StartupTimeout => StatusCode::GATEWAY_TIMEOUT,
                crate::replay_viewer::ReplayError::InvalidCommand(_)
                | crate::replay_viewer::ReplayError::Internal(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },
        }
    }

    fn get_error_code(&self) -> &'static str {
        match self {
            ApiError::NotFound => "not_found",
            ApiError::ValidationFailed(_) => "validation_failed",
            ApiError::Conflict(_) => "already_exists",
            ApiError::Internal(_) => "internal_error",
            ApiError::Replay(error) => match error {
                crate::replay_viewer::ReplayError::MatchNotFound
                | crate::replay_viewer::ReplayError::SessionNotFound
                | crate::replay_viewer::ReplayError::AssetNotFound => "not_found",
                crate::replay_viewer::ReplayError::Unavailable => "replay_unavailable",
                crate::replay_viewer::ReplayError::InvalidArtifact(_) => "invalid_replay",
                crate::replay_viewer::ReplayError::StartupFailed(_) => "replay_start_failed",
                crate::replay_viewer::ReplayError::StartupTimeout => "replay_start_timeout",
                crate::replay_viewer::ReplayError::InvalidCommand(_)
                | crate::replay_viewer::ReplayError::Internal(_) => "internal_error",
            },
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

#[derive(Serialize)]
struct ErrorResponse {
    error_code: &'static str,
    message: String,
}
