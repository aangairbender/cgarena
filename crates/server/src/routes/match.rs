use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use entity::r#match;
use rand::Rng;
use sea_orm::{EntityTrait, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use validator::Validate;

use crate::{errors::AppError, AppState};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/matches", post(create_match))
        .route("/matches", get(query_matches))
        .route("/matches/:id", get(get_match_by_id))
}

async fn create_match(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateMatchRequest>,
) -> Result<impl IntoResponse, AppError> {
    let seed = payload.seed.unwrap_or_else(|| rand::thread_rng().gen());

    let r#match = r#match::ActiveModel {
        id: Set(Uuid::new_v4()),
        seed: Set(seed),
        status: Set(r#match::MatchStatus::Pending),
        created_at: Set(Utc::now()),
        tag: Set(payload.tag),
    };

    let r#match = r#match::Entity::insert(r#match)
        .exec_with_returning(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "match": r#match,
    });

    Ok((StatusCode::CREATED, Json(response_body)))
}

async fn query_matches(State(app_state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let matches = r#match::Entity::find()
        .all(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "matches": matches,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

async fn get_match_by_id(
    State(app_state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let r#match = r#match::Entity::find_by_id(id)
        .one(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let Some(r#match) = r#match else {
        return Err(AppError::NotFound);
    };

    let response_body = json!({
        "match": r#match,
    });

    Ok((StatusCode::OK, Json(response_body)))
}

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateMatchRequest {
    pub seed: Option<i32>,
    #[validate(length(min = 1, max = 32))]
    pub tag: Option<String>,
}
