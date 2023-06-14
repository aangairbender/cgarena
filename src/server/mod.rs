mod bot;
mod services;
mod config;
mod workers;

use std::{net::SocketAddr, path::Path, sync::Arc, error::Error};

use axum::{Router, routing::{post, delete}};

pub use services::arena_service::*;

pub async fn start_arena_server(path: &Path) -> Result<(), Box<dyn Error>> {
    let arena_service = Arc::new(ArenaService::new(path)?);
    let port = arena_service.server_config().port;

    let app = Router::new()
        .route("/bots", post(bot::add).get(bot::list))
        .route("/bots/:id", delete(bot::remove).patch(bot::patch))
        .with_state(arena_service);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal());

    log::info!("Arena server started at {}", addr);
    if let Err(e) = server.await {
        log::error!("Server error: {}", e);
    }
    log::info!("Arena server closed");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen to Ctrl-C signal");
}