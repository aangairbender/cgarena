mod app;
mod config;
mod errors;
mod models;
mod routes;
mod services;
// mod workers;

use std::{
    error::Error,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
pub use services::arena::*;
use sqlx::{migrate::MigrateDatabase, Sqlite};

#[derive(Clone)]
pub struct AppState {
    pub arena_path: PathBuf,
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

    let app = app::create_app(arena_path, db).await;

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    log::info!("Arena server started at {}", addr);
    if let Err(e) = server.await {
        log::error!("Server error: {}", e);
    }
    log::info!("Arena server closed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use tempdir::TempDir;
    use tower::ServiceExt;

    use crate::routes::bot::CreateBotRequest;

    use super::*;

    #[tokio::test]
    async fn add_bot() {
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        create_new_arena(&path).unwrap();
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        let app = app::create_app(&path, db).await;

        let body = CreateBotRequest {
            name: "test".to_string(),
            source_code: "int main(){}".to_string(),
            language: "cpp".to_string(),
        };

        let request = Request::builder()
            .method("POST")
            .uri("/api/bots")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
