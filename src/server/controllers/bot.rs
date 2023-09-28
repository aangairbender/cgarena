use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::server::{entities::bot, enums::Language, services::bot_service::AddBotError, AppState};

#[derive(Deserialize, Validate)]
pub struct BotAddReq {
    #[validate(length(min = 1, max = 32))]
    name: String,
    source_code: String,
    language: Language,
}

#[derive(Serialize)]
pub struct ListBotsResponse {
    bots: Vec<bot::Model>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error_code: &'static str,
    description: String,
}

pub async fn add(State(app_state): State<AppState>, Json(payload): Json<BotAddReq>) -> Response {
    log::info!("bot add request received");
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error_code: "validation_failed",
                description: e.to_string(),
            }),
        )
            .into_response();
    }
    let BotAddReq {
        name,
        source_code,
        language,
    } = payload;
    match app_state
        .bot_service
        .add_bot(name, source_code, language)
        .await
    {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            log::error!("{}", e);
            match e {
                AddBotError::DuplicateName => (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error_code: "duplicate_name",
                        description: e.to_string(),
                    }),
                )
                    .into_response(),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error_code: "internal_error",
                        description: e.to_string(),
                    }),
                )
                    .into_response(),
            }
        }
    }
}

pub async fn list(State(app_state): State<AppState>) -> Response {
    match app_state.bot_service.list_bots().await {
        Ok(bots) => (StatusCode::OK, Json(ListBotsResponse { bots })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error_code: "internal_error",
                description: e.to_string(),
            }),
        )
            .into_response(),
    }
}

#[axum_macros::debug_handler]
pub async fn remove(State(app_state): State<AppState>, Path(id): Path<Uuid>) -> StatusCode {
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
