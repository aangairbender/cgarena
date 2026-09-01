mod errors;
mod models;
mod routes;
mod web_router;

use crate::api::routes::{
    bots, charts, enable_matchmaking, fetch_status, leaderboards, matches, replays,
};
use crate::api::web_router::create_web_router;
use crate::arena_handle::ArenaHandle;
use crate::replay_viewer::ReplayViewer;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::error;

pub async fn start(
    listener: TcpListener,
    arena_handle: ArenaHandle,
    replay_viewer: ReplayViewer,
    cancellation_token: CancellationToken,
) {
    let app_state = AppState {
        arena_handle,
        replay_viewer,
    };
    let router = create_router(app_state).await;
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move { cancellation_token.cancelled().await });

    if let Err(e) = server.await {
        error!("API Server error: {}", e);
    }
}

async fn create_router(app_state: AppState) -> Router {
    let api_router = Router::new()
        .route("/bots", post(bots::create_bot))
        .route("/bots/{id}", delete(bots::delete_bot))
        .route("/bots/{id}", patch(bots::rename_bot))
        .route("/bots/{id}/source", get(bots::fetch_source_code))
        .route("/leaderboards", post(leaderboards::create_leaderboard))
        .route("/leaderboards/{id}", patch(leaderboards::patch_leaderboard))
        .route(
            "/leaderboards/{id}",
            delete(leaderboards::delete_leaderboard),
        )
        .route("/status", get(fetch_status::fetch_status))
        .route("/chart", post(charts::chart))
        .route("/matchmaking", put(enable_matchmaking::enable_matchmaking))
        .route("/matches", get(matches::fetch_matches))
        .route("/matches/{id}/replay", get(replays::watch_replay))
        .route("/replays/{session_id}", delete(replays::close_replay))
        .route("/replays/{session_id}/{*path}", get(replays::replay_asset))
        .with_state(app_state);

    create_web_router()
        .nest("/api", api_router)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub arena_handle: ArenaHandle,
    pub replay_viewer: ReplayViewer,
}
