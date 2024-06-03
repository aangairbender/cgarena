use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::patch;
use axum::{
    extract::State,
    routing::{delete, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::api::errors::ApiError;
use crate::api::AppState;


pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/bots", post(create_bot))
        .route("/bots/:name", patch(patch_bot_by_name))
        .route("/bots/:name", delete(delete_bot_by_name))
}

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
    #[validate(length(max = 100000))]
    pub source_code: String,
    #[validate(length(min = 1, max = 32))]
    pub language: String,
}

async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate()?;

    let mut arena = app_state.arena.lock().await;
    arena.add_bot(payload.name, payload.source_code, payload.language)?;

    Ok(StatusCode::CREATED)
}

#[derive(Serialize, Deserialize, Validate)]
pub struct PatchBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
}


async fn patch_bot_by_name(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<PatchBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate()?;

    let mut arena = app_state.arena.lock().await;
    arena.rename_bot(&name, payload.name)?;

    Ok(StatusCode::OK)
}

async fn delete_bot_by_name(
    State(app_state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let mut arena = app_state.arena.lock().await;
    arena.remove_bot(&name)?;
    Ok(StatusCode::OK)
}
