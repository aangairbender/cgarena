use std::{path::Path, sync::Arc};

use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use sqlx::{Sqlite, migrate::MigrateDatabase};


pub async fn start_arena_server(arena_path: &Path) -> Result<(), anyhow::Error> {
    let config = api::config::Config::load(&arena_path.join("cgarena_config.toml"))?;

    let db_path = arena_path.join("cgarena.db");
    let db_url = format!("sqlite://{}", db_path.display());

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url).await?;
    }

    let db = Database::connect(&db_url).await?;
    Migrator::up(&db, None).await?;

    api::start_api_server(Arc::new(config), db).await
}
