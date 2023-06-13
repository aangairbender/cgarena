use axum::extract::Path;
use reqwest::StatusCode;
use uuid::Uuid;

pub async fn add() -> StatusCode {
    todo!()
}

pub async fn list() -> StatusCode {
    todo!()
}

pub async fn remove(
    Path(id): Path<Uuid>
) -> StatusCode {
    todo!()
}

pub async fn patch(
    Path(id): Path<Uuid>
) -> StatusCode {
    todo!()
}
