use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::Language;
use crate::server::services::bot_service::BotService;

#[derive(Deserialize)]
pub struct BotAddReq {
    name: String,
    source_code: String,
    language: Language,
}

pub async fn add(
    State(bot_service): State<Arc<BotService>>,
    Json(payload): Json<BotAddReq>,
) -> StatusCode {
    let BotAddReq {
        name,
        source_code,
        language,
    } = payload;
    match bot_service.add_bot(name, source_code, language).await {
        Ok(_) => StatusCode::OK,
        Err(e) => match e {
            crate::server::services::bot_service::AddBotError::IO(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            crate::server::services::bot_service::AddBotError::DB(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        },
    }
}

pub async fn list() -> StatusCode {
    todo!()
}

#[axum_macros::debug_handler]
pub async fn remove(
    State(bot_service): State<Arc<BotService>>,
    Path(id): Path<Uuid>,
) -> StatusCode {
    match bot_service.remove_bot(id).await {
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
