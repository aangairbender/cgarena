use axum::{
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    api::{errors::ApiError, AppState},
    domain::MatchId,
};

pub async fn watch_replay(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<WatchReplayResponse>, ApiError> {
    let replay = app_state
        .replay_viewer()
        .await?
        .watch(MatchId::from(id))
        .await?;
    Ok(Json(WatchReplayResponse {
        session_id: replay.session_id,
        viewer_url: replay.viewer_url,
    }))
}

pub async fn close_replay(
    State(app_state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<(), ApiError> {
    app_state.replay_viewer().await?.close(&session_id).await?;
    Ok(())
}

pub async fn replay_asset(
    State(app_state): State<AppState>,
    Path((session_id, path)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let asset = app_state
        .replay_viewer()
        .await?
        .asset(&session_id, &path)
        .await?;
    Ok(([(header::CONTENT_TYPE, asset.content_type)], asset.bytes).into_response())
}

#[derive(Serialize)]
pub struct WatchReplayResponse {
    pub session_id: String,
    pub viewer_url: String,
}
