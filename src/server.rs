use std::path::Path;

use crate::{api, arena::Arena, config::Config, db::Database};

pub async fn start_server(arena_path: &Path) -> Result<(), anyhow::Error> {
    let config = Config::load(arena_path)?;

    let db_path = arena_path.join("cgarena.db");
    let db_url = format!("sqlite://{}", db_path.display());

    let db = Database::new(&db_url).await;

    let server_port = config.server.port;
    let arena = Arena::new(arena_path.to_owned(), config, db).await;

    api::start_api_server(server_port, arena).await
}
