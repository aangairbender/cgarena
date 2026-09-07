use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;

use crate::api::{errors::ApiError, AppState};

#[derive(Deserialize)]
pub struct EvaluationSchedulingRequest {
    pub enabled: bool,
}

pub async fn set_evaluation_scheduling(
    State(app_state): State<AppState>,
    Json(payload): Json<EvaluationSchedulingRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let enabled = payload.enabled;

    app_state
        .arena_handle()
        .await?
        .set_evaluation_scheduling(enabled)
        .await?;

    Ok(())
}
