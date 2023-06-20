use std::sync::Arc;

use axum::{extract::{Path, State}, Json};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::Language;

use super::{ArenaService, services::arena};

#[derive(Deserialize)]
pub struct BotAddReq {
    name: String,
    source_code: String,
    language: Language,
}

pub async fn add(
    State(arena): State<Arc<ArenaService>>,
    Json(payload): Json<BotAddReq>,
) -> StatusCode {
    let BotAddReq{ name, source_code, language } = payload;
    match arena.add_bot(name, source_code, language).await {
        Ok(_) => StatusCode::OK,
        Err(e) => match e {
            arena::Error::InvalidConfig(_) => StatusCode::BAD_REQUEST,
            arena::Error::IO(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn list() -> StatusCode {
    todo!()
}

pub async fn remove(
    Path(id): Path<Uuid>
) -> StatusCode {
    todo!()
}

pub async fn patch(
    Path(id): Path<Uuid>
) -> StatusCode {
    todo!()
}
