use axum::body::Body;
use axum::extract::Path;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::server::entities::bot;
use crate::server::enums::Language;
use crate::server::errors::Error;
use crate::server::utils::custom_response::{CustomResponse, CustomResponseResult};
use crate::server::AppState;

pub fn create_route() -> Router<AppState, Body> {
    Router::new()
        .route("/bots", post(create_bot))
        .route("/bots", get(query_bots))
        .route("/bots/:id", delete(remove_bot_by_id))
}

fn bots_dir(arena_path: &std::path::Path) -> std::path::PathBuf {
    arena_path.join("bots")
}

pub async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> CustomResponseResult<()> {
    payload.validate()?;

    let duplicate = bot::Entity::find()
        .filter(bot::Column::Name.eq(&payload.name))
        .one(&app_state.db)
        .await?;

    if duplicate.is_some() {
        return Err(Error::AlreadyExists);
    }

    let source_filename = format!("{}.{}", payload.name, payload.language.file_extension());
    let source_file = bots_dir(&app_state.arena_path).join(&source_filename);
    std::fs::write(&source_file, payload.source_code)?;

    let bot = bot::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        name: Set(payload.name),
        source_filename: Set(source_filename),
        language: Set(payload.language),
    };

    bot::Entity::insert(bot)
        .exec_without_returning(&app_state.db)
        .await?;
    Ok(CustomResponse::default())
}

pub async fn query_bots(
    State(app_state): State<AppState>,
) -> CustomResponseResult<ListBotsResponse> {
    let bots = bot::Entity::find().all(&app_state.db).await?;
    Ok(CustomResponse::default().body(ListBotsResponse { bots }))
}

pub async fn remove_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> CustomResponseResult<()> {
    let Some(bot) = bot::Entity::find_by_id(id).one(&app_state.db).await? else {
        return Err(Error::NotFound)
    };
    let source_file_name = format!("{}.{}", bot.name, bot.language.file_extension());
    let source_file = bots_dir(&app_state.arena_path).join(source_file_name);
    std::fs::remove_file(source_file)?;
    bot.delete(&app_state.db).await?;
    Ok(CustomResponse::default())
}

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
    pub source_code: String,
    pub language: Language,
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
