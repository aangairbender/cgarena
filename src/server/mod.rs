mod app;
mod config;
mod entities;
mod enums;
mod routes;
mod services;
// mod workers;

use std::{error::Error, net::SocketAddr, path::Path, sync::Arc};

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
pub use services::arena::*;
use sqlx::{migrate::MigrateDatabase, Sqlite};

use crate::server::{config::Config, services::bot_service::BotService};

const DB_URL: &str = "sqlite://cgarena.db";

#[derive(Clone)]
pub struct AppState {
    pub bot_service: Arc<BotService>,
}

pub async fn start_arena_server(arena_path: &Path) -> Result<(), Box<dyn Error>> {
    let config = Config::load(&arena_path.join("cgarena_config.toml"))?;

    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        Sqlite::create_database(DB_URL).await?;
    }

    let db = Database::connect(DB_URL).await?;

    create_schema(&db).await?;

    let app = app::create_app(arena_path, db).await;

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));
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

async fn create_schema(db: &DatabaseConnection) -> Result<(), Box<dyn Error>> {
    db.execute(Statement::from_string(
        db.get_database_backend(),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql")),
    ))
    .await?;
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen to Ctrl-C signal");
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{self, Request},
    };
    use reqwest::StatusCode;
    use tempdir::TempDir;
    use tower::ServiceExt;

    use crate::server::routes::bot::BotAddReq;

    use super::*;

    #[tokio::test]
    async fn add_bot() {
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        create_new_arena(&path).unwrap();
        let db = Database::connect("sqlite::memory:").await.unwrap();
        create_schema(&db).await.unwrap();
        let app = app::create_app(&path, db).await;

        let body = BotAddReq {
            name: "test".to_string(),
            source_code: "int main(){}".to_string(),
            language: enums::Language::Cpp,
        };

        let request = Request::builder()
            .method("POST")
            .uri("/api/bots")
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
