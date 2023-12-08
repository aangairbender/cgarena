mod arena;

use std::{path::Path, sync::Arc};

use arena::Arena;
use config::Config;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use sqlx::{migrate::MigrateDatabase, Sqlite};
use tokio::sync::mpsc;

pub async fn start_arena_server(arena_path: &Path) -> Result<(), anyhow::Error> {
    let config = Arc::new(Config::load(&arena_path.join("cgarena_config.toml"))?);

    let db_path = arena_path.join("cgarena.db");
    let db_url = format!("sqlite://{}", db_path.display());

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url).await?;
    }

    let db = Database::connect(&db_url).await?;
    Migrator::up(&db, None).await?;

    let (match_queue_tx, match_queue_rx) = mpsc::unbounded_channel::<i32>();

    let arena = Arena::new(config.clone(), db.clone(), match_queue_rx);

    tokio::spawn(async move {
        arena.launch().await;
    });

    api::start_api_server(config, db, match_queue_tx).await
}
