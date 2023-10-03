use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use sea_orm::DbErr;
use serde_json::json;
use validator::ValidationErrors;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Not found")]
    NotFound,

    #[error("Validation failed: {0}")]
    ValidationFailed(#[from] ValidationErrors),

    #[error("Db Error: {0}")]
    DbError(#[from] DbErr),

    #[error("Already exists")]
    AlreadyExists,

    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}

impl Error {
    fn get_code(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::ValidationFailed(_) => StatusCode::BAD_REQUEST,
            Error::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::AlreadyExists => StatusCode::CONFLICT,
            Error::IOError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status_code = self.get_code();
        let body = Json(json!({
            "message": self.to_string(),
        }));

        (status_code, body).into_response()
    }
}
