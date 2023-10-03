use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::server::{utils::custom_response::CustomResponseResult, AppState};

pub fn create_route() -> Router<AppState, Body> {
    Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs", get(query_jobs))
        .route("/jobs/:id", get(query_job_by_id))
}

pub async fn create_job(
    State(app_state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> CustomResponseResult<()> {
    payload.validate()?;

    unimplemented!()
}

pub async fn query_jobs(State(app_state): State<AppState>) -> Response {
    unimplemented!()
}

pub async fn query_job_by_id(State(app_state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    unimplemented!()
}

#[derive(Serialize, Deserialize, Validate)]
pub struct CreateJobRequest {
    #[validate(range(min = 1))]
    pub matches_count: i32,
}
