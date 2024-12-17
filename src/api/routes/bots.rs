use crate::api::errors::ApiError;
use crate::api::AppState;
use crate::arena::{
    ArenaCommand, BotMinimal, CreateBotCommand, CreateBotResult, DeleteBotCommand,
    FetchBotsCommand, FetchLeaderboardCommand, FetchLeaderboardResult, LeaderboardBotOverview,
    LeaderboardItem,
};
use crate::domain::{BotId, BotName, Language, SourceCode};
use anyhow::anyhow;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Local};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/bots", post(create_bot))
        .route("/bots", get(fetch_bots))
        .route("/bots/:id", delete(delete_bot))
        .route("/bots/:id", get(fetch_bot_leaderboard))
}

#[derive(Serialize, Deserialize)]
struct CreateBotRequest {
    pub name: String,
    pub source_code: String,
    pub language: String,
}

#[derive(Serialize)]
struct BotMinimalResponse {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize)]
struct FetchLeaderboardResponse {
    pub bot_overview: LeaderboardBotOverviewResponse,
    pub items: Vec<LeaderboardItemResponse>,
}

#[derive(Serialize)]
struct LeaderboardBotOverviewResponse {
    pub id: i64,
    pub name: String,
    pub language: String,
    pub rating_mu: f64,
    pub rating_sigma: f64,
    pub matches_played: usize,
    pub matches_with_error: usize,
}

#[derive(Serialize)]
struct LeaderboardItemResponse {
    pub id: i64,
    pub rank: usize,
    pub name: String,
    pub rating_mu: f64,
    pub rating_sigma: f64,
    pub wins: usize,
    pub loses: usize,
    pub draws: usize,
    pub created_at: String,
}

impl From<BotMinimal> for BotMinimalResponse {
    fn from(value: BotMinimal) -> Self {
        BotMinimalResponse {
            id: value.id.into(),
            name: value.name.into(),
        }
    }
}

impl From<LeaderboardItem> for LeaderboardItemResponse {
    fn from(item: LeaderboardItem) -> Self {
        LeaderboardItemResponse {
            id: item.id.into(),
            rank: item.rank,
            name: item.name.into(),
            rating_mu: item.rating.mu,
            rating_sigma: item.rating.sigma,
            wins: item.wins,
            loses: item.loses,
            draws: item.draws,
            created_at: DateTime::<Local>::from(item.created_at)
                .format("%d/%m/%Y %H:%M")
                .to_string(),
        }
    }
}

impl From<LeaderboardBotOverview> for LeaderboardBotOverviewResponse {
    fn from(v: LeaderboardBotOverview) -> Self {
        LeaderboardBotOverviewResponse {
            id: v.id.into(),
            name: v.name.to_string(),
            language: v.language.to_string(),
            rating_mu: v.rating.mu,
            rating_sigma: v.rating.sigma,
            matches_played: v.matches_played,
            matches_with_error: v.matches_with_error,
        }
    }
}

impl From<FetchLeaderboardResult> for FetchLeaderboardResponse {
    fn from(value: FetchLeaderboardResult) -> Self {
        FetchLeaderboardResponse {
            bot_overview: value.bot_overview.into(),
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let name: BotName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;
    let source_code: SourceCode = payload
        .source_code
        .try_into()
        .map_err(ApiError::ValidationFailed)?;
    let language: Language = payload
        .language
        .try_into()
        .map_err(ApiError::ValidationFailed)?;

    let (tx, rx) = oneshot::channel();

    let command = CreateBotCommand {
        name,
        source_code,
        language,
        response: tx,
    };

    app_state
        .arena_tx
        .send(ArenaCommand::CreateBot(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    match res {
        CreateBotResult::Created(bot_id) => Ok(Json(i64::from(bot_id))),
        CreateBotResult::DuplicateName => Err(ApiError::AlreadyExists),
    }
}

async fn delete_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let bot_id: BotId = id.into();

    let command = DeleteBotCommand { id: bot_id };
    app_state
        .arena_tx
        .send(ArenaCommand::DeleteBot(command))
        .await
        .map_err(|e| anyhow!(e))?;
    Ok(StatusCode::OK)
}

async fn fetch_bot_leaderboard(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let (tx, rx) = oneshot::channel();
    let command = FetchLeaderboardCommand {
        bot_id: id.into(),
        response: tx,
    };

    app_state
        .arena_tx
        .send(ArenaCommand::FetchLeaderboard(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    let Some(res) = res else {
        return Err(ApiError::NotFound);
    };

    Ok(Json(FetchLeaderboardResponse::from(res)))
}

async fn fetch_bots(State(app_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let (tx, rx) = oneshot::channel();
    let command = FetchBotsCommand { response: tx };

    app_state
        .arena_tx
        .send(ArenaCommand::FetchBots(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    Ok(Json(
        res.into_iter().map(BotMinimalResponse::from).collect_vec(),
    ))
}
