use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    extract::State,
    routing::{delete, post, get, patch},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::api::errors::ApiError;
use crate::api::AppState;
use crate::db::DBError;
use crate::model::{Bot, Rating};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/bots", get(fetch_bots))
        .route("/bots", post(create_bot))
        .route("/bots/:id", patch(patch_bot))
        .route("/bots/:id", delete(delete_bot))
}

#[derive(Serialize, Deserialize, Validate)]
struct CreateBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
    #[validate(length(max = 100_000))]
    pub source_code: String,
    #[validate(length(min = 1, max = 32))]
    pub language: String,
}

#[derive(Serialize, Deserialize, Validate)]
struct PatchBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
}

#[derive(Deserialize)]
struct FetchBotsParams {
    name: Option<String>,
}

async fn fetch_bots(
    State(app_state): State<AppState>,
    Query(params): Query<FetchBotsParams>,
) -> Result<impl IntoResponse, ApiError> {
    let mut bots = app_state.db.fetch_bots()
        .await?;

    if let Some(name) = params.name {
        bots.retain(|b| b.name == name);
    }

    Ok(Json(bots))
}

async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate()?;

    let bot = Bot {
        id: 0,
        name: payload.name,
        source_code: payload.source_code,
        language: payload.language,
        created_at: Utc::now(),
    };

    app_state.db
        .add_bot(bot)
        .await?;

    Ok(StatusCode::CREATED)
}

async fn patch_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<PatchBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate()?;

    app_state.db.update_bot(id, payload.name).await?;

    Ok(StatusCode::OK)
}

async fn delete_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    app_state.db.remove_bot(id).await?;
    Ok(StatusCode::OK)
}

impl From<DBError> for ApiError {
    fn from(value: DBError) -> Self {
        match value {
            DBError::AlreadyExists => ApiError::AlreadyExists,
            DBError::NotFound => ApiError::NotFound,
            DBError::Unexpected(e) => ApiError::Internal(e),
        }
    }
}
