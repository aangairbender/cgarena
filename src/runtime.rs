use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context};
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

use crate::{
    arena,
    arena_handle::ArenaHandle,
    config::{ArenaConfig, WorkerConfig},
    db,
    managed_referee::{
        ActionOutcome, ManagedReferee, PreparedCandidate, RefereeAction, RefereeStatus,
    },
    referee_adapter::RefereeAdapter,
    replay_viewer::ReplayViewer,
    worker::{self, WorkerSupervisor},
};

#[derive(Clone)]
pub struct ArenaRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    pool: sqlx::SqlitePool,
    arena_path: PathBuf,
    state: RwLock<RuntimeState>,
    operation: Mutex<()>,
    managed_referee: ManagedReferee,
}

#[derive(Default)]
struct RuntimeState {
    active: Option<ActiveRuntime>,
    last_error: Option<String>,
}

struct ActiveRuntime {
    arena_handle: ArenaHandle,
    replay_viewer: ReplayViewer,
    arena_task: JoinHandle<anyhow::Result<()>>,
    worker_supervisor: WorkerSupervisor,
    cancellation_token: CancellationToken,
}

impl ArenaRuntime {
    pub fn new(pool: sqlx::SqlitePool, arena_path: PathBuf) -> Self {
        let managed_referee = ManagedReferee::new(arena_path.clone());
        Self {
            inner: Arc::new(RuntimeInner {
                pool,
                arena_path,
                state: RwLock::new(RuntimeState::default()),
                operation: Mutex::new(()),
                managed_referee,
            }),
        }
    }

    pub async fn start_saved(&self, config: ArenaConfig) {
        let _operation = self.inner.operation.lock().await;
        match self.build(config).await {
            Ok(active) => {
                let mut state = self.inner.state.write().await;
                state.active = Some(active);
                state.last_error = None;
            }
            Err(error) => {
                self.inner.state.write().await.last_error = Some(format!("{error:#}"));
            }
        }
    }

    pub async fn apply(&self, candidate: ArenaConfig) -> anyhow::Result<()> {
        candidate.validate()?;
        let _operation = self.inner.operation.lock().await;
        let previous_config = db::fetch_arena_config(&self.inner.pool).await?;

        if previous_config.is_none() {
            db::persist_arena_config(&self.inner.pool, &candidate).await?;
            match self.build(candidate).await {
                Ok(active) => {
                    let mut state = self.inner.state.write().await;
                    state.active = Some(active);
                    state.last_error = None;
                }
                Err(error) => {
                    self.inner.state.write().await.last_error = Some(format!("{error:#}"));
                }
            }
            return Ok(());
        }
        if let Some(selected) = managed_configuration(&candidate) {
            if !self
                .inner
                .managed_referee
                .selected_is_installed(selected)
                .await?
            {
                db::persist_arena_config(&self.inner.pool, &candidate).await?;
                self.inner.state.write().await.last_error = Some(
                    "The selected managed referee is not installed; use Install referee or Replace referee"
                        .to_string(),
                );
                return Ok(());
            }
        }
        self.validate_runtime_prerequisites(&candidate, true)
            .await?;

        let previous = self.inner.state.write().await.active.take();
        if let Some(previous) = previous {
            previous.drain_and_shutdown().await?;
        }

        match self.build(candidate.clone()).await {
            Ok(active) => {
                if let Err(error) = db::persist_arena_config(&self.inner.pool, &candidate).await {
                    active.shutdown().await?;
                    if let Some(previous_config) = previous_config {
                        self.restore(previous_config).await;
                    }
                    return Err(error).context("Cannot persist activated arena configuration");
                }
                let mut state = self.inner.state.write().await;
                state.active = Some(active);
                state.last_error = None;
                Ok(())
            }
            Err(error) => {
                if let Some(previous_config) = previous_config {
                    self.restore(previous_config).await;
                }
                Err(error).context("Cannot activate arena configuration")
            }
        }
    }

    pub async fn arena_handle(&self) -> Option<ArenaHandle> {
        self.inner
            .state
            .read()
            .await
            .active
            .as_ref()
            .map(|active| active.arena_handle.clone())
    }

    pub async fn replay_viewer(&self) -> Option<ReplayViewer> {
        self.inner
            .state
            .read()
            .await
            .active
            .as_ref()
            .map(|active| active.replay_viewer.clone())
    }

    pub async fn is_available(&self) -> bool {
        self.inner.state.read().await.active.is_some()
    }

    pub async fn last_error(&self) -> Option<String> {
        self.inner.state.read().await.last_error.clone()
    }
    pub async fn managed_referee_status(&self) -> anyhow::Result<RefereeStatus> {
        let selected = db::fetch_arena_config(&self.inner.pool)
            .await?
            .as_ref()
            .and_then(managed_configuration)
            .cloned();
        self.inner.managed_referee.status(selected).await
    }

    pub async fn start_managed_referee_action(&self, action: RefereeAction) -> anyhow::Result<()> {
        self.inner.managed_referee.reserve(action).await?;
        let runtime = self.clone();
        tokio::spawn(async move {
            let result = runtime.run_managed_referee_action(action).await;
            match result {
                Ok(diagnostic) => runtime.inner.managed_referee.finish(diagnostic).await,
                Err(error) => runtime.inner.managed_referee.fail(&error).await,
            }
        });
        Ok(())
    }

    async fn run_managed_referee_action(&self, action: RefereeAction) -> anyhow::Result<String> {
        let _operation = self.inner.operation.lock().await;
        let config = db::fetch_arena_config(&self.inner.pool)
            .await?
            .context("configure a managed CodinGame referee first")?;
        let selected = managed_configuration(&config)
            .context("the active configuration does not select a managed CodinGame referee")?;
        match self.inner.managed_referee.execute(action, selected).await? {
            ActionOutcome::Status(diagnostic) => Ok(diagnostic),
            ActionOutcome::Candidate(candidate) => {
                let diagnostic = candidate.diagnostic.clone();
                self.activate_managed_candidate(config, candidate).await?;
                Ok(diagnostic)
            }
        }
    }

    async fn activate_managed_candidate(
        &self,
        config: ArenaConfig,
        candidate: PreparedCandidate,
    ) -> anyhow::Result<()> {
        self.inner
            .managed_referee
            .phase("draining active matches")
            .await;
        let previous_config = db::fetch_arena_config(&self.inner.pool).await?;
        let previous_metadata = self.inner.managed_referee.metadata_snapshot().await?;
        let previous_runtime = self.inner.state.write().await.active.take();
        let matchmaking_intent = if let Some(previous) = previous_runtime {
            let intent = previous
                .arena_handle
                .fetch_status()
                .await
                .context("Cannot read matchmaking intent")?
                .matchmaking_enabled;
            if let Err(error) = previous.drain_and_shutdown().await {
                if let Some(mut previous_config) = previous_config {
                    set_matchmaking_intent(&mut previous_config, intent);
                    self.restore(previous_config).await;
                }
                return Err(error).context("Cannot drain the active arena runtime");
            }
            Some(intent)
        } else {
            None
        };

        let internal = self.inner.managed_referee.internal_path();
        fs::create_dir_all(&internal)?;
        let prepared_artifact = internal.join(format!("candidate-{}.jar", uuid::Uuid::new_v4()));
        fs::copy(&candidate.artifact, &prepared_artifact).with_context(|| {
            format!(
                "Cannot stage referee artifact {}",
                candidate.artifact.display()
            )
        })?;
        let artifact = self.inner.managed_referee.artifact_path();
        let artifact_backup = artifact
            .is_file()
            .then(|| internal.join(format!("previous-{}.jar", uuid::Uuid::new_v4())));
        if let Some(backup) = &artifact_backup {
            fs::rename(&artifact, backup)?;
        }

        let checkout = self.inner.managed_referee.checkout_path();
        let candidate_root = candidate
            .checkout
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_owned);
        let checkout_backup = candidate.checkout.as_ref().and_then(|_| {
            checkout.is_dir().then(|| {
                self.inner
                    .arena_path
                    .join(format!("referee.previous-{}", uuid::Uuid::new_v4()))
            })
        });
        if let Some(backup) = &checkout_backup {
            if let Err(error) = fs::rename(&checkout, backup) {
                restore_path(artifact_backup.as_deref(), &artifact);
                return Err(error).context("Cannot stage the previous managed referee checkout");
            }
        }
        if let Some(candidate_checkout) = &candidate.checkout {
            if let Err(error) = fs::rename(candidate_checkout, &checkout) {
                restore_path(artifact_backup.as_deref(), &artifact);
                if let Some(backup) = &checkout_backup {
                    let _ = fs::rename(backup, &checkout);
                }
                return Err(error).context("Cannot publish managed referee checkout");
            }
        }
        if let Err(error) = fs::rename(&prepared_artifact, &artifact) {
            if let Some(candidate_checkout) = &candidate.checkout {
                let _ = fs::rename(&checkout, candidate_checkout);
            }
            if let Some(backup) = &checkout_backup {
                let _ = fs::rename(backup, &checkout);
            }
            restore_path(artifact_backup.as_deref(), &artifact);
            return Err(error).context("Cannot publish managed referee artifact");
        }

        let mut activation_config = config.clone();
        if let Some(intent) = matchmaking_intent {
            set_matchmaking_intent(&mut activation_config, intent);
        }
        self.inner.managed_referee.phase("activating referee").await;
        match self.build_inner(activation_config, false).await {
            Ok(active) => {
                let persistence = async {
                    self.inner
                        .managed_referee
                        .publish_metadata(&candidate.metadata)
                        .await?;
                    db::persist_arena_config(&self.inner.pool, &config).await
                }
                .await;
                if let Err(error) = persistence {
                    active.shutdown().await.ok();
                    if artifact.is_file() {
                        let _ = fs::remove_file(&artifact);
                    }
                    restore_path(artifact_backup.as_deref(), &artifact);
                    if candidate.checkout.is_some() && checkout.is_dir() {
                        let _ = fs::remove_dir_all(&checkout);
                    }
                    if let Some(backup) = &checkout_backup {
                        let _ = fs::rename(backup, &checkout);
                    }
                    let metadata_restore = self
                        .inner
                        .managed_referee
                        .restore_metadata(previous_metadata)
                        .await;
                    if let Some(mut previous_config) = previous_config {
                        if let Some(intent) = matchmaking_intent {
                            set_matchmaking_intent(&mut previous_config, intent);
                        }
                        self.restore(previous_config).await;
                    }
                    if let Some(candidate_root) = candidate_root {
                        let _ = fs::remove_dir_all(candidate_root);
                    }
                    metadata_restore.context("Cannot restore managed referee metadata")?;
                    return Err(error).context("Cannot persist activated managed referee");
                }
                let mut state = self.inner.state.write().await;
                state.active = Some(active);
                state.last_error = None;
                if let Some(backup) = artifact_backup {
                    let _ = fs::remove_file(backup);
                }
                if let Some(backup) = checkout_backup {
                    let _ = fs::remove_dir_all(backup);
                }
                if let Some(candidate_root) = candidate_root {
                    let _ = fs::remove_dir_all(candidate_root);
                }
                Ok(())
            }
            Err(error) => {
                if artifact.is_file() {
                    let _ = fs::remove_file(&artifact);
                }
                restore_path(artifact_backup.as_deref(), &artifact);
                if candidate.checkout.is_some() && checkout.is_dir() {
                    let _ = fs::remove_dir_all(&checkout);
                }
                if let Some(backup) = checkout_backup {
                    let _ = fs::rename(backup, &checkout);
                }
                if let Some(mut previous_config) = previous_config {
                    if let Some(intent) = matchmaking_intent {
                        set_matchmaking_intent(&mut previous_config, intent);
                    }
                    self.restore(previous_config).await;
                }
                if let Some(candidate_root) = candidate_root {
                    let _ = fs::remove_dir_all(candidate_root);
                }
                Err(error).context("Cannot activate managed referee candidate")
            }
        }
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let _operation = self.inner.operation.lock().await;
        if let Some(active) = self.inner.state.write().await.active.take() {
            active.shutdown().await?;
        }
        Ok(())
    }

    async fn restore(&self, config: ArenaConfig) {
        match self.build(config).await {
            Ok(active) => {
                self.inner.state.write().await.active = Some(active);
            }
            Err(error) => {
                self.inner.state.write().await.last_error = Some(format!(
                    "Configuration activation failed and the previous runtime could not be restored: {error:#}"
                ));
            }
        }
    }

    async fn validate_runtime_prerequisites(
        &self,
        config: &ArenaConfig,
        require_installed_identity: bool,
    ) -> anyhow::Result<()> {
        let [WorkerConfig::Embedded(worker_config)] = config.workers.as_slice() else {
            bail!("exactly one embedded worker must be configured");
        };
        if require_installed_identity {
            if let Some(selected) = managed_configuration(config) {
                if !self
                    .inner
                    .managed_referee
                    .selected_is_installed(selected)
                    .await?
                {
                    bail!("the selected managed referee is not installed");
                }
            }
        }
        RefereeAdapter::from(&worker_config.referee)
            .validate_startup(&self.inner.arena_path)
            .await
            .context("Referee is unavailable")
    }

    async fn build(&self, config: ArenaConfig) -> anyhow::Result<ActiveRuntime> {
        self.build_inner(config, true).await
    }

    async fn build_inner(
        &self,
        config: ArenaConfig,
        require_installed_identity: bool,
    ) -> anyhow::Result<ActiveRuntime> {
        let [WorkerConfig::Embedded(worker_config)] = config.workers.as_slice() else {
            bail!("exactly one embedded worker must be configured");
        };
        self.validate_runtime_prerequisites(&config, require_installed_identity)
            .await?;

        let referee = RefereeAdapter::from(&worker_config.referee);
        let worker::StartedWorker {
            worker,
            supervisor: worker_supervisor,
        } = worker::start_embedded_worker_with_referee(
            &self.inner.arena_path,
            worker_config.clone(),
            referee.clone(),
        )
        .context("Cannot start embedded worker")?;
        let cancellation_token = CancellationToken::new();
        let replay_viewer = ReplayViewer::new(
            self.inner.pool.clone(),
            self.inner.arena_path.clone(),
            referee,
            cancellation_token.clone(),
        );
        let (arena_tx, arena_rx) = tokio::sync::mpsc::channel(16);
        let arena_task = match arena::run(
            config.game.clone(),
            config.matchmaking.clone(),
            config.leaderboards.clone(),
            config.ranking.clone(),
            self.inner.pool.clone(),
            self.inner.arena_path.clone(),
            worker,
            arena_rx,
            cancellation_token.clone(),
        )
        .await
        {
            Ok(task) => task,
            Err(error) => {
                cancellation_token.cancel();
                replay_viewer.shutdown().await;
                worker_supervisor.shutdown().await.ok();
                return Err(error).context("Cannot start arena");
            }
        };

        Ok(ActiveRuntime {
            arena_handle: ArenaHandle::new(arena_tx),
            replay_viewer,
            arena_task,
            worker_supervisor,
            cancellation_token,
        })
    }
}

fn set_matchmaking_intent(config: &mut ArenaConfig, enabled: bool) {
    config.matchmaking.enabled_on_start = Some(enabled);
}

fn managed_configuration(
    config: &ArenaConfig,
) -> Option<&crate::config::ManagedCodingameRefereeConfig> {
    let [WorkerConfig::Embedded(worker)] = config.workers.as_slice() else {
        return None;
    };
    match &worker.referee {
        crate::config::RefereeConfig::ManagedCodingame(config) => Some(config),
        crate::config::RefereeConfig::Command(_) => None,
    }
}

fn restore_path(backup: Option<&Path>, destination: &Path) {
    if let Some(backup) = backup {
        let _ = fs::rename(backup, destination);
    }
}

impl ActiveRuntime {
    async fn drain_and_shutdown(self) -> anyhow::Result<()> {
        self.arena_handle
            .enable_matchmaking(false)
            .await
            .context("Cannot pause matchmaking")?;
        self.worker_supervisor
            .wait_until_idle()
            .await
            .context("Cannot drain embedded worker")?;
        self.shutdown().await
    }

    async fn shutdown(self) -> anyhow::Result<()> {
        self.cancellation_token.cancel();
        let worker_result = self.worker_supervisor.shutdown().await;
        self.replay_viewer.shutdown().await;
        let arena_result = self.arena_task.await;
        worker_result.context("Embedded worker shutdown failed")?;
        arena_result.context("Arena task terminated unexpectedly")??;
        Ok(())
    }
}
