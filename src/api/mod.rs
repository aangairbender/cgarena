mod app;
mod errors;
mod routes;

use std::net::SocketAddr;

use tracing::{error, info};

use crate::db::Database;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

pub async fn start_api_server(port: u16, db: Database) -> Result<(), anyhow::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let app_state = AppState { db };

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
