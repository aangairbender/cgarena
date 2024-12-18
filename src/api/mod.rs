mod errors;
mod routes;
mod web_router;

use crate::api::web_router::create_web_router;
use crate::arena::ArenaCommand;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

pub async fn start(
    listener: TcpListener,
    arena_tx: Sender<ArenaCommand>,
    cancellation_token: CancellationToken,
) {
    let app_state = AppState { arena_tx };
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
        .merge(routes::bots::create_router())
        .with_state(app_state);

    create_web_router()
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub arena_tx: Sender<ArenaCommand>,
}
