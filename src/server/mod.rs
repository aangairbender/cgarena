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

use sea_orm::{ConnectionTrait, Database, Statement, DatabaseConnection};
pub use services::arena::*;
use sqlx::{migrate::MigrateDatabase, Sqlite};
use tower_http::cors::CorsLayer;

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

    let app = app(arena_path, db);

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

async fn create_schema(db: &DatabaseConnection) -> Result<(), Box<dyn Error>> {
    db.execute(Statement::from_string(
        db.get_database_backend(),
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.sql")),
    ))
    .await?;
    Ok(())
}

fn app(arena_path: &Path, db: DatabaseConnection) -> Router {
    let bot_service = Arc::new(BotService::new(&arena_path.join("bots"), db));

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

    
    // .fallback(get_service(ServeFile::new("./web-ui/build/index.html")).handle_error(|_| async move {
    //     (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    // }));
    Router::new()
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen to Ctrl-C signal");
}

#[cfg(test)]
mod tests {
    use axum::{http::{Request, self}, body::Body};
    use reqwest::StatusCode;
    use simplelog::TermLogger;
    use tempdir::TempDir;
    use tower::ServiceExt;
    use crate::server::controllers::bot::BotAddReq;

    use super::*;

    #[tokio::test]
    async fn add_bot() {
        TermLogger::init(
            log::LevelFilter::Info,
            simplelog::Config::default(),
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )
        .unwrap();
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        create_new_arena(&path).unwrap();
        let db = Database::connect("sqlite::memory:").await.unwrap();
        create_schema(&db).await.unwrap();
        let app = app(&path, db);

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

        let response = app
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}