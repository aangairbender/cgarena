mod errors;
mod models;
mod routes;
mod web_router;

use crate::api::routes::{
    bots, charts, configuration, enable_matchmaking, fetch_status, leaderboards, matches, replays,
};
use crate::api::web_router::create_web_router;
use crate::arena_handle::ArenaHandle;
use crate::replay_viewer::ReplayViewer;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::error;

pub async fn start(
    listener: TcpListener,
    pool: SqlitePool,
    runtime: Option<RuntimeDependencies>,
    cancellation_token: CancellationToken,
) {
    let app_state = AppState { pool, runtime };
    let router = create_router(app_state).await;
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async move { cancellation_token.cancelled().await });

    if let Err(e) = server.await {
        error!("API Server error: {}", e);
    }
}
pub(crate) async fn create_router(app_state: AppState) -> Router {
    let api_router = Router::new()
        .route("/bots", post(bots::create_bot))
        .route("/bots/{id}", delete(bots::delete_bot))
        .route("/bots/{id}", patch(bots::rename_bot))
        .route(
            "/configuration",
            get(configuration::fetch_configuration).put(configuration::apply_configuration),
        )
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
pub(crate) struct RuntimeDependencies {
    pub arena_handle: ArenaHandle,
    pub replay_viewer: ReplayViewer,
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub pool: SqlitePool,
    runtime: Option<RuntimeDependencies>,
}

impl AppState {
    pub fn arena_handle(&self) -> Result<&ArenaHandle, errors::ApiError> {
        self.runtime
            .as_ref()
            .map(|runtime| &runtime.arena_handle)
            .ok_or(errors::ApiError::RuntimeUnavailable)
    }

    pub fn replay_viewer(&self) -> Result<&ReplayViewer, errors::ApiError> {
        self.runtime
            .as_ref()
            .map(|runtime| &runtime.replay_viewer)
            .ok_or(errors::ApiError::RuntimeUnavailable)
    }

    pub fn runtime_available(&self) -> bool {
        self.runtime.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    async fn setup_app() -> (Router, SqlitePool) {
        let pool = crate::db::in_memory().await.unwrap();
        crate::db::migrate(&pool).await.unwrap();
        let app = create_router(AppState {
            pool: pool.clone(),
            runtime: None,
        })
        .await;
        (app, pool)
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn unconfigured_http_app_accepts_one_atomic_configuration() {
        let (app, pool) = setup_app().await;

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/configuration")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let state = response_json(response).await;
        assert!(state["active"].is_null());
        assert_eq!(state["runtime_available"], false);

        let mut invalid = crate::config::ArenaConfig::default();
        invalid.workers.clear();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&invalid).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(crate::db::fetch_arena_config(&pool)
            .await
            .unwrap()
            .is_none());

        let candidate = crate::config::ArenaConfig::default();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&candidate).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let state = response_json(response).await;
        assert_eq!(state["active"]["game"]["min_players"], 2);
        assert_eq!(state["runtime_available"], false);
        assert!(crate::db::fetch_arena_config(&pool)
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn runtime_routes_report_unavailable_during_setup() {
        let (app, _) = setup_app().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response_json(response).await["error_code"],
            "arena_unavailable"
        );
    }
}
