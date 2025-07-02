use crate::api::errors::ApiError;
use crate::api::models::{BotMinimalResponse, CreateBotRequest};
use crate::api::AppState;
use crate::arena::{CreateBotResult};
use crate::domain::{BotName, Language, SourceCode};
use anyhow::anyhow;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;

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
        .await;

    match res {
        CreateBotResult::Created(bot_minimal) => Ok(Json(BotMinimalResponse::from(bot_minimal))),
        CreateBotResult::DuplicateName => Err(ApiError::Conflict(anyhow!(
            "Bot with the same name already exists"
        ))),
    }
}
