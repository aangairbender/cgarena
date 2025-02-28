use crate::api::errors::ApiError;
use crate::api::AppState;
use crate::arena::{
    ArenaCommand, BotMinimal, CreateBotCommand, CreateBotResult, DeleteBotCommand,
    FetchBotsCommand, FetchLeaderboardCommand, FetchLeaderboardResult, LeaderboardBotOverview,
    LeaderboardItem, RenameBotCommand, RenameBotResult,
};
use crate::domain::{BotId, BotName, Build, BuildResult, BuildStatus, Language, SourceCode};
use anyhow::anyhow;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::patch;
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
        .route("/bots/:id", patch(rename_bot))
}

#[derive(Deserialize)]
struct CreateBotRequest {
    pub name: String,
    pub source_code: String,
    pub language: String,
}

#[derive(Deserialize)]
struct RenameBotRequest {
    pub name: String,
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
    pub builds: Vec<BuildResponse>,
}

#[derive(Serialize)]
struct BuildResponse {
    pub worker_name: String,
    pub status: String,
    pub stderr: Option<String>,
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
            builds: v.builds.into_iter().map(|b| b.into()).collect(),
        }
    }
}

impl From<Build> for BuildResponse {
    fn from(b: Build) -> Self {
        let (status, stderr) = match b.status {
            BuildStatus::Pending => ("pending".to_string(), None),
            BuildStatus::Running => ("running".to_string(), None),
            BuildStatus::Finished(BuildResult::Success) => ("finished".to_string(), None),
            BuildStatus::Finished(BuildResult::Failure { stderr }) => {
                ("finished".to_string(), Some(stderr))
            }
        };
        BuildResponse {
            worker_name: b.worker_name.into(),
            status,
            stderr,
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
        CreateBotResult::Created(bot_minimal) => Ok(Json(BotMinimalResponse::from(bot_minimal))),
        CreateBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
    }
}

async fn rename_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RenameBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id: BotId = id.into();
    let new_name: BotName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;

    let (tx, rx) = oneshot::channel();
    let command = RenameBotCommand {
        id,
        new_name,
        response: tx,
    };

    app_state
        .arena_tx
        .send(ArenaCommand::RenameBot(command))
        .await
        .map_err(|e| anyhow!(e))?;

    let res = rx.await.map_err(|e| anyhow!(e))?;

    match res {
        RenameBotResult::Renamed(bot_minimal) => Ok(Json(BotMinimalResponse::from(bot_minimal))),
        RenameBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
        RenameBotResult::NotFound => Err(ApiError::NotFound),
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
