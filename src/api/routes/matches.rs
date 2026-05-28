use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{errors::ApiError, AppState},
    arena_commands::{FetchMatchesResult, MatchOverview, ParticipantOverview},
    domain::{MatchAttribute, MatchAttributeValue, MatchFilter},
};

#[derive(Deserialize)]
pub struct FetchMatches {
    pub filter: String,
    pub including_bots: String, // comma separated
    pub offset: usize,
    pub limit: usize,
}

pub async fn fetch_matches(
    State(app_state): State<AppState>,
    Query(payload): Query<FetchMatches>,
) -> Result<impl IntoResponse, ApiError> {
    let filter: MatchFilter = payload.filter.parse().map_err(ApiError::ValidationFailed)?;
    let including_bots = payload
        .including_bots
        .split(",")
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<i64>().expect("invalid bot id in query"))
        .map(Into::into)
        .collect();

    let res = app_state
        .arena_handle
        .fetch_matches(filter, including_bots, payload.offset, payload.limit)
        .await?;

    Ok(Json(FetchMatchesResponse::from(res)))
}

#[derive(Serialize)]
pub struct FetchMatchesResponse {
    pub matches: Vec<MatchOverviewResponse>,
}

#[derive(Serialize)]
pub struct MatchOverviewResponse {
    pub id: i64,
    pub participants: Vec<ParticipantOverviewResponse>,
    pub seed: i64,
    pub attributes: Vec<MatchAttributeResponse>,
}

#[derive(Serialize)]
pub struct ParticipantOverviewResponse {
    pub rank: u8,
    pub index: usize,
    pub bot_id: i64,
    pub bot_name: String,
    pub error: bool,
}

#[derive(Serialize)]
pub struct MatchAttributeResponse {
    pub name: String,
    pub bot_id: Option<i64>,
    pub turn: Option<u16>,
    pub value: MatchAttributeValueResponse,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum MatchAttributeValueResponse {
    Integer(i64),
    Float(f64),
    String(String),
}

impl From<FetchMatchesResult> for FetchMatchesResponse {
    fn from(value: FetchMatchesResult) -> Self {
        Self {
            matches: value.matches.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<MatchOverview> for MatchOverviewResponse {
    fn from(value: MatchOverview) -> Self {
        Self {
            id: value.id.into(),
            participants: value.participants.into_iter().map(Into::into).collect(),
            seed: value.seed,
            attributes: value.attributes.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ParticipantOverview> for ParticipantOverviewResponse {
    fn from(value: ParticipantOverview) -> Self {
        Self {
            rank: value.rank,
            index: value.index,
            bot_id: value.bot_id.into(),
            bot_name: value.bot_name.to_string(),
            error: value.error,
        }
    }
}

impl From<MatchAttribute> for MatchAttributeResponse {
    fn from(value: MatchAttribute) -> Self {
        Self {
            name: value.name,
            bot_id: value.bot_id.map(|id| id.into()),
            turn: value.turn.into(),
            value: value.value.into(),
        }
    }
}

impl From<MatchAttributeValue> for MatchAttributeValueResponse {
    fn from(value: MatchAttributeValue) -> Self {
        match value {
            MatchAttributeValue::Integer(x) => Self::Integer(x),
            MatchAttributeValue::Float(x) => Self::Float(x),
            MatchAttributeValue::String(x) => Self::String(x),
        }
    }
}
