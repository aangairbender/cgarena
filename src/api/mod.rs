mod app;
mod errors;
mod routes;

use std::net::SocketAddr;

use tracing::{error, info};

use crate::arena::Arena;

#[derive(Clone)]
pub struct AppState {
    pub arena: Arena,
}

pub async fn start_api_server(port: u16, arena: Arena) -> Result<(), anyhow::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    let app_state = AppState { arena };

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
