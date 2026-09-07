use anyhow::anyhow;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::{
    api::{
        errors::ApiError,
        models::{BotOverviewResponse, CreateBotRequest, RenameBotRequest},
        AppState,
    },
    arena_commands::{
        BotRoleTransition, BotRoleTransitionResult, BotSourceCode, CreateBotResult, RenameBotResult,
    },
    domain::{BotId, BotName, BotRole, Language, SourceCode},
};

pub async fn create_bot(
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

    let res = app_state
        .arena_handle()
        .await?
        .create_bot_with_role(name, source_code, language, payload.role)
        .await?;

    match res {
        CreateBotResult::Created(bot_overview) => Ok(Json(BotOverviewResponse::from(bot_overview))),
        CreateBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
    }
}

pub async fn reject_candidate(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let result = app_state
        .arena_handle()
        .await?
        .reject_candidate(id.into())
        .await?;
    lifecycle_response(result)
}

pub async fn promote_candidate(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let result = app_state
        .arena_handle()
        .await?
        .change_bot_role(id.into(), BotRoleTransition::Promote)
        .await?;
    lifecycle_response(result)
}

pub async fn archive_benchmark(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let result = app_state
        .arena_handle()
        .await?
        .change_bot_role(id.into(), BotRoleTransition::Archive)
        .await?;
    lifecycle_response(result)
}

fn lifecycle_response(result: BotRoleTransitionResult) -> Result<StatusCode, ApiError> {
    match result {
        BotRoleTransitionResult::Changed => Ok(StatusCode::OK),
        BotRoleTransitionResult::NotFound => Err(ApiError::NotFound),
        BotRoleTransitionResult::InvalidState => Err(ApiError::Conflict(anyhow!(
            "Bot lifecycle transition is not valid from the current role"
        ))),
    }
}

pub async fn fetch_archived_bots(
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let bots = app_state
        .arena_handle()
        .await?
        .fetch_status()
        .await?
        .bots
        .into_iter()
        .filter(|bot| bot.role == BotRole::ArchivedBenchmark)
        .map(BotOverviewResponse::from)
        .collect::<Vec<_>>();
    Ok(Json(bots))
}

pub async fn rename_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<RenameBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let id: BotId = id.into();
    let new_name: BotName = payload
        .name
        .try_into()
        .map_err(ApiError::ValidationFailed)?;

    let res = app_state
        .arena_handle()
        .await?
        .rename_bot(id, new_name)
        .await?;

    match res {
        RenameBotResult::Renamed => Ok(()),
        RenameBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
        RenameBotResult::NotFound => Err(ApiError::NotFound),
    }
}

pub async fn fetch_source_code(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let id: BotId = id.into();

    let res = app_state
        .arena_handle()
        .await?
        .fetch_bot_source_code(id)
        .await?;

    match res {
        Some(res) => Ok(Json(BotSourceCodeResponse::from(res))),
        None => Err(ApiError::NotFound),
    }
}

#[derive(Serialize)]
pub struct BotSourceCodeResponse {
    pub language: String,
    pub source_code: String,
}

impl From<BotSourceCode> for BotSourceCodeResponse {
    fn from(value: BotSourceCode) -> Self {
        BotSourceCodeResponse {
            language: value.language.into(),
            source_code: value.source_code.into(),
        }
    }
}
