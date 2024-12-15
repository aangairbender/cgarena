use crate::api::errors::ApiError;
use crate::api::AppState;
use crate::app::use_cases;
use crate::domain::{Bot, BotId, Build, BuildStatus};
use anyhow::anyhow;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    extract::State,
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/bots", get(fetch_bots))
        .route("/bots", post(create_bot))
        .route("/bots/:id", patch(patch_bot))
        .route("/bots/:id", delete(delete_bot))
        .route("/bots/:id", get(fetch_bot))
        .route("/bots/:id/builds", get(fetch_bot_builds))
}

#[derive(Serialize, Deserialize)]
struct CreateBotRequest {
    pub name: String,
    pub source_code: String,
    pub language: String,
}

#[derive(Serialize, Deserialize)]
struct PatchBotRequest {
    pub name: String,
}

#[derive(Serialize)]
struct BotResponse {
    pub id: i64,
    pub name: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
    pub stats: BotStats,
}

impl From<Bot> for BotResponse {
    fn from(bot: Bot) -> Self {
        let stats = BotStats::from(&bot);
        Self {
            id: bot.id.into(),
            name: bot.name.into(),
            language: bot.language.into(),
            created_at: bot.created_at,
            stats,
        }
    }
}

#[derive(Serialize)]
struct BotStats {
    pub matches_played: u64,
    pub rating_mu: f64,
    pub rating_sigma: f64,
}

impl From<&Bot> for BotStats {
    fn from(bot: &Bot) -> Self {
        BotStats {
            matches_played: bot.matches_played,
            rating_mu: bot.rating.mu,
            rating_sigma: bot.rating.sigma,
        }
    }
}

#[derive(Serialize)]
struct BuildResponse {
    worker_name: String,
    status: String,
    error: Option<String>,
}

impl From<Build> for BuildResponse {
    fn from(build: Build) -> Self {
        Self {
            worker_name: build.worker_name.into(),
            status: match &build.status {
                BuildStatus::Pending => "pending".to_string(),
                BuildStatus::Running => "running".to_string(),
                BuildStatus::Success => "success".to_string(),
                BuildStatus::Failure(_) => "failure".to_string(),
            },
            error: if let BuildStatus::Failure(err) = build.status {
                Some(err)
            } else {
                None
            },
        }
    }
}

async fn fetch_bots(State(app_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let bots = app_state
        .db
        .fetch_bots()
        .await
        .into_iter()
        .map(BotResponse::from)
        .collect_vec();

    Ok(Json(bots))
}

impl TryFrom<CreateBotRequest> for use_cases::create_bot::Input {
    type Error = anyhow::Error;

    fn try_from(req: CreateBotRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            name: req.name.try_into()?,
            source_code: req.source_code.try_into()?,
            language: req.language.try_into()?,
        })
    }
}

async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use use_cases::create_bot::*;

    let input = payload
        .try_into()
        .map_err(|err| ApiError::ValidationFailed(err))?;

    let bot_id = match execute(input, app_state.db.clone(), app_state.wm.clone()).await {
        Output::Created(id) => id,
        Output::AlreadyExists => return Err(ApiError::AlreadyExists),
    };

    let bot = app_state
        .db
        .fetch_bot(bot_id)
        .await
        .ok_or(anyhow!("Failed to fetch bot after creation"))?;

    Ok(Json(BotResponse::from(bot)))
}

impl TryFrom<(i64, PatchBotRequest)> for use_cases::rename_bot::Input {
    type Error = anyhow::Error;

    fn try_from((id, req): (i64, PatchBotRequest)) -> Result<Self, Self::Error> {
        Ok(Self {
            bot_id: id.into(),
            new_name: req.name.try_into()?,
        })
    }
}

async fn patch_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<PatchBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    use use_cases::rename_bot::*;

    let input = Input::try_from((id, payload)).map_err(|err| ApiError::ValidationFailed(err))?;

    match execute(input, app_state.db.clone()).await {
        Output::Ok => Ok(StatusCode::OK),
        Output::NotFound => Err(ApiError::NotFound),
        Output::AlreadyExists => Err(ApiError::AlreadyExists),
    }
}

async fn delete_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    use use_cases::delete_bot::*;
    let bot_id: BotId = id.into();
    match execute(bot_id, app_state.db).await {
        Output::Deleted => Ok(StatusCode::OK),
        Output::NotFound => Err(ApiError::NotFound),
    }
}

async fn fetch_bot_builds(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let builds = app_state
        .db
        .fetch_builds(id.into())
        .await
        .into_iter()
        .map(BuildResponse::from)
        .collect_vec();
    Ok(Json(builds))
}

async fn fetch_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    match app_state.db.fetch_bot(id.into()).await {
        Some(bot) => Ok(Json(BotResponse::from(bot))),
        None => Err(ApiError::NotFound),
    }
}

// impl From<DBError> for ApiError {
//     fn from(value: DBError) -> Self {
//         match value {
//             DBError::AlreadyExists => ApiError::AlreadyExists,
//             DBError::NotFound => ApiError::NotFound,
//             DBError::Unexpected(e) => ApiError::Internal(e),
//         }
//     }
// }
