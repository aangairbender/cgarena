use crate::api::errors::ApiError;
use crate::api::models::{BotMinimalResponse, RenameBotRequest};
use crate::api::AppState;
use crate::arena::RenameBotResult;
use crate::domain::{BotId, BotName};
use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;

pub async fn rename_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RenameBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id: BotId = id.into();
    let new_name: BotName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;

    let res = app_state.arena_handle.rename_bot(id, new_name).await;

    match res {
        RenameBotResult::Renamed(bot_minimal) => Ok(Json(BotMinimalResponse::from(bot_minimal))),
        RenameBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
        RenameBotResult::NotFound => Err(ApiError::NotFound),
    }
}
