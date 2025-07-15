use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    api::{errors::ApiError, models::LeaderboardOverviewResponse, AppState},
    arena_commands::PatchLeaderboardResult,
    domain::{LeaderboardId, LeaderboardName, MatchFilter},
};

#[derive(Deserialize)]
pub struct CreateLeaderboardRequest {
    pub name: String,
    pub filter: String,
}

#[derive(Deserialize)]
pub struct PatchLeaderboardRequest {
    pub name: String,
    pub filter: String,
}

pub async fn create_leaderboard(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateLeaderboardRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let name: LeaderboardName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;
    let filter: MatchFilter = payload.filter.parse().map_err(ApiError::ValidationFailed)?;

    let res = app_state
        .arena_handle
        .create_leaderboard(name, filter)
        .await?;

    Ok(Json(LeaderboardOverviewResponse::from(res)))
}

pub async fn patch_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<PatchLeaderboardRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id: LeaderboardId = id.into();
    let name: LeaderboardName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;
    let filter: MatchFilter = payload.filter.parse().map_err(ApiError::ValidationFailed)?;

    let res = app_state
        .arena_handle
        .patch_leaderboard(id, name, filter)
        .await?;

    match res {
        PatchLeaderboardResult::OK => Ok(()),
        PatchLeaderboardResult::NotFound => Err(ApiError::NotFound),
    }
}

pub async fn delete_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let id: LeaderboardId = id.into();
    app_state.arena_handle.delete_leaderboard(id).await?;
    Ok(())
}
