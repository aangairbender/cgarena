mod errors;
mod models;
mod routes;
mod web_router;

use crate::api::routes::{
    bots, charts, configuration, enable_matchmaking, fetch_status, leaderboards, managed_referee,
    matches, replays,
};
use crate::api::web_router::create_web_router;
use crate::runtime::ArenaRuntime;
use crate::{arena_handle::ArenaHandle, replay_viewer::ReplayViewer};
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
    runtime: ArenaRuntime,
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
        .route(
            "/referee",
            get(managed_referee::fetch_referee_status).post(managed_referee::start_referee_action),
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
pub(crate) struct AppState {
    pub pool: SqlitePool,
    runtime: ArenaRuntime,
}

impl AppState {
    pub async fn arena_handle(&self) -> Result<ArenaHandle, errors::ApiError> {
        self.runtime
            .arena_handle()
            .await
            .ok_or(errors::ApiError::RuntimeUnavailable)
    }

    pub async fn replay_viewer(&self) -> Result<ReplayViewer, errors::ApiError> {
        self.runtime
            .replay_viewer()
            .await
            .ok_or(errors::ApiError::RuntimeUnavailable)
    }

    pub async fn runtime_available(&self) -> bool {
        self.runtime.is_available().await
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

    async fn setup_app() -> (Router, SqlitePool, ArenaRuntime, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let pool = crate::db::in_memory().await.unwrap();
        crate::db::migrate(&pool).await.unwrap();
        let runtime = ArenaRuntime::new(pool.clone(), directory.path().to_owned());
        let app = create_router(AppState {
            pool: pool.clone(),
            runtime: runtime.clone(),
        })
        .await;
        (app, pool, runtime, directory)
    }

    async fn response_json(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn unconfigured_http_app_accepts_one_atomic_configuration() {
        let (app, pool, runtime, _directory) = setup_app().await;

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
        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn database_command_configuration_starts_runtime_and_serves_a_replay() {
        use crate::config::{CommandRefereeConfig, RefereeConfig, WorkerConfig};
        use std::time::Duration;

        let (app, pool, runtime, directory) = setup_app().await;
        let play_script = directory.path().join("play-match.sh");
        std::fs::write(
            &play_script,
            "#!/bin/sh\nprintf replay > \"$1\"\nprintf '%s\\n' '{\"ranks\":[0,1],\"errors\":[0,0]}'\n",
        )
        .unwrap();

        let mut candidate = crate::config::ArenaConfig::default();
        let WorkerConfig::Embedded(worker) = &mut candidate.workers[0];
        worker.cmd_build = "sh -c true".to_string();
        worker.cmd_run = "true".to_string();
        worker.referee = RefereeConfig::Command(CommandRefereeConfig {
            play_match: format!(
                "sh {} {{REPLAY_PATH}} {{SEED}} {{PLAYERS}}",
                shell_words::quote(&play_script.to_string_lossy())
            ),
            watch_replay:
                "sh -c 'printf fixture > \"$2/test.html\"' sh {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}"
                    .to_string(),
            legacy: false,
        });

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
        assert_eq!(response_json(response).await["runtime_available"], true);

        for name in ["Bot1", "Bot2"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/bots")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({
                                "name": name,
                                "source_code": "fixture",
                                "language": "test"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri("/api/status")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                let status = response_json(response).await;
                if status["leaderboards"][0]["total_matches"]
                    .as_u64()
                    .is_some_and(|matches| matches > 0)
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("database-backed runtime should complete a match");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/matchmaking")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"enabled":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let matches_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM matches")
            .fetch_one(&pool)
            .await
            .unwrap();
        candidate.matchmaking.enabled_on_start = Some(false);
        let WorkerConfig::Embedded(worker) = &mut candidate.workers[0];
        worker.threads = 2;
        worker.cmd_build = "sh -c 'exit 99'".to_string();
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
        let configuration = response_json(response).await;
        assert_eq!(configuration["runtime_available"], true);
        assert_eq!(configuration["active"]["workers"][0]["threads"], 2);
        let matches_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM matches")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(matches_after >= matches_before);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response_json(response).await;
        assert_eq!(status["matchmaking_enabled"], false);
        assert_eq!(status["bots"][0]["builds"][0]["status"], "finished");
        assert_eq!(status["bots"][1]["builds"][0]["status"], "finished");

        let match_id: i64 = sqlx::query_scalar("SELECT id FROM matches LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/matches/{match_id}/replay"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let replay = response_json(response).await;
        let asset_path = format!(
            "/api/replays/{}/test.html",
            replay["session_id"].as_str().unwrap()
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri(asset_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn failed_live_activation_restores_previous_runtime_and_configuration() {
        use crate::config::{CommandRefereeConfig, RefereeConfig, WorkerConfig};

        let (app, pool, runtime, _directory) = setup_app().await;
        let mut original = crate::config::ArenaConfig::default();
        let WorkerConfig::Embedded(worker) = &mut original.workers[0];
        worker.cmd_build = "true".to_string();
        worker.cmd_run = "true".to_string();
        worker.referee = RefereeConfig::Command(CommandRefereeConfig {
            play_match: "true {SEED} {REPLAY_PATH} {PLAYERS}".to_string(),
            watch_replay: "true {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}".to_string(),
            legacy: false,
        });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&original).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let mut unsupported = original;
        unsupported.game.max_players = 3;
        unsupported.ranking =
            crate::config::RankingConfig::Elo(crate::ranking::algorithms::elo::Config { k: None });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&unsupported).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let active = crate::db::fetch_arena_config(&pool).await.unwrap().unwrap();
        assert_eq!(active.game.max_players, 2);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        runtime.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn runtime_routes_report_unavailable_during_setup() {
        let (app, _, _runtime, _directory) = setup_app().await;
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
    #[cfg(unix)]
    fn git(path: &std::path::Path, arguments: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(arguments)
            .current_dir(path)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    #[cfg(unix)]
    fn create_referee_repository(root: &std::path::Path, name: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let repository = root.join(name);
        std::fs::create_dir_all(repository.join("src/main/java/com/codingame/gameengine/runner"))
            .unwrap();
        std::fs::write(
            repository.join("pom.xml"),
            "<project><modelVersion>4.0.0</modelVersion><dependencies></dependencies></project>",
        )
        .unwrap();
        std::fs::write(
            repository
                .join("src/main/java/com/codingame/gameengine/runner/CommandLineInterface.java"),
            "package com.codingame.gameengine.runner; public class CommandLineInterface {}",
        )
        .unwrap();
        std::fs::write(
            repository.join("mvnw"),
            "#!/bin/sh\nset -eu\nmkdir -p target\nprintf fixture > target/referee.jar\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(repository.join("mvnw"))
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(repository.join("mvnw"), permissions).unwrap();
        git(&repository, &["init", "-b", "trunk"]);
        git(&repository, &["add", "."]);
        git(
            &repository,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                "initial referee",
            ],
        );
        repository
    }

    #[cfg(unix)]
    async fn referee_status(app: &Router) -> Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/referee")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        response_json(response).await
    }

    #[cfg(unix)]
    async fn wait_for_referee_action(app: &Router) -> Value {
        for _ in 0..1000 {
            let status = referee_status(app).await;
            if status["operation"]["action"].is_null()
                && !status["operation"]["diagnostic"].is_null()
            {
                return status;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("managed referee action did not finish");
    }

    #[cfg(unix)]
    async fn post_referee_action(app: &Router, action: &str) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/referee")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"action":"{action}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn managed_referee_http_lifecycle_is_explicit_and_preserves_local_state() {
        use std::os::unix::fs::PermissionsExt;

        let (app, pool, runtime, arena) = setup_app().await;
        let source = create_referee_repository(arena.path(), "source-referee");
        let java = arena.path().join("fake-java.sh");
        std::fs::write(
            &java,
            r#"#!/bin/sh
set -eu
tmp=''
replay=''
render=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --cgarena-compat) printf '%s\n' cgarena-referee-v1; exit 0 ;;
    -Djava.io.tmpdir=*) tmp="${1#*=}"; shift ;;
    -r) render=1; shift 2 ;;
    -l) replay="$2"; shift 2 ;;
    *) shift ;;
  esac
done
if [ "$render" = 1 ]; then
  mkdir -p "$tmp/codingame"
  printf fixture > "$tmp/codingame/test.html"
  printf 'Exposed web server dir: %s\n' "$tmp/codingame"
  exec sleep 30
fi
printf '%s\n' '{"scores":{"0":9,"1":4},"errors":{"0":[null],"1":[null]},"agents":[{},{}]}' > "$replay"
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&java).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&java, permissions).unwrap();

        let mut configuration = crate::config::ArenaConfig::default();
        let [crate::config::WorkerConfig::Embedded(worker)] = configuration.workers.as_mut_slice()
        else {
            panic!("fixture must contain one embedded worker");
        };
        worker.referee = crate::config::RefereeConfig::ManagedCodingame(
            crate::config::ManagedCodingameRefereeConfig {
                repository_url: source.display().to_string(),
                branch: None,
                java: Some(java.display().to_string()),
                maven: None,
            },
        );
        worker.cmd_build = "true".to_string();
        worker.cmd_run = "true".to_string();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&configuration).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!arena.path().join("referee").exists());
        assert!(!runtime.is_available().await);

        post_referee_action(&app, "install").await;
        let installed = wait_for_referee_action(&app).await;
        assert_eq!(installed["installed"], true, "{installed}");
        assert_eq!(installed["branch"], "trunk");
        assert!(installed["operation"]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("installed and activated"));
        assert!(runtime.is_available().await);
        assert!(arena.path().join(".cgarena/referee/referee.jar").is_file());
        assert_eq!(
            git(&arena.path().join("referee"), &["log", "-1", "--pretty=%s"]),
            "CG Arena referee adapter v1"
        );
        let pom = std::fs::read_to_string(arena.path().join("referee/pom.xml")).unwrap();
        assert_eq!(
            pom.matches("<artifactId>commons-cli</artifactId>").count(),
            1
        );
        assert_eq!(
            pom.matches("<artifactId>maven-shade-plugin</artifactId>")
                .count(),
            1
        );

        for name in ["ManagedBot1", "ManagedBot2"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/bots")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({
                                "name": name,
                                "source_code": "fixture",
                                "language": "test"
                            })
                            .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let matches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM matches")
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                if matches > 0 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("installed managed referee should run a deterministic match");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/matchmaking")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"enabled":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let match_id: i64 = sqlx::query_scalar("SELECT id FROM matches LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/matches/{match_id}/replay"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let replay = response_json(response).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/replays/{}/test.html",
                        replay["session_id"].as_str().unwrap()
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        std::fs::write(source.join("upstream-change.txt"), "new version").unwrap();
        git(&source, &["add", "."]);
        std::fs::write(arena.path().join("referee/user-commit.txt"), "user commit").unwrap();
        git(&arena.path().join("referee"), &["add", "user-commit.txt"]);
        git(
            &arena.path().join("referee"),
            &[
                "-c",
                "user.name=User",
                "-c",
                "user.email=user@localhost",
                "commit",
                "-m",
                "user referee change",
            ],
        );
        assert_eq!(referee_status(&app).await["committed_ahead"], 2);

        git(
            &source,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                "upstream update",
            ],
        );
        let before_check = referee_status(&app).await;
        assert_eq!(before_check["update_status"], "unavailable");
        assert!(before_check["observed_remote_commit"].is_null());

        post_referee_action(&app, "check").await;
        let checked = wait_for_referee_action(&app).await;
        assert_eq!(checked["update_status"], "update_available");
        assert!(checked["last_successful_check"].is_string());
        assert!(checked["observed_remote_commit"].is_string());
        let fetch_head = std::fs::read(arena.path().join("referee/.git/FETCH_HEAD")).unwrap();
        let restarted = ArenaRuntime::new(pool.clone(), arena.path().to_owned());
        restarted.start_saved(configuration.clone()).await;
        assert!(restarted.is_available().await);
        let restarted_status = restarted.managed_referee_status().await.unwrap();
        assert!(restarted_status.last_successful_check.is_some());
        assert_eq!(
            std::fs::read(arena.path().join("referee/.git/FETCH_HEAD")).unwrap(),
            fetch_head
        );
        restarted.shutdown().await.unwrap();

        let checkout = arena.path().join("referee");
        std::fs::write(checkout.join("staged.txt"), "staged").unwrap();
        git(&checkout, &["add", "staged.txt"]);
        std::fs::write(
            checkout
                .join("src/main/java/com/codingame/gameengine/runner/CommandLineInterface.java"),
            format!(
                "{}\n// local\n",
                include_str!("../../assets/CommandLineInterface.java")
            ),
        )
        .unwrap();
        std::fs::write(checkout.join("untracked.txt"), "untracked").unwrap();
        let dirty = referee_status(&app).await;
        assert_eq!(dirty["staged"], true, "{dirty}");
        assert_eq!(dirty["unstaged"], true, "{dirty}");
        assert_eq!(dirty["untracked"], true, "{dirty}");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/matchmaking")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"enabled":false}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        post_referee_action(&app, "rebuild").await;
        let rebuilt = wait_for_referee_action(&app).await;
        assert!(rebuilt["operation"]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("rebuilt and activated"));
        assert_eq!(rebuilt["staged"], true);
        assert_eq!(rebuilt["untracked"], true);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["matchmaking_enabled"], false);

        post_referee_action(&app, "update").await;
        let updated = wait_for_referee_action(&app).await;
        assert!(
            updated["operation"]["diagnostic"]
                .as_str()
                .unwrap()
                .contains("updated and activated"),
            "{updated}"
        );
        assert_eq!(updated["update_status"], "up_to_date");
        assert!(checkout.join("staged.txt").is_file());
        assert!(checkout.join("untracked.txt").is_file());
        assert!(checkout.join("user-commit.txt").is_file());

        let active_artifact =
            std::fs::read(arena.path().join(".cgarena/referee/referee.jar")).unwrap();
        let replacement_source = create_referee_repository(arena.path(), "replacement-referee");
        let mut replacement_configuration = configuration.clone();
        let [crate::config::WorkerConfig::Embedded(worker)] =
            replacement_configuration.workers.as_mut_slice()
        else {
            panic!("fixture must contain one embedded worker");
        };
        let crate::config::RefereeConfig::ManagedCodingame(selected) = &mut worker.referee else {
            panic!("fixture must select a managed referee");
        };
        selected.repository_url = replacement_source.display().to_string();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&replacement_configuration).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        post_referee_action(&app, "install").await;
        let rejected_replacement = wait_for_referee_action(&app).await;
        assert!(rejected_replacement["operation"]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("checkout has committed, staged, unstaged, or untracked changes"));
        assert_eq!(
            std::fs::read(arena.path().join(".cgarena/referee/referee.jar")).unwrap(),
            active_artifact
        );
        assert!(runtime.is_available().await);

        let adaptation_commit = updated["adaptation_commit"].as_str().unwrap();
        git(&checkout, &["reset", "--hard", adaptation_commit]);
        git(&checkout, &["clean", "-fd"]);
        post_referee_action(&app, "install").await;
        let replaced = wait_for_referee_action(&app).await;
        assert_eq!(
            replaced["installed_repository_url"],
            replacement_source.display().to_string()
        );
        assert!(runtime.is_available().await);

        let failing_source = create_referee_repository(arena.path(), "failing-referee");
        std::fs::write(
            failing_source.join("mvnw"),
            "#!/bin/sh\necho failed-build >&2\nexit 9\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(failing_source.join("mvnw"))
            .unwrap()
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(failing_source.join("mvnw"), permissions).unwrap();
        git(&failing_source, &["add", "mvnw"]);
        git(
            &failing_source,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@localhost",
                "commit",
                "-m",
                "break build",
            ],
        );
        let artifact_before_failed_replacement =
            std::fs::read(arena.path().join(".cgarena/referee/referee.jar")).unwrap();
        let mut failing_configuration = replacement_configuration.clone();
        let [crate::config::WorkerConfig::Embedded(worker)] =
            failing_configuration.workers.as_mut_slice()
        else {
            panic!("fixture must contain one embedded worker");
        };
        let crate::config::RefereeConfig::ManagedCodingame(selected) = &mut worker.referee else {
            panic!("fixture must select a managed referee");
        };
        selected.repository_url = failing_source.display().to_string();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&failing_configuration).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        post_referee_action(&app, "install").await;
        let failed_replacement = wait_for_referee_action(&app).await;
        assert!(failed_replacement["operation"]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("failed-build"));
        assert_eq!(
            std::fs::read(arena.path().join(".cgarena/referee/referee.jar")).unwrap(),
            artifact_before_failed_replacement
        );
        assert_eq!(
            git(&checkout, &["remote", "get-url", "upstream"]),
            replacement_source.display().to_string()
        );
        assert!(runtime.is_available().await);

        let command_configuration = crate::config::ArenaConfig {
            matchmaking: crate::config::MatchmakingConfig {
                enabled_on_start: Some(false),
                ..configuration.matchmaking.clone()
            },
            workers: vec![crate::config::WorkerConfig::Embedded(
                crate::config::EmbeddedWorkerConfig {
                    threads: 1,
                    referee: crate::config::RefereeConfig::Command(
                        crate::config::CommandRefereeConfig {
                            play_match: "true {SEED} {REPLAY_PATH} {P1} {P2}".to_string(),
                            watch_replay: "true {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}"
                                .to_string(),
                            legacy: false,
                        },
                    ),
                    cmd_build: "true".to_string(),
                    cmd_run: "true".to_string(),
                },
            )],
            ..configuration.clone()
        };
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&command_configuration).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(runtime.is_available().await);
        assert!(checkout.is_dir());
        let mut switch_back = replacement_configuration;
        switch_back.matchmaking.enabled_on_start = Some(false);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/configuration")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&switch_back).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let state = response_json(response).await;
        assert_eq!(state["runtime_available"], true);
        assert_eq!(
            state["active"]["workers"][0]["referee"]["type"],
            "managed_codingame"
        );
        assert_eq!(
            git(&checkout, &["remote", "get-url", "upstream"]),
            replacement_source.display().to_string()
        );

        runtime.shutdown().await.unwrap();
    }
}
