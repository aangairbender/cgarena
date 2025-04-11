use crate::api::errors::ApiError;
use crate::api::models::BotMinimalResponse;
use crate::api::AppState;
use crate::arena::{ArenaCommand, FetchBotsCommand};
use anyhow::anyhow;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use itertools::Itertools;
use tokio::sync::oneshot;

pub async fn fetch_bots(State(app_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let (tx, rx) = oneshot::channel();
    let command = FetchBotsCommand { response: tx };

    app_state
        .arena_tx
        .send(ArenaCommand::FetchBots(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    Ok(Json(
        res.into_iter().map(BotMinimalResponse::from).collect_vec(),
    ))
}
