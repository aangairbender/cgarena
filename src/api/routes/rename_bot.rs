use crate::api::errors::ApiError;
use crate::api::models::{BotMinimalResponse, RenameBotRequest};
use crate::api::AppState;
use crate::arena::{ArenaCommand, RenameBotCommand, RenameBotResult};
use crate::domain::{BotId, BotName};
use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use tokio::sync::oneshot;

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

    let (tx, rx) = oneshot::channel();
    let command = RenameBotCommand {
        id,
        new_name,
        response: tx,
    };

    app_state
        .arena_tx
        .send(ArenaCommand::RenameBot(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    match res {
        RenameBotResult::Renamed(bot_minimal) => Ok(Json(BotMinimalResponse::from(bot_minimal))),
        RenameBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
        RenameBotResult::NotFound => Err(ApiError::NotFound),
    }
}
