mod app;
// TODO: move this to proper place
pub mod config;
mod errors;
mod routes;

use std::{net::SocketAddr, sync::Arc};

use config::Config;
use sea_orm::DatabaseConnection;
use tracing::{info, error};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DatabaseConnection,
}

pub async fn start_api_server<'a>(config: Arc<Config>, db: DatabaseConnection) -> Result<(), anyhow::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));

    let app_state = AppState {
        config,
        db,
    };

    let app = app::create_app(app_state).await;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    info!("Arena API server started at {}", addr);
    if let Err(e) = server.await {
        error!("API Server error: {}", e);
    }
    info!("Arena API server closed");
    Ok(())
}
