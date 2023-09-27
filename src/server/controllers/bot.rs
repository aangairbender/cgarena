use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::server::{enums::Language, AppState};

#[derive(Deserialize)]
pub struct BotAddReq {
    name: String,
    source_code: String,
    language: Language,
}

pub async fn add(
    State(app_state): State<AppState>,
    Json(payload): Json<BotAddReq>,
) -> StatusCode {
    log::info!("bot add request received");
    let BotAddReq {
        name,
        source_code,
        language,
    } = payload;
    match app_state.bot_service.add_bot(name, source_code, language).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            log::error!("{}", e);
            match e {
                crate::server::services::bot_service::AddBotError::IO(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
                crate::server::services::bot_service::AddBotError::DB(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            }
        },
    }
}

pub async fn list() -> StatusCode {
    todo!()
}

#[axum_macros::debug_handler]
pub async fn remove(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    match app_state.bot_service.remove_bot(id).await {
        Ok(_) => StatusCode::OK,
        Err(e) => match e {
            crate::server::services::bot_service::RemoveBotError::NotFound => StatusCode::NOT_FOUND,
            crate::server::services::bot_service::RemoveBotError::IO(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            crate::server::services::bot_service::RemoveBotError::DB(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        },
    }
}

pub async fn patch(Path(_id): Path<Uuid>) -> StatusCode {
    todo!()
}
