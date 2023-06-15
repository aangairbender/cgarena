use std::sync::Arc;

use axum::{extract::{Path, State}, Json};
use reqwest::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use super::ArenaService;

#[derive(Deserialize)]
pub struct BotAddReq {
    name: String,
    source_code: String,
    language_name: String,
}

#[axum_macros::debug_handler]
pub async fn add(
    State(arena): State<Arc<ArenaService>>,
    Json(payload): Json<BotAddReq>,
) -> StatusCode {
    let BotAddReq{ name, source_code, language_name } = payload;
    arena.add_bot(name, source_code, language_name).await;
    StatusCode::OK
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
