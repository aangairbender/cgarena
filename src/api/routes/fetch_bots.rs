use crate::api::errors::ApiError;
use crate::api::models::BotMinimalResponse;
use crate::api::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use itertools::Itertools;

pub async fn fetch_bots(State(app_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let res = app_state
        .arena_handle
        .fetch_all_bots()
        .await;

    Ok(Json(
        res.into_iter().map(BotMinimalResponse::from).collect_vec(),
    ))
}
