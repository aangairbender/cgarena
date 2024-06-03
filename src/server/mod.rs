mod arena;
pub use arena::*;
use tokio::sync::Mutex;

use std::{path::Path, sync::Arc};


use crate::{api, config::Config};

pub async fn start_arena_server(arena_path: &Path) -> Result<(), anyhow::Error> {
    let config = Config::load(&arena_path)?;

    // let db_path = arena_path.join("cgarena.db");
    // let db_url = format!("sqlite://{}", db_path.display());

    // if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
    //     Sqlite::create_database(&db_url).await?;
    // }

    // let conn = SqliteConnection::connect(&db_url).await?;
    let server_port = config.server.port;
    let arena = Arc::new(Mutex::new(Arena::new(arena_path.to_owned(), config)));

    // tokio::spawn(async move {
    //     arena.launch().await;
    // });

    api::start_api_server(server_port, arena.clone()).await
}
