use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use config::Config;
use entity::{participation, r#match::{self, MatchStatus}};
use rand::Rng;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, EntityTrait, IntoActiveModel, Set, TransactionTrait, ModelTrait, prelude::DateTimeUtc,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::{Validate, ValidationError, ValidateArgs};

use crate::{errors::ApiError, AppState};

pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/matches", post(create_match))
        .route("/matches", get(query_matches))
        .route("/matches/:id", get(get_match_by_id))
}

async fn create_match(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateMatchRequest>,
) -> Result<impl IntoResponse, ApiError> {
    payload.validate_args(&app_state.config)?;

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

    app_state.match_queue_tx.send(r#match.id)
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "match": r#match,
    });

    Ok((StatusCode::CREATED, Json(response_body)))
}

async fn query_matches(State(app_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
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
) -> Result<impl IntoResponse, ApiError> {
    let r#match = r#match::Entity::find_by_id(id)
        .one(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let Some(r#match) = r#match else {
        return Err(ApiError::NotFound);
    };

    let participations = r#match.find_related(participation::Entity)
        .all(&app_state.db)
        .await
        .map_err(anyhow::Error::from)?;

    let response_body = json!({
        "match": MatchResponse {
            id: r#match.id,
            seed: r#match.seed,
            status: r#match.status,
            created_at: r#match.created_at,
            tag: r#match.tag,
            participants: participations.into_iter().map(|p| {
                Participant { bot_id: p.bot_id, index: p.index, score: p.score }
            }).collect()
        },
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

#[derive(Serialize)]
struct MatchResponse {
    id: i32,
    seed: i32,
    status: MatchStatus,
    created_at: DateTimeUtc,
    tag: Option<String>,
    participants: Vec<Participant>,
}

#[derive(Serialize)]
struct Participant {
    bot_id: i32,
    index: u8,
    score: Option<i32>,
}