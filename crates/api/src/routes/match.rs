use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use entity::{participation, r#match};
use rand::Rng;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, EntityTrait, IntoActiveModel, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::{Validate, ValidationError};

use crate::{config::Config, errors::AppError, AppState};

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

    let txn = app_state.db.begin().await.map_err(anyhow::Error::from)?;

    let r#match = r#match::ActiveModel {
        id: NotSet,
        seed: Set(seed),
        status: Set(r#match::MatchStatus::Pending),
        created_at: Set(Utc::now()),
        tag: Set(payload.tag),
    };

    let r#match = r#match.insert(&txn).await.map_err(anyhow::Error::from)?;

    for (index, bot_id) in payload.bot_ids.into_iter().enumerate() {
        let participation = participation::Model {
            bot_id,
            match_id: r#match.id,
            index: index as u8,
            score: None,
        };

        participation
            .into_active_model()
            .insert(&txn)
            .await
            .map_err(anyhow::Error::from)?;
    }

    txn.commit().await.map_err(anyhow::Error::from)?;

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
    Path(id): Path<i32>,
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
    #[validate(custom(function = "validate_bot_ids", arg = "&'v_a Config"))]
    pub bot_ids: Vec<i32>,
}

fn validate_bot_ids(bot_ids: &Vec<i32>, config: &Config) -> Result<(), ValidationError> {
    if (bot_ids.len() as u32) < config.game.min_players {
        return Err(ValidationError::new("Not enough bots, check your config"));
    }
    if (bot_ids.len() as u32) > config.game.max_players {
        return Err(ValidationError::new("Too many bots, check your config"));
    }
    Ok(())
}
