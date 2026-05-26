use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    api::{errors::ApiError, AppState},
    domain::MatchFilter,
};

#[derive(Deserialize)]
pub struct ValidateFilterRequest {
    pub filter: String,
}

pub async fn validate_filter(
    State(_): State<AppState>,
    Query(payload): Query<ValidateFilterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let _: MatchFilter = payload.filter.parse().map_err(ApiError::ValidationFailed)?;

    Ok(())
}
