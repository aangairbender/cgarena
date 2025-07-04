mod errors;
mod models;
mod routes;
mod web_router;

use crate::api::routes::{delete_bot, fetch_bot_leaderboard, fetch_bots, rename_bot};
use crate::api::web_router::create_web_router;
use crate::arena_handle::ArenaHandle;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use routes::create_bot;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

pub async fn start(
    listener: TcpListener,
    arena_handle: ArenaHandle,
    cancellation_token: CancellationToken,
) {
    let app_state = AppState { arena_handle };
    let router = create_router(app_state).await;
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move { cancellation_token.cancelled().await });

    info!("Arena API server started");
    if let Err(e) = server.await {
        error!("API Server error: {}", e);
    }
    info!("Arena API server closed");
}

async fn create_router(app_state: AppState) -> Router {
    let api_router = Router::new()
        .route("/bots", post(create_bot))
        .route("/bots", get(fetch_bots))
        .route("/bots/:id", delete(delete_bot))
        .route("/bots/:id", get(fetch_bot_leaderboard))
        .route("/bots/:id", patch(rename_bot))
        .with_state(app_state);

    create_web_router()
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub arena_handle: ArenaHandle,
}
