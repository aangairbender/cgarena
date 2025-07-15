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
    arena_commands::{BotSourceCode, CreateBotResult, RenameBotResult},
    domain::{BotId, BotName, Language, SourceCode},
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
        .arena_handle
        .create_bot(name, source_code, language)
        .await?;

    match res {
        CreateBotResult::Created(bot_overview) => Ok(Json(BotOverviewResponse::from(bot_overview))),
        CreateBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
    }
}

pub async fn delete_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let bot_id: BotId = id.into();

    app_state.arena_handle.delete_bot(bot_id).await?;

    Ok(StatusCode::OK)
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

    let res = app_state.arena_handle.rename_bot(id, new_name).await?;

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

    let res = app_state.arena_handle.fetch_bot_source_code(id).await?;

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
