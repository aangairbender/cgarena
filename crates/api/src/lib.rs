mod app;
mod config;
mod errors;
mod routes;

use std::{error::Error, net::SocketAddr, path::Path, sync::Arc};

use config::Config;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use sqlx::{migrate::MigrateDatabase, Sqlite};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DatabaseConnection,
}

pub async fn start_arena_server(arena_path: &Path) -> Result<(), Box<dyn Error>> {
    let config = config::Config::load(&arena_path.join("cgarena_config.toml"))?;

    let db_path = arena_path.join("cgarena.db");
    let db_url = format!("sqlite://{}", db_path.display());

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url).await?;
    }

    let db = Database::connect(&db_url).await?;
    Migrator::up(&db, None).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));

    let app_state = AppState {
        config: Arc::new(config),
        db,
    };

    let app = app::create_app(app_state).await;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    log::info!("Arena server started at {}", addr);
    if let Err(e) = server.await {
        log::error!("Server error: {}", e);
    }
    log::info!("Arena server closed");
    Ok(())
}
