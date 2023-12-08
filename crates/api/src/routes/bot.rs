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
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, Set, ConnectionTrait, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use crate::errors::ApiError;
use crate::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/bots", post(create_bot))
        .route("/bots", get(query_bots))
        .route("/bots/:id", get(get_bot_by_id))
        .route("/bots/:id", patch(patch_bot_by_id))
        .route("/bots/:id", delete(delete_bot_by_id))
}

async fn ensure_name_is_unique(c: &impl ConnectionTrait, name: &str) -> Result<(), ApiError> {
    let duplicate = bot::Entity::find()
        .filter(bot::Column::Name.eq(name))
        .one(c)
        .await?;

    if duplicate.is_some() {
        return Err(ApiError::AlreadyExists);
    }

    Ok(())
}

async fn create_bot(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate()?;

    let txn = app_state.db.begin().await?;

    ensure_name_is_unique(&txn, &payload.name).await?;

    let bot = bot::ActiveModel {
        id: NotSet,
        name: Set(payload.name),
        source_code: Set(payload.source_code),
        language: Set(payload.language),
        created_at: Set(Utc::now()),
        deleted: Set(false),
    };

    let bot = bot::Entity::insert(bot.into_active_model())
        .exec_with_returning(&txn)
        .await?;

    txn.commit().await?;

    let response_body = json!({
        "bot": &bot,
    });

    Ok((StatusCode::CREATED, Json(response_body)))
}

async fn query_bots(State(app_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let bots = bot::Entity::find().all(&app_state.db).await?;

    let response_body = json!({
        "bots": bots,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn get_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let bot = bot::Entity::find_by_id(id).one(&app_state.db).await?;

    let Some(bot) = bot else {
        return Err(ApiError::NotFound);
    };

    let response_body = json!({
        "bot": bot,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn patch_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<PatchBotRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let txn = app_state.db.begin().await?;

    let bot = bot::Entity::find_by_id(id).one(&txn).await?;

    let Some(bot) = bot else {
        return Err(ApiError::NotFound);
    };

    ensure_name_is_unique(&txn, &payload.name).await?;

    let mut bot: bot::ActiveModel = bot.into();
    bot.name = Set(payload.name);
    let bot = bot.update(&txn).await?;

    txn.commit().await?;

    let response_body = json!({
        "bot": bot,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn delete_bot_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(bot) = bot::Entity::find_by_id(id).one(&app_state.db).await? else {
        return Err(ApiError::NotFound);
    };
    bot.delete(&app_state.db).await?;
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
