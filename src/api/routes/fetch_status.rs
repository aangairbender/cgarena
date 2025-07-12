use crate::api::errors::ApiError;
use crate::api::models::FetchStatusResponse;
use crate::api::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;

pub async fn fetch_status(
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let res = app_state.arena_handle.fetch_status().await;
    Ok(Json(FetchStatusResponse::from(res)))
}
