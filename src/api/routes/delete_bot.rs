use crate::api::errors::ApiError;
use crate::api::AppState;
use crate::domain::BotId;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn delete_bot(
    State(app_state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let bot_id: BotId = id.into();

    app_state
        .arena_handle
        .delete_bot(bot_id)
        .await;

    Ok(StatusCode::OK)
}
