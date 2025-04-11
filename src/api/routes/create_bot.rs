use crate::api::errors::ApiError;
use crate::api::models::{BotMinimalResponse, CreateBotRequest};
use crate::api::AppState;
use crate::arena::{ArenaCommand, CreateBotCommand, CreateBotResult};
use crate::domain::{BotName, Language, SourceCode};
use anyhow::anyhow;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use tokio::sync::oneshot;

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
