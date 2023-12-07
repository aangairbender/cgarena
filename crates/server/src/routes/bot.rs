use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::patch;
use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use entity::bot;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::errors::AppError;
use crate::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/bots", post(create_bot))
        .route("/bots", get(query_bots))
        .route("/bots/:id", get(get_bot_by_id))
        .route("/bots/:id", patch(patch_bot_by_id))
        .route("/bots/:id", delete(delete_bot_by_id))
}

async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate()?;

    let duplicate = bot::Entity::find()
        .filter(bot::Column::Name.eq(&payload.name))
        .one(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    if duplicate.is_some() {
        return Err(AppError::AlreadyExists);
    }

    let bot = bot::ActiveModel {
        id: Set(uuid::Uuid::new_v4()),
        name: Set(payload.name),
        source_code: Set(payload.source_code),
        language: Set(payload.language),
        created_at: Set(Utc::now()),
        deleted: NotSet, // intentionally leaving it null
    };

    let bot = bot::Entity::insert(bot)
        .exec_with_returning(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "bot": bot,
    });

    Ok((StatusCode::CREATED, Json(response_body)))
}

async fn query_bots(State(app_state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let bots = bot::Entity::find()
        .all(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "bots": bots,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn get_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let bot = bot::Entity::find_by_id(id)
        .one(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let Some(bot) = bot else {
        return Err(AppError::NotFound);
    };

    let response_body = json!({
        "bot": bot,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn patch_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<PatchBotRequest>,
) -> Result<impl IntoResponse, AppError> {
    let bot = bot::Entity::find_by_id(id)
        .one(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let Some(bot) = bot else {
        return Err(AppError::NotFound);
    };

    let mut bot: bot::ActiveModel = bot.into();
    bot.name = Set(payload.name);
    let bot = bot
        .update(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "bot": bot,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn delete_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let Some(bot) = bot::Entity::find_by_id(id)
        .one(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?
    else {
        return Err(AppError::NotFound);
    };
    bot.delete(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(StatusCode::OK)
}

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
    #[validate(length(max = 100000))]
    pub source_code: String,
    #[validate(length(min = 1, max = 32))]
    pub language: String,
}

#[derive(Serialize, Deserialize, Validate)]
pub struct PatchBotRequest {
    #[validate(length(min = 1, max = 32))]
    pub name: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error_code: &'static str,
    description: String,
}
