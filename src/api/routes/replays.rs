use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::{
    api::{errors::ApiError, AppState},
    arena_commands::WatchReplayResult,
    domain::MatchId,
};

pub async fn watch_replay(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let match_id: MatchId = id.into();
    let res = app_state.arena_handle.watch_replay(match_id).await?;

    Ok(Json(WatchReplayResponse::from(res)))
}

pub async fn close_replay(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let match_id: MatchId = id.into();
    let _ = app_state.arena_handle.close_replay(match_id).await?;

    Ok(())
}

#[derive(Serialize)]
pub struct WatchReplayResponse {
    pub viewer_url: String,
}

impl From<WatchReplayResult> for WatchReplayResponse {
    fn from(value: WatchReplayResult) -> Self {
        Self {
            viewer_url: value.viewer_url,
        }
    }
}
