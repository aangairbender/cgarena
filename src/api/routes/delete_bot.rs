use crate::api::errors::ApiError;
use crate::api::AppState;
use crate::arena::{ArenaCommand, DeleteBotCommand};
use crate::domain::BotId;
use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn delete_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let bot_id: BotId = id.into();

    let command = DeleteBotCommand { id: bot_id };
    app_state
        .arena_tx
        .send(ArenaCommand::DeleteBot(command))
        .await
        .map_err(|e| anyhow!(e))?;
    Ok(StatusCode::OK)
}
