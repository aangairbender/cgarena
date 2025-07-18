use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;

use crate::api::{errors::ApiError, AppState};

#[derive(Deserialize)]
pub struct EnableMatchmakingRequest {
    pub enabled: bool,
}

pub async fn enable_matchmaking(
    State(app_state): State<AppState>,
    Json(payload): Json<EnableMatchmakingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let enabled = payload.enabled;

    app_state.arena_handle.enable_matchmaking(enabled).await?;

    Ok(())
}
