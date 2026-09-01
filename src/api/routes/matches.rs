use anyhow::{anyhow, bail};
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{errors::ApiError, AppState},
    arena_commands::{FetchMatchesResult, MatchOverview, ParticipantOverview},
    domain::{BotId, MatchAttribute, MatchAttributeValue, MatchFilter},
};

#[derive(Deserialize)]
pub struct FetchMatches {
    pub filter: String,
    pub including_bots: String, // comma separated
    pub offset: String,
    pub limit: String,
}

pub async fn fetch_matches(
    State(app_state): State<AppState>,
    Query(payload): Query<FetchMatches>,
) -> Result<impl IntoResponse, ApiError> {
    let (filter, including_bots, offset, limit) = parse_request(&payload)?;

    let res = app_state
        .arena_handle
        .fetch_matches(filter, including_bots, offset, limit)
        .await?;

    Ok(Json(FetchMatchesResponse::from(res)))
}

fn parse_request(
    payload: &FetchMatches,
) -> Result<(MatchFilter, Vec<BotId>, usize, usize), ApiError> {
    let offset = payload.offset.parse().map_err(|_| {
        ApiError::ValidationFailed(anyhow!("offset must be a non-negative integer"))
    })?;
    if i64::try_from(offset).is_err() {
        return Err(ApiError::ValidationFailed(anyhow!("offset is too large")));
    }
    let limit = payload
        .limit
        .parse()
        .map_err(|_| ApiError::ValidationFailed(anyhow!("limit must be an integer")))?;
    if !(1..=100).contains(&limit) {
        return Err(ApiError::ValidationFailed(anyhow!(
            "limit must be between 1 and 100"
        )));
    }
    let filter = payload.filter.parse().map_err(ApiError::ValidationFailed)?;
    let including_bots =
        parse_including_bots(&payload.including_bots).map_err(ApiError::ValidationFailed)?;
    Ok((filter, including_bots, offset, limit))
}

fn parse_including_bots(value: &str) -> anyhow::Result<Vec<BotId>> {
    value
        .split(',')
        .filter(|value| !value.is_empty())
        .map(|value| {
            let id = value
                .parse::<i64>()
                .map_err(|_| anyhow!("invalid bot id: {value}"))?;
            if id <= 0 {
                bail!("invalid bot id: {value}");
            }
            Ok(id.into())
        })
        .collect()
}

#[derive(Serialize)]
pub struct FetchMatchesResponse {
    pub matches: Vec<MatchOverviewResponse>,
    pub has_more: bool,
}

#[derive(Serialize)]
pub struct MatchOverviewResponse {
    pub id: i64,
    pub participants: Vec<ParticipantOverviewResponse>,
    pub seed: String,
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
    pub value: String,
}

impl From<FetchMatchesResult> for FetchMatchesResponse {
    fn from(value: FetchMatchesResult) -> Self {
        Self {
            matches: value.matches.into_iter().map(Into::into).collect(),
            has_more: value.has_more,
        }
    }
}

impl From<MatchOverview> for MatchOverviewResponse {
    fn from(value: MatchOverview) -> Self {
        Self {
            id: value.id.into(),
            participants: value.participants.into_iter().map(Into::into).collect(),
            seed: value.seed.to_string(),
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
            turn: value.turn,
            value: value.value.into(),
        }
    }
}

impl From<MatchAttributeValue> for String {
    fn from(value: MatchAttributeValue) -> Self {
        match value {
            MatchAttributeValue::Integer(x) => x.to_string(),
            MatchAttributeValue::Float(x) => x.to_string(),
            MatchAttributeValue::String(x) => x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(including_bots: &str, limit: &str) -> FetchMatches {
        FetchMatches {
            filter: String::new(),
            including_bots: including_bots.to_string(),
            offset: "0".to_string(),
            limit: limit.to_string(),
        }
    }

    #[test]
    fn malformed_bot_ids_are_validation_errors() {
        for including_bots in ["abc", "1,abc", "0", "-1"] {
            assert!(matches!(
                parse_request(&request(including_bots, "10")),
                Err(ApiError::ValidationFailed(_))
            ));
        }
    }

    #[test]
    fn limits_outside_supported_range_are_validation_errors() {
        for limit in ["0", "101", "-1", "abc", "184467440737095516160"] {
            assert!(matches!(
                parse_request(&request("", limit)),
                Err(ApiError::ValidationFailed(_))
            ));
        }
    }

    #[test]
    fn malformed_offsets_are_validation_errors() {
        for offset in ["-1", "abc", "9223372036854775808", "184467440737095516160"] {
            let mut payload = request("", "10");
            payload.offset = offset.to_string();
            assert!(matches!(
                parse_request(&payload),
                Err(ApiError::ValidationFailed(_))
            ));
        }
    }
    #[test]
    fn serializes_i64_seed_without_precision_loss() {
        let response = MatchOverviewResponse {
            id: 1,
            participants: vec![],
            seed: i64::MAX.to_string(),
            attributes: vec![],
        };

        let json = serde_json::to_value(response).unwrap();

        assert_eq!(
            json["seed"],
            serde_json::Value::String(i64::MAX.to_string())
        );
    }
}
