use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

use crate::{
    api::{errors::ApiError, AppState},
    managed_referee::{RefereeAction, RefereeStatus},
};

pub async fn fetch_referee_status(
    State(state): State<AppState>,
) -> Result<Json<RefereeStatus>, ApiError> {
    Ok(Json(state.runtime.managed_referee_status().await?))
}

#[derive(Deserialize)]
pub struct RefereeActionRequest {
    pub action: RefereeAction,
}

pub async fn start_referee_action(
    State(state): State<AppState>,
    Json(request): Json<RefereeActionRequest>,
) -> Result<StatusCode, ApiError> {
    state
        .runtime
        .start_managed_referee_action(request.action)
        .await
        .map_err(ApiError::Conflict)?;
    Ok(StatusCode::ACCEPTED)
}
