use axum::Router;
use sea_orm::DatabaseConnection;
use std::path::Path;
use tower_http::{cors::CorsLayer, trace};

use super::{routes, AppState};

pub async fn create_app(arena_path: &Path, db: DatabaseConnection) -> Router {
    let app_state = AppState {
        arena_path: arena_path.to_owned(),
        db,
    };

    let api_router = Router::new()
        .merge(routes::bot::create_route())
        .with_state(app_state);

    // .fallback(get_service(ServeFile::new("./web-ui/build/index.html")).handle_error(|_| async move {
    //     (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    // }));
    Router::new()
        .nest("/api", api_router)
        .layer(
            trace::TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().include_headers(true))
                .on_request(trace::DefaultOnRequest::new().level(tracing::Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO)),
        )
        .layer(CorsLayer::permissive())
}
