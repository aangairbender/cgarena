use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use tokio::{
    fs,
    io::AsyncReadExt,
    net::TcpListener,
    process::Command,
    sync::Mutex,
    time::{timeout, Instant},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{db, domain::MatchId, replay_artifact};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_IDLE_LIFETIME: Duration = Duration::from_secs(15 * 60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(30);
const SESSION_DIRECTORY: &str = ".cgarena/replay-sessions";

#[derive(Clone)]
pub struct ReplayViewer {
    inner: Arc<ReplayViewerInner>,
}

struct ReplayViewerInner {
    pool: sqlx::SqlitePool,
    arena_path: PathBuf,
    command: String,
    sessions: Mutex<HashMap<String, ReplaySession>>,
    startup_timeout: Duration,
    idle_lifetime: Duration,
}

struct ReplaySession {
    directory: PathBuf,
    last_accessed: Instant,
}

#[derive(Debug)]
pub struct StartedReplay {
    pub session_id: String,
    pub viewer_url: String,
}

pub struct ReplayAsset {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("match not found")]
    MatchNotFound,
    #[error("this match has no replay artifact")]
    Unavailable,
    #[error("replay artifact is missing or invalid: {0}")]
    InvalidArtifact(String),
    #[error("invalid replay command: {0}")]
    InvalidCommand(String),
    #[error("replay renderer failed to start: {0}")]
    StartupFailed(String),
    #[error("replay renderer startup timed out")]
    StartupTimeout,
    #[error("replay session not found")]
    SessionNotFound,
    #[error("replay asset not found")]
    AssetNotFound,
    #[error("replay subsystem failed: {0}")]
    Internal(String),
}

impl ReplayViewer {
    pub fn new(
        pool: sqlx::SqlitePool,
        arena_path: PathBuf,
        command: String,
        cancellation_token: CancellationToken,
    ) -> Self {
        let viewer = Self::new_with_timeouts(
            pool,
            arena_path,
            command,
            STARTUP_TIMEOUT,
            SESSION_IDLE_LIFETIME,
        );
        viewer.spawn_cleanup_task(cancellation_token);
        viewer
    }

    fn new_with_timeouts(
        pool: sqlx::SqlitePool,
        arena_path: PathBuf,
        command: String,
        startup_timeout: Duration,
        idle_lifetime: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(ReplayViewerInner {
                pool,
                arena_path,
                command,
                sessions: Mutex::new(HashMap::new()),
                startup_timeout,
                idle_lifetime,
            }),
        }
    }

    pub async fn watch(&self, match_id: MatchId) -> Result<StartedReplay, ReplayError> {
        let metadata = db::fetch_replay_metadata(&self.inner.pool, match_id)
            .await
            .map_err(|error| ReplayError::Internal(error.to_string()))?
            .ok_or(ReplayError::MatchNotFound)?;
        let replay_path = metadata.replay_path.ok_or(ReplayError::Unavailable)?;
        let artifact_path = replay_artifact::resolve(&self.inner.arena_path, &replay_path)
            .map_err(|error| ReplayError::InvalidArtifact(error.to_string()))?;
        let artifact_metadata = fs::metadata(&artifact_path)
            .await
            .map_err(|error| ReplayError::InvalidArtifact(error.to_string()))?;
        if !artifact_metadata.is_file() || artifact_metadata.len() == 0 {
            return Err(ReplayError::InvalidArtifact(
                "artifact is not a non-empty file".to_string(),
            ));
        }

        let session_id = Uuid::new_v4().to_string();
        let session_directory = self
            .inner
            .arena_path
            .join(SESSION_DIRECTORY)
            .join(&session_id);
        fs::create_dir_all(&session_directory)
            .await
            .map_err(|error| ReplayError::Internal(error.to_string()))?;

        let result = self
            .generate_bundle(
                &artifact_path,
                &session_directory,
                metadata.participant_count,
            )
            .await;
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&session_directory).await;
            return Err(error);
        }

        self.inner.sessions.lock().await.insert(
            session_id.clone(),
            ReplaySession {
                directory: session_directory,
                last_accessed: Instant::now(),
            },
        );

        Ok(StartedReplay {
            viewer_url: format!("/api/replays/{session_id}/test.html"),
            session_id,
        })
    }

    pub async fn asset(
        &self,
        session_id: &str,
        relative_path: &str,
    ) -> Result<ReplayAsset, ReplayError> {
        let relative_path = if relative_path.is_empty() {
            Path::new("test.html")
        } else {
            Path::new(relative_path)
        };
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(ReplayError::AssetNotFound);
        }

        let path = {
            let mut sessions = self.inner.sessions.lock().await;
            let session = sessions
                .get_mut(session_id)
                .ok_or(ReplayError::SessionNotFound)?;
            session.last_accessed = Instant::now();
            session.directory.join(relative_path)
        };
        let bytes = fs::read(&path).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ReplayError::AssetNotFound
            } else {
                ReplayError::Internal(error.to_string())
            }
        })?;

        Ok(ReplayAsset {
            bytes,
            content_type: mime_guess::from_path(relative_path)
                .first_or_octet_stream()
                .to_string(),
        })
    }

    pub async fn close(&self, session_id: &str) -> Result<(), ReplayError> {
        let session = self.inner.sessions.lock().await.remove(session_id);
        if let Some(session) = session {
            fs::remove_dir_all(session.directory)
                .await
                .map_err(|error| ReplayError::Internal(error.to_string()))?;
        }
        Ok(())
    }

    pub async fn shutdown(&self) {
        let sessions = {
            let mut sessions = self.inner.sessions.lock().await;
            sessions
                .drain()
                .map(|(_, session)| session)
                .collect::<Vec<_>>()
        };
        for session in sessions {
            let _ = fs::remove_dir_all(session.directory).await;
        }
    }

    async fn generate_bundle(
        &self,
        artifact_path: &Path,
        session_directory: &Path,
        participant_count: u8,
    ) -> Result<(), ReplayError> {
        let port = available_local_port().await?;
        let command_parts = replay_command(
            &self.inner.command,
            artifact_path,
            session_directory,
            port,
            participant_count,
        )?;
        let mut child = Command::new(&command_parts[0])
            .args(&command_parts[1..])
            .current_dir(&self.inner.arena_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| ReplayError::StartupFailed(error.to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ReplayError::Internal("cannot capture renderer stderr".to_string()))?;
        let stderr_task = tokio::spawn(read_stderr(stderr));

        let status = match timeout(self.inner.startup_timeout, child.wait()).await {
            Ok(result) => result.map_err(|error| ReplayError::StartupFailed(error.to_string()))?,
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                let _ = stderr_task.await;
                return Err(ReplayError::StartupTimeout);
            }
        };
        let stderr = stderr_task
            .await
            .map_err(|error| ReplayError::Internal(error.to_string()))?
            .map_err(|error| ReplayError::Internal(error.to_string()))?;
        if !status.success() {
            let message = String::from_utf8_lossy(&stderr).trim().to_string();
            return Err(ReplayError::StartupFailed(if message.is_empty() {
                format!("renderer exited with {status}")
            } else {
                message
            }));
        }

        let entrypoint = session_directory.join("test.html");
        if !entrypoint.is_file() {
            return Err(ReplayError::StartupFailed(
                "renderer produced no test.html replay bundle".to_string(),
            ));
        }
        Ok(())
    }

    fn spawn_cleanup_task(&self, cancellation_token: CancellationToken) {
        let viewer = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        viewer.shutdown().await;
                        break;
                    }
                    _ = interval.tick() => viewer.cleanup_expired().await,
                }
            }
        });
    }

    async fn cleanup_expired(&self) {
        let expired = {
            let mut sessions = self.inner.sessions.lock().await;
            let now = Instant::now();
            let expired_ids = sessions
                .iter()
                .filter(|(_, session)| {
                    now.duration_since(session.last_accessed) >= self.inner.idle_lifetime
                })
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            expired_ids
                .into_iter()
                .filter_map(|id| sessions.remove(&id))
                .collect::<Vec<_>>()
        };
        for session in expired {
            let _ = fs::remove_dir_all(session.directory).await;
        }
    }
}

fn replay_command(
    template: &str,
    artifact_path: &Path,
    session_directory: &Path,
    port: u16,
    participant_count: u8,
) -> Result<Vec<String>, ReplayError> {
    let mut parts = shell_words::split(template)
        .map_err(|error| ReplayError::InvalidCommand(error.to_string()))?;
    if parts.is_empty() {
        return Err(ReplayError::InvalidCommand(
            "cmd_watch_replay must not be blank".to_string(),
        ));
    }

    let port = port.to_string();
    let participant_count = participant_count.to_string();
    let replacements = [
        ("{REPLAY_PATH}", artifact_path.to_str()),
        ("{REPLAY_DIR}", session_directory.to_str()),
        ("{PORT}", Some(port.as_str())),
        ("{PLAYER_COUNT}", Some(participant_count.as_str())),
    ];
    for (placeholder, value) in replacements {
        let value = value.ok_or_else(|| {
            ReplayError::InvalidCommand(format!("{placeholder} value is not valid UTF-8"))
        })?;
        let Some(part) = parts.iter_mut().find(|part| part.as_str() == placeholder) else {
            return Err(ReplayError::InvalidCommand(format!(
                "cmd_watch_replay must contain {placeholder}"
            )));
        };
        *part = value.to_string();
    }
    Ok(parts)
}

async fn available_local_port() -> Result<u16, ReplayError> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|error| ReplayError::Internal(error.to_string()))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| ReplayError::Internal(error.to_string()))
}

async fn read_stderr(mut stderr: tokio::process::ChildStderr) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stderr.read_to_end(&mut bytes).await?;
    Ok(bytes)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    async fn fixture(
        script_body: &str,
        startup_timeout: Duration,
    ) -> (TempDir, ReplayViewer, MatchId) {
        let temporary_directory = tempfile::tempdir().unwrap();
        let arena_path = temporary_directory.path().join("arena with spaces");
        fs::create_dir_all(arena_path.join("replays"))
            .await
            .unwrap();
        let artifact_path = PathBuf::from("replays/fixture.json");
        fs::write(arena_path.join(&artifact_path), b"{\"agents\":[{},{}]}")
            .await
            .unwrap();

        let launcher_path = arena_path.join("launcher with spaces.sh");
        fs::write(
            &launcher_path,
            format!("#!/bin/sh\nset -eu\n{script_body}\n"),
        )
        .await
        .unwrap();
        let mut permissions = std::fs::metadata(&launcher_path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&launcher_path, permissions).unwrap();

        let pool = db::connect(&arena_path).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let match_id: MatchId = sqlx::query(
            "INSERT INTO matches (seed, participant_cnt, replay_path) VALUES (7, 2, ?)",
        )
        .bind(artifact_path.to_str().unwrap())
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid()
        .into();
        let command = format!(
            "\"{}\" {{REPLAY_PATH}} {{REPLAY_DIR}} {{PORT}} {{PLAYER_COUNT}}",
            launcher_path.display()
        );
        let viewer = ReplayViewer::new_with_timeouts(
            pool,
            arena_path,
            command,
            startup_timeout,
            Duration::from_secs(60),
        );
        (temporary_directory, viewer, match_id)
    }

    #[tokio::test]
    async fn sessions_are_isolated_and_serve_static_assets() {
        let (_temporary_directory, viewer, match_id) = fixture(
            "mkdir -p \"$2\"\nprintf '<html>fixture replay</html>' > \"$2/test.html\"",
            Duration::from_secs(2),
        )
        .await;

        let (first, second) = tokio::join!(viewer.watch(match_id), viewer.watch(match_id));
        let first = first.unwrap();
        let second = second.unwrap();
        assert_ne!(first.session_id, second.session_id);
        assert!(first.viewer_url.starts_with("/api/replays/"));

        viewer.close(&first.session_id).await.unwrap();
        assert!(matches!(
            viewer.asset(&first.session_id, "test.html").await,
            Err(ReplayError::SessionNotFound)
        ));
        let asset = viewer.asset(&second.session_id, "test.html").await.unwrap();
        assert_eq!(asset.bytes, b"<html>fixture replay</html>");

        viewer.shutdown().await;
        assert!(matches!(
            viewer.asset(&second.session_id, "test.html").await,
            Err(ReplayError::SessionNotFound)
        ));
    }

    #[tokio::test]
    async fn startup_timeout_kills_the_launcher() {
        let (_temporary_directory, viewer, match_id) =
            fixture("sleep 5", Duration::from_millis(50)).await;

        let started = Instant::now();
        assert!(matches!(
            viewer.watch(match_id).await,
            Err(ReplayError::StartupTimeout)
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn launcher_failure_returns_stderr() {
        let (_temporary_directory, viewer, match_id) = fixture(
            "echo actionable failure >&2\nexit 7",
            Duration::from_secs(2),
        )
        .await;

        let error = viewer.watch(match_id).await.unwrap_err();
        assert!(matches!(
            error,
            ReplayError::StartupFailed(message) if message.contains("actionable failure")
        ));
    }

    #[tokio::test]
    async fn missing_match_and_artifact_are_normal_errors() {
        let (_temporary_directory, viewer, match_id) = fixture(
            "mkdir -p \"$2\"\nprintf ok > \"$2/test.html\"",
            Duration::from_secs(2),
        )
        .await;

        assert!(matches!(
            viewer.watch(MatchId::from(i64::from(match_id) + 1)).await,
            Err(ReplayError::MatchNotFound)
        ));
        replay_artifact::remove(&viewer.inner.arena_path, Path::new("replays/fixture.json"))
            .await
            .unwrap();
        assert!(matches!(
            viewer.watch(match_id).await,
            Err(ReplayError::InvalidArtifact(_))
        ));
    }

    #[test]
    fn replay_command_preserves_quoted_paths_and_requires_contract() {
        let command = replay_command(
            "\"renderer path\" {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}",
            Path::new("/artifact path/replay.json"),
            Path::new("/session path"),
            12345,
            4,
        )
        .unwrap();
        assert_eq!(
            command,
            [
                "renderer path",
                "/artifact path/replay.json",
                "/session path",
                "12345",
                "4",
            ]
        );

        assert!(matches!(
            replay_command(
                "renderer {REPLAY_PATH}",
                Path::new("replay.json"),
                Path::new("session"),
                1,
                2,
            ),
            Err(ReplayError::InvalidCommand(_))
        ));
    }
}
