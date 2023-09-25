mod config;
mod controllers;
mod services;
// mod workers;

use std::{error::Error, net::SocketAddr, path::Path, sync::Arc};

use axum::{
    routing::{delete, post, get_service},
    Router,
};

use reqwest::StatusCode;
pub use services::arena::*;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite};
use tower_http::services::ServeFile;

use crate::server::{config::Config, services::bot_service::BotService};

const DB_URL: &str = "sqlite://cgarena.db";

pub async fn start_arena_server(path: &Path) -> Result<(), Box<dyn Error>> {
    let config = Config::load(&path.join("cgarena_config.toml"))?;

    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        Sqlite::create_database(DB_URL).await?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(DB_URL)
        .await?;

    let bot_service = Arc::new(BotService::new(path, pool.clone()));

    let api_router = Router::new()
        .route(
            "/bots",
            post(controllers::bot::add).get(controllers::bot::list),
        )
        .route(
            "/bots/:id",
            delete(controllers::bot::remove).patch(controllers::bot::patch),
        )
        .with_state(bot_service);

    let app = Router::new()
        .nest("/api", api_router)
        .fallback(get_service(ServeFile::new("./web-ui/build/index.html")).handle_error(|_| async move {
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
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
