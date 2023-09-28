mod config;
mod controllers;
mod entities;
mod enums;
mod services;
// mod workers;

use std::{error::Error, net::SocketAddr, path::Path, sync::Arc};

use axum::{
    routing::{delete, post},
    Router,
};

use sea_orm::{ConnectionTrait, Database, Statement};
pub use services::arena::*;
use sqlx::{migrate::MigrateDatabase, Sqlite};
use tower_http::cors::CorsLayer;

use crate::server::{config::Config, services::bot_service::BotService};

const DB_URL: &str = "sqlite://cgarena.db";

#[derive(Clone)]
pub struct AppState {
    pub bot_service: Arc<BotService>,
}

pub async fn start_arena_server(path: &Path) -> Result<(), Box<dyn Error>> {
    let config = Config::load(&path.join("cgarena_config.toml"))?;

    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        Sqlite::create_database(DB_URL).await?;
    }

    let db = Database::connect(DB_URL).await?;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql")),
    ))
    .await?;

    let bot_service = Arc::new(BotService::new(&path.join("bots"), db.clone()));

    let app_state = AppState { bot_service };

    let api_router = Router::new()
        .route(
            "/bots",
            post(controllers::bot::add).get(controllers::bot::list),
        )
        .route(
            "/bots/:id",
            delete(controllers::bot::remove).patch(controllers::bot::patch),
        )
        .with_state(app_state);

    let app = Router::new()
        .nest("/api", api_router)
        .layer(CorsLayer::permissive());
    // .fallback(get_service(ServeFile::new("./web-ui/build/index.html")).handle_error(|_| async move {
    //     (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    // }));

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
