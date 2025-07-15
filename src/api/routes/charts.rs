use anyhow::anyhow;
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{
    api::{errors::ApiError, AppState},
    arena_commands::{ChartItem, ChartOverview, ChartTurnData},
    domain::MatchFilter,
};

#[derive(Deserialize)]
pub struct ChartRequest {
    pub filter: String,
    pub attribute_name: String,
}

pub async fn chart(
    State(app_state): State<AppState>,
    Json(payload): Json<ChartRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let filter: MatchFilter = payload.filter.parse().map_err(ApiError::ValidationFailed)?;
    if payload.attribute_name.is_empty() {
        return Err(ApiError::ValidationFailed(anyhow!(
            "Attribute name cannot be empty"
        )));
    }

    let res = app_state
        .arena_handle
        .chart(filter, payload.attribute_name)
        .await?;

    Ok(Json(ChartOverviewResponse::from(res)))
}

#[derive(Serialize)]
pub struct ChartOverviewResponse {
    pub items: Vec<ChartItemResponse>,
    pub total_matches: u64,
}

#[derive(Serialize)]
pub struct ChartItemResponse {
    pub bot_id: i64,
    pub data: Vec<ChartTurnDataResponse>,
}

#[derive(Serialize)]
pub struct ChartTurnDataResponse {
    pub turn: u16,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

impl From<ChartOverview> for ChartOverviewResponse {
    fn from(value: ChartOverview) -> Self {
        ChartOverviewResponse {
            items: value.items.into_iter().map(Into::into).collect(),
            total_matches: value.total_matches,
        }
    }
}

impl From<ChartItem> for ChartItemResponse {
    fn from(value: ChartItem) -> Self {
        ChartItemResponse {
            bot_id: value.bot_id.into(),
            data: value.data.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<ChartTurnData> for ChartTurnDataResponse {
    fn from(value: ChartTurnData) -> Self {
        ChartTurnDataResponse {
            turn: value.turn,
            avg: value.avg,
            min: value.min,
            max: value.max,
        }
    }
}
