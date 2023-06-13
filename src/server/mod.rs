mod bot;

use std::net::SocketAddr;

use axum::{Router, routing::{post, delete}};

use crate::config::ServerConfig;

pub async fn start_arena_server(config: &ServerConfig) {
    let app = Router::new()
        .route("/bots", post(bot::add).get(bot::list))
        .route("/bots/:id", delete(bot::remove).patch(bot::patch));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Server cannot be started");
}
