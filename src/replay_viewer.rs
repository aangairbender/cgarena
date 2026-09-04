use crate::config::RefereeConfig;
use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use tokio::{
    fs,
    io::{AsyncBufReadExt, AsyncReadExt, BufReader},
    net::TcpListener,
    process::{Child, Command},
    sync::Mutex,
    time::{timeout, Instant},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[cfg(test)]
use crate::db;
use crate::{
    domain::MatchId,
    replay_artifact::{ReplayArtifacts, ReplayLookupError},
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_IDLE_LIFETIME: Duration = Duration::from_secs(15 * 60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(30);
const SESSION_DIRECTORY: &str = ".cgarena/replay-sessions";

#[derive(Clone)]
pub struct ReplayViewer {
    inner: Arc<ReplayViewerInner>,
}

struct ReplayViewerInner {
    replay_artifacts: ReplayArtifacts,
    arena_path: PathBuf,
    referee: RefereeConfig,
    sessions: Mutex<HashMap<String, ReplaySession>>,
    startup_timeout: Duration,
    idle_lifetime: Duration,
    cancellation_token: CancellationToken,
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
    #[error("replay renderer startup timed out: {0}")]
    StartupTimeout(String),
    #[error("replay session not found")]
    SessionNotFound,
    #[error("replay asset not found")]
    AssetNotFound,
    #[error("replay subsystem failed: {0}")]
    Internal(String),
}

impl From<ReplayLookupError> for ReplayError {
    fn from(error: ReplayLookupError) -> Self {
        match error {
            ReplayLookupError::MatchNotFound => Self::MatchNotFound,
            ReplayLookupError::Unavailable => Self::Unavailable,
            ReplayLookupError::InvalidArtifact(message) => Self::InvalidArtifact(message),
            ReplayLookupError::Internal(message) => Self::Internal(message),
        }
    }
}

impl ReplayViewer {
    pub fn new(
        pool: sqlx::SqlitePool,
        arena_path: PathBuf,
        referee: RefereeConfig,
        cancellation_token: CancellationToken,
    ) -> Self {
        let viewer = Self::new_with_timeouts(
            pool,
            arena_path,
            referee,
            STARTUP_TIMEOUT,
            SESSION_IDLE_LIFETIME,
            cancellation_token.clone(),
        );
        viewer.spawn_cleanup_task(cancellation_token);
        viewer
    }

    fn new_with_timeouts(
        pool: sqlx::SqlitePool,
        arena_path: PathBuf,
        referee: RefereeConfig,
        startup_timeout: Duration,
        idle_lifetime: Duration,
        cancellation_token: CancellationToken,
    ) -> Self {
        let replay_artifacts = ReplayArtifacts::new(pool, arena_path.clone());
        Self {
            inner: Arc::new(ReplayViewerInner {
                replay_artifacts,
                arena_path,
                referee,
                sessions: Mutex::new(HashMap::new()),
                startup_timeout,
                idle_lifetime,
                cancellation_token,
            }),
        }
    }

    pub async fn watch(&self, match_id: MatchId) -> Result<StartedReplay, ReplayError> {
        let artifact = self.inner.replay_artifacts.lookup(match_id).await?;

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
                artifact.path(),
                &session_directory,
                artifact.participant_count(),
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
        if let RefereeConfig::CodingameJar(referee) = &self.inner.referee {
            return self
                .generate_codingame_bundle(
                    artifact_path,
                    session_directory,
                    participant_count,
                    referee,
                )
                .await;
        }
        let port = available_local_port().await?;
        let command_parts = replay_command(
            &self.inner.referee,
            artifact_path,
            session_directory,
            port,
            participant_count,
        )?;
        let mut command = Command::new(&command_parts[0]);
        command
            .args(&command_parts[1..])
            .current_dir(&self.inner.arena_path)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
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
                let _ = terminate_renderer(&mut child).await;
                let stderr = stderr_task
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or_default();
                return Err(ReplayError::StartupTimeout(
                    String::from_utf8_lossy(&stderr).trim().to_string(),
                ));
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
        if !session_directory.join("test.html").is_file() {
            return Err(ReplayError::StartupFailed(
                "renderer produced no test.html replay bundle".to_string(),
            ));
        }
        Ok(())
    }

    async fn generate_codingame_bundle(
        &self,
        artifact_path: &Path,
        session_directory: &Path,
        participant_count: u8,
        referee: &crate::config::CodingameJarRefereeConfig,
    ) -> Result<(), ReplayError> {
        validate_codingame_replay(artifact_path, participant_count).await?;
        let temporary_directory = session_directory.join(format!(".jvm-{}", Uuid::new_v4()));
        fs::create_dir_all(&temporary_directory)
            .await
            .map_err(|error| ReplayError::Internal(error.to_string()))?;
        let temporary_directory = fs::canonicalize(&temporary_directory)
            .await
            .map_err(|error| ReplayError::Internal(error.to_string()))?;
        let result = self
            .run_codingame_renderer(
                artifact_path,
                session_directory,
                &temporary_directory,
                referee,
            )
            .await;
        let _ = fs::remove_dir_all(&temporary_directory).await;
        result
    }

    async fn run_codingame_renderer(
        &self,
        artifact_path: &Path,
        session_directory: &Path,
        temporary_directory: &Path,
        referee: &crate::config::CodingameJarRefereeConfig,
    ) -> Result<(), ReplayError> {
        let port = available_local_port().await?;
        let mut command = Command::new(referee.java.as_deref().unwrap_or("java"));
        command
            .args([
                format!("-Djava.io.tmpdir={}", temporary_directory.display()),
                "--add-opens".to_string(),
                "java.base/java.lang=ALL-UNNAMED".to_string(),
                "-jar".to_string(),
                referee.path.clone(),
                "-r".to_string(),
                artifact_path.to_string_lossy().to_string(),
                "-port".to_string(),
                port.to_string(),
            ])
            .current_dir(&self.inner.arena_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let mut child = command
            .spawn()
            .map_err(|error| ReplayError::StartupFailed(error.to_string()))?;
        let stdout = child
            .stdout
            .take()
            .expect("renderer stdout is configured as piped");
        let stderr = child
            .stderr
            .take()
            .expect("renderer stderr is configured as piped");
        let stderr_task = tokio::spawn(read_stderr(stderr));
        let mut lines = BufReader::new(stdout).lines();
        let exposed_result = tokio::select! {
            _ = self.inner.cancellation_token.cancelled() => {
                Err(ReplayError::StartupFailed("renderer canceled during arena shutdown".to_string()))
            }
            result = timeout(self.inner.startup_timeout, read_exposed_directory(&mut lines)) => {
                result.unwrap_or_else(|_| Err(ReplayError::StartupTimeout(String::new())))
            }
        };
        let termination_result = terminate_renderer(&mut child).await;
        let stderr = stderr_task
            .await
            .map_err(|error| ReplayError::Internal(error.to_string()))?
            .map_err(|error| ReplayError::Internal(error.to_string()))?;
        let exposed = exposed_result.map_err(|error| with_renderer_stderr(error, &stderr))?;
        termination_result.map_err(|error| {
            ReplayError::StartupFailed(format!("cannot reap renderer: {error}"))
        })?;

        let exposed = if exposed.is_absolute() {
            exposed
        } else {
            self.inner.arena_path.join(exposed)
        };
        let exposed = fs::canonicalize(&exposed)
            .await
            .map_err(|error| ReplayError::StartupFailed(error.to_string()))?;
        if exposed == temporary_directory || !exposed.starts_with(temporary_directory) {
            return Err(ReplayError::StartupFailed(
                "renderer exposed a directory outside its temporary directory".to_string(),
            ));
        }
        copy_directory(&exposed, session_directory)
            .map_err(|error| ReplayError::StartupFailed(error.to_string()))?;
        normalize_replay_bundle(session_directory)
            .map_err(|error| ReplayError::StartupFailed(error.to_string()))?;
        if !session_directory.join("test.html").is_file() {
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

async fn validate_codingame_replay(
    artifact_path: &Path,
    participant_count: u8,
) -> Result<(), ReplayError> {
    let bytes = fs::read(artifact_path)
        .await
        .map_err(|error| ReplayError::InvalidArtifact(error.to_string()))?;
    let replay: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| ReplayError::InvalidArtifact(error.to_string()))?;
    let agents = replay
        .get("agents")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ReplayError::InvalidArtifact("artifact has no agents array".to_string()))?;
    if agents.len() != usize::from(participant_count) {
        return Err(ReplayError::InvalidArtifact(format!(
            "artifact has {} participants, match has {participant_count}",
            agents.len()
        )));
    }
    Ok(())
}

async fn read_exposed_directory<R>(lines: &mut tokio::io::Lines<R>) -> Result<PathBuf, ReplayError>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|error| ReplayError::StartupFailed(error.to_string()))?
            .ok_or_else(|| {
                ReplayError::StartupFailed(
                    "renderer exited without producing a replay bundle".to_string(),
                )
            })?;
        if let Some(path) = line.strip_prefix("Exposed web server dir: ") {
            return Ok(PathBuf::from(path.trim()));
        }
    }
}

fn with_renderer_stderr(error: ReplayError, stderr: &[u8]) -> ReplayError {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    match (error, stderr.is_empty()) {
        (ReplayError::StartupFailed(message), false) => {
            ReplayError::StartupFailed(format!("{message}: {stderr}"))
        }
        (ReplayError::StartupTimeout(_), false) => ReplayError::StartupTimeout(stderr),
        (error, _) => error,
    }
}

async fn terminate_renderer(child: &mut Child) -> std::io::Result<()> {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // The renderer is launched in its own process group; a negative PID addresses the group.
        let result = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if result == -1 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
    }
    #[cfg(windows)]
    if let Some(pid) = child.id() {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .await;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = child.start_kill();
    }
    child.wait().await.map(|_| ())
}

fn copy_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "renderer bundle contains a symbolic link",
            ));
        }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
fn normalize_replay_bundle(directory: &Path) -> std::io::Result<()> {
    let assets = directory.join("assets");
    if assets.is_dir() {
        let nested = assets.join("assets");
        std::fs::create_dir_all(&nested)?;
        for entry in std::fs::read_dir(&assets)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "png")
            {
                std::fs::copy(entry.path(), nested.join(entry.file_name()))?;
            }
        }
    }
    let app = directory.join("app.js");
    if app.is_file() {
        let content = std::fs::read_to_string(&app)?
            .replace("from '../config.js'", "from './config.js'")
            .replace("from '../demo.js'", "from './demo.js'")
            .replace(
                "viewerUrl: '/core/Drawer.js'",
                "viewerUrl: './core/Drawer.js'",
            );
        std::fs::write(app, content)?;
    }
    Ok(())
}

fn replay_command(
    referee: &RefereeConfig,
    artifact_path: &Path,
    session_directory: &Path,
    port: u16,
    participant_count: u8,
) -> Result<Vec<String>, ReplayError> {
    let RefereeConfig::Command(referee) = referee else {
        return Err(ReplayError::InvalidCommand(
            "codingame_jar replay rendering is not implemented".to_string(),
        ));
    };
    let mut parts = shell_words::split(&referee.watch_replay)
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
    use crate::{
        domain::{BotId, Match, Participant},
        replay_artifact::ProvisionalReplay,
    };
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    async fn fixture(
        script_body: &str,
        startup_timeout: Duration,
    ) -> (TempDir, ReplayViewer, MatchId, PathBuf) {
        let temporary_directory = tempfile::tempdir().unwrap();
        let arena_path = temporary_directory.path().join("arena with spaces");
        let provisional = ProvisionalReplay::create(&arena_path).await.unwrap();
        let artifact_path = arena_path.join(provisional.command_path());
        fs::write(&artifact_path, b"{\"agents\":[{},{}]}")
            .await
            .unwrap();
        let pending = provisional.finish().await.unwrap();

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
        let mut bot_ids = Vec::new();
        for name in ["one", "two"] {
            let bot_id: BotId = sqlx::query(
                "INSERT INTO bots (name, source_code, language, created_at) VALUES (?, '', 'rust', 0)",
            )
            .bind(name)
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_rowid()
            .into();
            bot_ids.push(bot_id);
        }
        let mut replay_match = Match::new(
            7,
            bot_ids
                .into_iter()
                .enumerate()
                .map(|(rank, bot_id)| Participant {
                    bot_id,
                    rank: rank as u8,
                    error: false,
                })
                .collect(),
            vec![],
            None,
        );
        ReplayArtifacts::new(pool.clone(), arena_path.clone())
            .persist_match(pending, &mut replay_match)
            .await
            .unwrap();
        let command = format!(
            "\"{}\" {{REPLAY_PATH}} {{REPLAY_DIR}} {{PORT}} {{PLAYER_COUNT}}",
            launcher_path.display()
        );
        let viewer = ReplayViewer::new_with_timeouts(
            pool,
            arena_path,
            RefereeConfig::Command(crate::config::CommandRefereeConfig {
                play_match: "true".to_string(),
                watch_replay: command,
            }),
            startup_timeout,
            Duration::from_secs(60),
            CancellationToken::new(),
        );
        (temporary_directory, viewer, replay_match.id, artifact_path)
    }
    async fn assert_process_gone(pid_path: &Path) {
        let pid: i32 = std::fs::read_to_string(pid_path)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        for _ in 0..100 {
            if unsafe { libc::kill(pid, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("process {pid} was not reaped");
    }

    async fn codingame_fixture(
        script_body: &str,
        replay: &str,
        startup_timeout: Duration,
    ) -> (TempDir, ReplayViewer, PathBuf, PathBuf, CancellationToken) {
        let temporary_directory = tempfile::tempdir().unwrap();
        let arena_path = temporary_directory.path().join("codingame arena");
        fs::create_dir_all(&arena_path).await.unwrap();
        let artifact_path = arena_path.join("replay.json");
        fs::write(&artifact_path, replay).await.unwrap();
        let session_directory = arena_path.join("session");
        fs::create_dir_all(&session_directory).await.unwrap();
        let launcher_path = arena_path.join("fake java.sh");
        fs::write(
            &launcher_path,
            format!(
                "#!/bin/sh\nset -eu\ntmp=''\nfor argument in \"$@\"; do\n  case \"$argument\" in\n    -Djava.io.tmpdir=*) tmp=\"${{argument#*=}}\" ;;\n  esac\ndone\n{script_body}\n"
            ),
        )
        .await
        .unwrap();
        let mut permissions = std::fs::metadata(&launcher_path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&launcher_path, permissions).unwrap();
        let cancellation_token = CancellationToken::new();
        let viewer = ReplayViewer::new_with_timeouts(
            db::in_memory().await.unwrap(),
            arena_path,
            RefereeConfig::CodingameJar(crate::config::CodingameJarRefereeConfig {
                path: "fixture.jar".to_string(),
                java: Some(launcher_path.to_string_lossy().to_string()),
                league: None,
            }),
            startup_timeout,
            Duration::from_secs(60),
            cancellation_token.clone(),
        );
        (
            temporary_directory,
            viewer,
            artifact_path,
            session_directory,
            cancellation_token,
        )
    }

    #[tokio::test]
    async fn sessions_are_isolated_and_serve_static_assets() {
        let (_temporary_directory, viewer, match_id, _artifact_path) = fixture(
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
        let (_temporary_directory, viewer, match_id, _artifact_path) =
            fixture("sleep 5", Duration::from_millis(50)).await;

        let started = Instant::now();
        assert!(matches!(
            viewer.watch(match_id).await,
            Err(ReplayError::StartupTimeout(_))
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn launcher_failure_returns_stderr() {
        let (_temporary_directory, viewer, match_id, _artifact_path) = fixture(
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
        let (_temporary_directory, viewer, match_id, artifact_path) = fixture(
            "mkdir -p \"$2\"\nprintf ok > \"$2/test.html\"",
            Duration::from_secs(2),
        )
        .await;

        assert!(matches!(
            viewer.watch(MatchId::from(i64::from(match_id) + 1)).await,
            Err(ReplayError::MatchNotFound)
        ));
        fs::remove_file(artifact_path).await.unwrap();
        assert!(matches!(
            viewer.watch(match_id).await,
            Err(ReplayError::InvalidArtifact(_))
        ));
    }

    #[tokio::test]
    async fn codingame_renderer_normalizes_bundle_and_reaps_process() {
        let (temporary_directory, viewer, artifact, session, _cancellation_token) =
            codingame_fixture(
            r#"mkdir -p "$tmp/codingame/assets"
printf '<html>native replay</html>' > "$tmp/codingame/test.html"
printf png > "$tmp/codingame/assets/image.png"
printf "from '../config.js'; from '../demo.js'; viewerUrl: '/core/Drawer.js'" > "$tmp/codingame/app.js"
echo $$ > renderer.pid
echo "Exposed web server dir: $tmp/codingame"
exec sleep 30"#,
            r#"{"agents":[{},{}]}"#,
            Duration::from_secs(2),
        )
        .await;

        viewer
            .generate_bundle(&artifact, &session, 2)
            .await
            .unwrap();

        assert_eq!(
            fs::read(session.join("test.html")).await.unwrap(),
            b"<html>native replay</html>"
        );
        assert!(session.join("assets/assets/image.png").is_file());
        let app = fs::read_to_string(session.join("app.js")).await.unwrap();
        assert!(app.contains("from './config.js'"));
        assert!(app.contains("from './demo.js'"));
        assert!(app.contains("viewerUrl: './core/Drawer.js'"));
        assert!(!std::fs::read_dir(&session).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".jvm-")
        }));
        let pid: i32 = std::fs::read_to_string(
            temporary_directory
                .path()
                .join("codingame arena/renderer.pid"),
        )
        .unwrap()
        .trim()
        .parse()
        .unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[tokio::test]
    async fn codingame_replay_rejects_participant_mismatch_before_launch() {
        let (temporary_directory, viewer, artifact, session, _cancellation_token) =
            codingame_fixture(
                "touch renderer-started",
                r#"{"agents":[{}]}"#,
                Duration::from_secs(2),
            )
            .await;

        assert!(matches!(
            viewer.generate_bundle(&artifact, &session, 2).await,
            Err(ReplayError::InvalidArtifact(message)) if message.contains("1 participants")
        ));
        assert!(!temporary_directory
            .path()
            .join("codingame arena/renderer-started")
            .exists());
    }

    #[tokio::test]
    async fn codingame_renderer_reports_stderr_on_timeout_and_is_reaped() {
        let (temporary_directory, viewer, artifact, session, _cancellation_token) =
            codingame_fixture(
            "echo $$ > renderer.pid\nsleep 30 &\necho $! > renderer-child.pid\necho actionable-timeout >&2\nwait",
            r#"{"agents":[{},{}]}"#,
            Duration::from_secs(1),
        )
        .await;

        let result = viewer.generate_bundle(&artifact, &session, 2).await;
        assert!(
            matches!(
                &result,
                Err(ReplayError::StartupTimeout(message))
                    if message.contains("actionable-timeout")
            ),
            "unexpected result: {result:?}"
        );
        assert_process_gone(
            &temporary_directory
                .path()
                .join("codingame arena/renderer.pid"),
        )
        .await;
        assert_process_gone(
            &temporary_directory
                .path()
                .join("codingame arena/renderer-child.pid"),
        )
        .await;
        assert!(!std::fs::read_dir(&session).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".jvm-")
        }));
    }

    #[tokio::test]
    async fn codingame_renderer_is_reaped_on_cancellation() {
        let (temporary_directory, viewer, artifact, session, cancellation_token) =
            codingame_fixture(
                "echo $$ > renderer.pid\nsleep 30 &\necho $! > renderer-child.pid\nwait",
                r#"{"agents":[{},{}]}"#,
                Duration::from_secs(30),
            )
            .await;
        let renderer_pid = temporary_directory
            .path()
            .join("codingame arena/renderer.pid");
        let renderer_child_pid = temporary_directory
            .path()
            .join("codingame arena/renderer-child.pid");
        let start_marker = renderer_child_pid.clone();
        let cancellation = tokio::spawn(async move {
            for _ in 0..200 {
                if start_marker.exists() {
                    cancellation_token.cancel();
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("renderer did not start");
        });

        let result = viewer.generate_bundle(&artifact, &session, 2).await;
        cancellation.await.unwrap();

        assert!(matches!(
            result,
            Err(ReplayError::StartupFailed(message))
                if message.contains("canceled during arena shutdown")
        ));
        assert_process_gone(&renderer_pid).await;
        assert_process_gone(&renderer_child_pid).await;
        assert!(!std::fs::read_dir(&session).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".jvm-")
        }));
    }
    #[tokio::test]
    async fn codingame_renderer_returns_early_stderr() {
        let (_temporary_directory, viewer, artifact, session, _cancellation_token) =
            codingame_fixture(
                "echo actionable-failure >&2\nexit 7",
                r#"{"agents":[{},{}]}"#,
                Duration::from_secs(2),
            )
            .await;

        assert!(matches!(
            viewer.generate_bundle(&artifact, &session, 2).await,
            Err(ReplayError::StartupFailed(message)) if message.contains("actionable-failure")
        ));
    }

    #[tokio::test]
    async fn codingame_renderer_rejects_exposed_directory_escape() {
        let (_temporary_directory, viewer, artifact, session, _cancellation_token) =
            codingame_fixture(
                "mkdir -p outside\necho \"Exposed web server dir: $(pwd)/outside\"\nexec sleep 30",
                r#"{"agents":[{},{}]}"#,
                Duration::from_secs(2),
            )
            .await;

        assert!(matches!(
            viewer.generate_bundle(&artifact, &session, 2).await,
            Err(ReplayError::StartupFailed(message)) if message.contains("outside")
        ));
    }

    #[test]
    fn replay_command_preserves_quoted_paths_and_requires_contract() {
        let command = replay_command(
            &RefereeConfig::Command(crate::config::CommandRefereeConfig {
                play_match: "true".to_string(),
                watch_replay: "\"renderer path\" {REPLAY_PATH} {REPLAY_DIR} {PORT} {PLAYER_COUNT}"
                    .to_string(),
            }),
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
                &RefereeConfig::Command(crate::config::CommandRefereeConfig {
                    play_match: "true".to_string(),
                    watch_replay: "renderer {REPLAY_PATH}".to_string(),
                }),
                Path::new("replay.json"),
                Path::new("session"),
                1,
                2,
            ),
            Err(ReplayError::InvalidCommand(_))
        ));
    }
}
