use crate::api::errors::ApiError;
use crate::api::models::FetchLeaderboardResponse;
use crate::api::AppState;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;

pub async fn fetch_bot_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let bot_id = id.into();

    let res = app_state
        .arena_handle
        .fetch_leaderboard(bot_id)
        .await;

    let Some(res) = res else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(FetchLeaderboardResponse::from(res)))
}
