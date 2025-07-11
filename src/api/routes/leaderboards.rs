use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    api::{errors::ApiError, AppState},
    domain::{LeaderboardId, LeaderboardName, MatchFilter},
};

#[derive(Deserialize)]
pub struct CreateLeaderboardRequest {
    pub name: String,
    pub filter: String,
}

#[derive(Deserialize)]
pub struct RenameLeaderboardRequest {
    pub new_name: String,
}

pub async fn create_leaderboard(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateLeaderboardRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let name: LeaderboardName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;
    let filter: MatchFilter =
        MatchFilter::parse(&payload.filter).map_err(ApiError::ValidationFailed)?;

    app_state
        .arena_handle
        .create_leaderboard(name, filter)
        .await;

    Ok(())
}

pub async fn rename_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RenameLeaderboardRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id: LeaderboardId = id.into();
    let new_name: LeaderboardName = payload
        .new_name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;

    let res = app_state
        .arena_handle
        .rename_leaderboard(id, new_name)
        .await;

    match res {
        crate::arena::RenameLeaderboardResult::Renamed => Ok(()),
        crate::arena::RenameLeaderboardResult::NotFound => Err(ApiError::NotFound),
    }
}

pub async fn delete_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let id: LeaderboardId = id.into();
    app_state.arena_handle.delete_leaderboard(id).await;
    Ok(())
}
