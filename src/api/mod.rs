mod errors;
mod routes;

use crate::db::Database;
use crate::worker_manager::WorkerManager;
use axum::Router;
use std::net::SocketAddr;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tracing::{error, info};

pub async fn start(
    port: u16,
    db: Database,
    wm: WorkerManager,
    cancellation_token: CancellationToken,
) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let app_state = AppState { db, wm };

    let router = create_router(app_state).await;

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Cannot bind tcp listener to the target address");
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move { cancellation_token.cancelled().await });

    info!("Arena API server started at {}", addr);
    if let Err(e) = server.await {
        error!("API Server error: {}", e);
    }
    info!("Arena API server closed");
}

async fn create_router(app_state: AppState) -> Router {
    let api_router = Router::new()
        .merge(routes::bots::create_router())
        .with_state(app_state);

    Router::new()
        .nest("/api", api_router)
        // .fallback(get_service(ServeFile::new("./web-ui/build/index.html")).handle_error(|_| async move {
        //     (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
        // }));
        .layer(CorsLayer::permissive())
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub db: Database,
    pub wm: WorkerManager,
}
