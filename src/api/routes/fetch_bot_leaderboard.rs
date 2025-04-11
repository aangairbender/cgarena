use crate::api::errors::ApiError;
use crate::api::models::FetchLeaderboardResponse;
use crate::api::AppState;
use crate::arena::{ArenaCommand, FetchLeaderboardCommand};
use anyhow::anyhow;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use tokio::sync::oneshot;

pub async fn fetch_bot_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let (tx, rx) = oneshot::channel();
    let command = FetchLeaderboardCommand {
        bot_id: id.into(),
        response: tx,
    };

    app_state
        .arena_tx
        .send(ArenaCommand::FetchLeaderboard(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    let Some(res) = res else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(FetchLeaderboardResponse::from(res)))
}
