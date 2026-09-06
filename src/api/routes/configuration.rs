use axum::{extract::State, Json};
use serde::Serialize;

use crate::{
    api::{errors::ApiError, AppState},
    config::ArenaConfig,
    db,
};

#[derive(Serialize)]
pub struct ConfigurationState {
    pub active: Option<ArenaConfig>,
    pub runtime_available: bool,
    pub runtime_error: Option<String>,
}

pub async fn fetch_configuration(
    State(app_state): State<AppState>,
) -> Result<Json<ConfigurationState>, ApiError> {
    let active = db::fetch_arena_config(&app_state.pool).await?;
    Ok(Json(ConfigurationState {
        active,
        runtime_available: app_state.runtime_available().await,
        runtime_error: app_state.runtime.last_error().await,
    }))
}

pub async fn apply_configuration(
    State(app_state): State<AppState>,
    Json(candidate): Json<ArenaConfig>,
) -> Result<Json<ConfigurationState>, ApiError> {
    candidate.validate().map_err(ApiError::ValidationFailed)?;
    app_state
        .runtime
        .apply(candidate)
        .await
        .map_err(ApiError::ValidationFailed)?;
    let active = db::fetch_arena_config(&app_state.pool).await?;
    Ok(Json(ConfigurationState {
        active,
        runtime_available: app_state.runtime_available().await,
        runtime_error: app_state.runtime.last_error().await,
    }))
}
