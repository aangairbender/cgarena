use crate::config::EmbeddedWorkerConfig;
#[cfg(test)]
use crate::config::RefereeConfig;
use crate::domain::{
    BotId, Build, BuildResult, Language, MatchAttribute, MatchAttributeValue, Participant,
    SourceCode, WorkerName,
};
use crate::referee_adapter::{
    read_codingame_match_result, validate_match_result, MatchCommandAttribute as CmdMatchAttribute,
    MatchCommandOutput as CmdPlayMatchStdout, MatchResultSource, PreparedMatchCommand,
    RefereeAdapter,
};
use crate::replay_artifact::{PendingReplay, ProvisionalReplay};
use anyhow::{bail, Context};
use itertools::Itertools;
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output, Stdio};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{oneshot, watch, Mutex};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

const DIR_BOTS: &str = "bots";

/// The embedded worker's caller-facing execution interface.
///
/// Admission is bounded and waits for capacity. Completions are delivered in
/// completion order. Queue topology, process ownership, and task lifecycle stay
/// inside the worker module.
pub struct Worker {
    work_tx: Sender<Work>,
    completion_rx: Mutex<Receiver<Completion>>,
    state_rx: watch::Receiver<WorkerState>,
    available_bot_ids: HashSet<BotId>,
}

/// Owns worker failure observation and orderly shutdown.
///
/// The server must retain this value and call [`WorkerSupervisor::shutdown`]
/// before exiting.
pub struct WorkerSupervisor {
    cancellation_token: CancellationToken,
    state_rx: watch::Receiver<WorkerState>,
    work_tx: Sender<Work>,
    task: Option<JoinHandle<Result<(), WorkerFailure>>>,
}

/// The two ownership views returned when the embedded worker starts.
pub struct StartedWorker {
    pub worker: Worker,
    pub supervisor: WorkerSupervisor,
}

/// Work accepted by the embedded worker.
pub enum Work {
    Build(BuildBotInput),
    Match(PlayMatchInput),
    Drain(oneshot::Sender<()>),
}

/// A completed worker operation.
pub enum Completion {
    Build(BuildBotOutput),
    Match {
        input: PlayMatchInput,
        output: PlayMatchOutput,
    },
}

/// Durable identity used to correlate a build with its worker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildKey {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
}

/// Startup build-state changes computed from the worker's private artifacts.
pub struct BuildReconciliation {
    reset_builds: Vec<BuildKey>,
}

impl BuildReconciliation {
    pub fn into_reset_builds(self) -> Vec<BuildKey> {
        self.reset_builds
    }
}

/// A terminal worker coordination or match-execution failure.
#[derive(Clone, Debug, thiserror::Error)]
#[error("{message}")]
pub struct WorkerFailure {
    message: Arc<str>,
}

impl WorkerFailure {
    fn new(error: impl std::fmt::Display) -> Self {
        Self {
            message: error.to_string().into(),
        }
    }
}

/// Why the worker can no longer accept or complete work.
#[derive(Clone, Debug, thiserror::Error)]
pub enum WorkerUnavailable {
    #[error("worker is shutting down")]
    ShuttingDown,
    #[error("worker failed: {0}")]
    Failed(WorkerFailure),
}

#[derive(Clone, Debug)]
enum WorkerState {
    Running,
    Stopping,
    Stopped,
    Failed(WorkerFailure),
}

/// Starts the concrete embedded worker and all supervised execution tasks.
#[cfg(test)]
pub fn start_embedded_worker(
    worker_path: &Path,
    config: EmbeddedWorkerConfig,
) -> anyhow::Result<StartedWorker> {
    let referee = RefereeAdapter::from(&config.referee);
    start_embedded_worker_with_referee(worker_path, config, referee)
}

pub fn start_embedded_worker_with_referee(
    worker_path: &Path,
    config: EmbeddedWorkerConfig,
    referee: RefereeAdapter,
) -> anyhow::Result<StartedWorker> {
    if config.threads == 0 {
        bail!("embedded worker must have at least one thread");
    }

    let available_bot_ids = known_bot_ids(worker_path)?.into_iter().collect();
    let capacity = usize::from(config.threads) * 2 + 1;
    let (work_tx, work_rx) = channel(capacity);
    let (completion_tx, completion_rx) = channel(capacity);
    let (state_tx, state_rx) = watch::channel(WorkerState::Running);
    let cancellation_token = CancellationToken::new();
    let task = tokio::spawn(run_worker(
        worker_path.to_path_buf(),
        Arc::new(config),
        Arc::new(referee),
        work_rx,
        completion_tx,
        state_tx,
        cancellation_token.clone(),
    ));

    Ok(StartedWorker {
        worker: Worker {
            work_tx: work_tx.clone(),
            completion_rx: Mutex::new(completion_rx),
            state_rx: state_rx.clone(),
            available_bot_ids,
        },
        supervisor: WorkerSupervisor {
            cancellation_token,
            state_rx,
            work_tx,
            task: Some(task),
        },
    })
}

impl Worker {
    /// Returns the persisted builds that must be reset without exposing the
    /// worker's filesystem inventory.
    pub fn reconcile_builds(&self, builds: &[Build]) -> BuildReconciliation {
        let reset_builds = builds
            .iter()
            .filter(|build| {
                build.is_running()
                    || (build.was_finished_successfully()
                        && !self.available_bot_ids.contains(&build.bot_id))
            })
            .map(|build| BuildKey {
                bot_id: build.bot_id,
                worker_name: build.worker_name.clone(),
            })
            .collect();
        BuildReconciliation { reset_builds }
    }

    /// Waits until `work` is admitted to the bounded worker backlog.
    ///
    /// Canceling the returned future before it resolves does not submit the
    /// work. After `Ok(())`, the worker owns the work.
    pub fn submit(
        &self,
        work: Work,
    ) -> impl Future<Output = Result<(), WorkerUnavailable>> + Send + 'static {
        let work_tx = self.work_tx.clone();
        let mut state_rx = self.state_rx.clone();
        async move {
            if let Some(unavailable) = unavailable_state(&state_rx.borrow()) {
                return Err(unavailable);
            }

            tokio::select! {
                biased;
                unavailable = wait_until_unavailable(&mut state_rx) => Err(unavailable),
                result = work_tx.send(work) => result.map_err(|_| {
                    unavailable_state(&state_rx.borrow()).unwrap_or(WorkerUnavailable::ShuttingDown)
                }),
            }
        }
    }

    /// Waits for one completion without exposing or draining an internal
    /// receiver. Only one `next` call may be outstanding.
    pub async fn next(&self) -> Result<Completion, WorkerUnavailable> {
        let mut completion_rx = self.completion_rx.lock().await;
        let mut state_rx = self.state_rx.clone();

        if let Some(unavailable) = unavailable_state(&state_rx.borrow()) {
            return Err(unavailable);
        }

        tokio::select! {
            completion = completion_rx.recv() => match completion {
                Some(completion) => Ok(completion),
                None => Err(
                    unavailable_state(&state_rx.borrow())
                        .unwrap_or_else(|| WorkerUnavailable::Failed(WorkerFailure::new(
                            "worker completion stream closed unexpectedly",
                        ))),
                ),
            },
            unavailable = wait_until_unavailable(&mut state_rx) => Err(unavailable),
        }
    }
}

impl WorkerSupervisor {
    #[cfg(test)]
    /// Resolves when the worker enters a terminal failed state.
    pub async fn failed(&self) -> WorkerFailure {
        let mut state_rx = self.state_rx.clone();
        loop {
            let state = state_rx.borrow().clone();
            match state {
                WorkerState::Failed(failure) => return failure,
                WorkerState::Stopped => std::future::pending::<()>().await,
                WorkerState::Running | WorkerState::Stopping => {}
            }
            if state_rx.changed().await.is_err() {
                return WorkerFailure::new("worker task terminated unexpectedly");
            }
        }
    }
    /// Waits until every item admitted before this call has completed.
    pub async fn wait_until_idle(&self) -> Result<(), WorkerFailure> {
        let (response_tx, response_rx) = oneshot::channel();
        self.work_tx
            .send(Work::Drain(response_tx))
            .await
            .map_err(|_| WorkerFailure::new("worker stopped before admitted work drained"))?;
        response_rx
            .await
            .map_err(|_| match self.state_rx.borrow().clone() {
                WorkerState::Failed(failure) => failure,
                _ => WorkerFailure::new("worker stopped before admitted work drained"),
            })
    }

    /// Rejects new work, cancels queued and running work, reaps child
    /// processes, and joins every worker task.
    pub async fn shutdown(mut self) -> Result<(), WorkerFailure> {
        self.cancellation_token.cancel();
        let task = self.task.take().expect("worker task must be owned");
        match task.await {
            Ok(result) => result,
            Err(error) => Err(WorkerFailure::new(format!(
                "worker task terminated unexpectedly: {error}"
            ))),
        }
    }
}

impl Drop for WorkerSupervisor {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}

fn unavailable_state(state: &WorkerState) -> Option<WorkerUnavailable> {
    match state {
        WorkerState::Running => None,
        WorkerState::Stopping | WorkerState::Stopped => Some(WorkerUnavailable::ShuttingDown),
        WorkerState::Failed(failure) => Some(WorkerUnavailable::Failed(failure.clone())),
    }
}

async fn wait_until_unavailable(state_rx: &mut watch::Receiver<WorkerState>) -> WorkerUnavailable {
    loop {
        if let Some(unavailable) = unavailable_state(&state_rx.borrow()) {
            return unavailable;
        }
        if state_rx.changed().await.is_err() {
            return WorkerUnavailable::Failed(WorkerFailure::new(
                "worker task terminated unexpectedly",
            ));
        }
    }
}

#[derive(Clone, Copy)]
enum WorkKind {
    Build,
    Match,
}

struct FinishedWork {
    kind: WorkKind,
    result: Result<(), WorkerFailure>,
}

enum StopReason {
    Shutdown,
    Failed(WorkerFailure),
}

async fn run_worker(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    referee: Arc<RefereeAdapter>,
    mut work_rx: Receiver<Work>,
    completion_tx: Sender<Completion>,
    state_tx: watch::Sender<WorkerState>,
    cancellation_token: CancellationToken,
) -> Result<(), WorkerFailure> {
    let execution_token = CancellationToken::new();
    let mut tasks = JoinSet::new();
    let mut pending_work = None;
    let mut build_running = false;
    let mut matches_running = 0usize;

    let stop_reason = 'running: loop {
        if pending_work
            .as_ref()
            .is_some_and(|work| matches!(work, Work::Drain(_)))
            && !build_running
            && matches_running == 0
        {
            let Work::Drain(response) = pending_work.take().expect("drain work must exist") else {
                unreachable!();
            };
            let _ = response.send(());
            continue;
        }
        if pending_work
            .as_ref()
            .is_some_and(|work| can_start(work, build_running, matches_running, &config))
        {
            let work = pending_work.take().expect("pending work must exist");
            match &work {
                Work::Build(_) => build_running = true,
                Work::Match(_) => matches_running += 1,
                Work::Drain(_) => unreachable!("drain messages are handled before execution"),
            }
            spawn_work(
                &mut tasks,
                worker_path.clone(),
                Arc::clone(&referee),
                Arc::clone(&config),
                completion_tx.clone(),
                execution_token.clone(),
                work,
            );
            continue;
        }

        tokio::select! {
            biased;
            _ = cancellation_token.cancelled() => break StopReason::Shutdown,
            result = tasks.join_next(), if !tasks.is_empty() => {
                match result {
                    Some(Ok(finished)) => {
                        match finished.kind {
                            WorkKind::Build => build_running = false,
                            WorkKind::Match => matches_running -= 1,
                        }
                        if let Err(failure) = finished.result {
                            state_tx.send_replace(WorkerState::Failed(failure.clone()));
                            break 'running StopReason::Failed(failure);
                        }
                    }
                    Some(Err(error)) => {
                        let failure = WorkerFailure::new(format!(
                            "worker execution task terminated unexpectedly: {error}"
                        ));
                        state_tx.send_replace(WorkerState::Failed(failure.clone()));
                        break 'running StopReason::Failed(failure);
                    }
                    None => {}
                }
            }
            work = work_rx.recv(), if pending_work.is_none() => {
                match work {
                    Some(work) => pending_work = Some(work),
                    None => break StopReason::Shutdown,
                }
            }
        }
    };

    work_rx.close();
    drop(pending_work);
    while work_rx.try_recv().is_ok() {}

    if matches!(stop_reason, StopReason::Shutdown) {
        state_tx.send_replace(WorkerState::Stopping);
    }
    execution_token.cancel();

    let mut cleanup_failure = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(FinishedWork {
                result: Err(failure),
                ..
            }) if cleanup_failure.is_none() => cleanup_failure = Some(failure),
            Err(error) if cleanup_failure.is_none() => {
                cleanup_failure = Some(WorkerFailure::new(format!(
                    "worker execution task terminated unexpectedly: {error}"
                )));
            }
            _ => {}
        }
    }
    drop(completion_tx);

    let failure = match stop_reason {
        StopReason::Failed(failure) => Some(failure),
        StopReason::Shutdown => cleanup_failure,
    };
    if let Some(failure) = failure {
        state_tx.send_replace(WorkerState::Failed(failure.clone()));
        Err(failure)
    } else {
        state_tx.send_replace(WorkerState::Stopped);
        Ok(())
    }
}

fn can_start(
    work: &Work,
    build_running: bool,
    matches_running: usize,
    config: &EmbeddedWorkerConfig,
) -> bool {
    match work {
        Work::Build(_) => !build_running,
        Work::Match(_) => matches_running < usize::from(config.threads),
        Work::Drain(_) => false,
    }
}

fn spawn_work(
    tasks: &mut JoinSet<FinishedWork>,
    worker_path: PathBuf,
    referee: Arc<RefereeAdapter>,
    config: Arc<EmbeddedWorkerConfig>,
    completion_tx: Sender<Completion>,
    cancellation_token: CancellationToken,
    work: Work,
) {
    tasks.spawn(async move {
        match work {
            Work::Build(input) => {
                let result = if let Some(output) =
                    execute_build(worker_path, config, input, cancellation_token.clone()).await
                {
                    publish_completion(
                        &completion_tx,
                        Completion::Build(output),
                        &cancellation_token,
                    )
                    .await;
                    Ok(())
                } else {
                    Ok(())
                };
                FinishedWork {
                    kind: WorkKind::Build,
                    result,
                }
            }
            Work::Match(input) => {
                let result = match execute_match(
                    worker_path,
                    config,
                    referee,
                    input,
                    cancellation_token.clone(),
                )
                .await
                {
                    Ok(Some(completion)) => {
                        publish_completion(&completion_tx, completion, &cancellation_token).await;
                        Ok(())
                    }
                    Ok(None) => Ok(()),
                    Err(failure) => Err(failure),
                };
                FinishedWork {
                    kind: WorkKind::Match,
                    result,
                }
            }
            Work::Drain(_) => unreachable!("drain messages are handled before execution"),
        }
    });
}

async fn publish_completion(
    completion_tx: &Sender<Completion>,
    completion: Completion,
    cancellation_token: &CancellationToken,
) -> bool {
    tokio::select! {
        biased;
        _ = cancellation_token.cancelled() => false,
        result = completion_tx.send(completion) => result.is_ok(),
    }
}

async fn execute_build(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    input: BuildBotInput,
    cancellation_token: CancellationToken,
) -> Option<BuildBotOutput> {
    let bot_id = input.bot_id;
    let worker_name = input.worker_name.clone();
    let result = match build_bot(worker_path, config, input, &cancellation_token).await {
        Ok(Some(result)) => result,
        Ok(None) => return None,
        Err(error) => BuildResult::Failure {
            stderr: format!("{error:#}"),
        },
    };
    Some(BuildBotOutput {
        bot_id,
        worker_name,
        result,
    })
}

async fn execute_match(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    referee: Arc<RefereeAdapter>,
    input: PlayMatchInput,
    cancellation_token: CancellationToken,
) -> Result<Option<Completion>, WorkerFailure> {
    if cancellation_token.is_cancelled() {
        return Ok(None);
    }
    let (command, replay) = prepare_play_match_command(&config, &referee, &input, &worker_path)
        .await
        .map_err(WorkerFailure::new)?;
    let output =
        spawn_play_match_command(command, replay, worker_path, &input, &cancellation_token)
            .await
            .map_err(|error| WorkerFailure::new(format!("{error:#}")))?;
    Ok(output.map(|output| Completion::Match { input, output }))
}

fn known_bot_ids(worker_path: &Path) -> anyhow::Result<Vec<BotId>> {
    let bots_folder = worker_path.join(DIR_BOTS);
    let mut res = vec![];

    if !bots_folder.exists() {
        return Ok(vec![]);
    }

    for entry in std::fs::read_dir(bots_folder)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let Ok(bot_id_i64) = name.parse::<i64>() else {
            continue;
        };

        res.push(BotId::from(bot_id_i64));
    }

    Ok(res)
}

async fn build_bot(
    worker_path: PathBuf,
    config: Arc<EmbeddedWorkerConfig>,
    input: BuildBotInput,
    cancellation_token: &CancellationToken,
) -> anyhow::Result<Option<BuildResult>> {
    if cancellation_token.is_cancelled() {
        return Ok(None);
    }

    let bot_folder_relative = PathBuf::from(DIR_BOTS).join(i64::from(input.bot_id).to_string());
    let bot_folder = worker_path.join(&bot_folder_relative);

    tokio::fs::create_dir_all(&bot_folder)
        .await
        .context("Failed to create bot folder")?;
    tokio::fs::write(
        bot_folder.join("source.txt"),
        &String::from(input.source_code),
    )
    .await
    .context("Cannot create source.txt file")?;

    if cancellation_token.is_cancelled() {
        return Ok(None);
    }

    let dir_param_value = bot_folder_relative
        .to_str()
        .context("Bot folder path is not utf-8")?;
    let command_parts = config
        .cmd_build
        .replace("{DIR}", dir_param_value)
        .replace("{LANG}", &input.language)
        .split_ascii_whitespace()
        .map(str::to_owned)
        .collect_vec();
    if command_parts.is_empty() {
        bail!("cmd_build must not be blank");
    }

    let Some(output) = run_command(command_parts, &worker_path, cancellation_token).await? else {
        return Ok(None);
    };
    Ok(Some(if output.status.success() {
        BuildResult::Success
    } else {
        BuildResult::Failure {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }))
}

async fn run_command(
    command_parts: Vec<String>,
    current_dir: &Path,
    cancellation_token: &CancellationToken,
) -> anyhow::Result<Option<Output>> {
    let program = command_parts.first().context("command must not be blank")?;
    let mut command = Command::new(program);
    command
        .args(&command_parts[1..])
        .current_dir(current_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().context("failed to spawn command")?;
    let mut stdout = child
        .stdout
        .take()
        .context("command stdout must be piped")?;
    let mut stderr = child
        .stderr
        .take()
        .context("command stderr must be piped")?;
    let stdout_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).await?;
        Ok::<_, std::io::Error>(bytes)
    });
    let stderr_task = tokio::spawn(async move {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).await?;
        Ok::<_, std::io::Error>(bytes)
    });

    let status: anyhow::Result<Option<ExitStatus>> = tokio::select! {
        result = child.wait() => result.context("failed to wait for command").map(Some),
        _ = cancellation_token.cancelled() => {
            let kill_result = child
                .start_kill()
                .or_else(|error| {
                    if error.kind() == std::io::ErrorKind::InvalidInput {
                        Ok(())
                    } else {
                        Err(error)
                    }
                })
                .context("failed to terminate canceled command");
            let wait_result = child
                .wait()
                .await
                .context("failed to reap canceled command");
            match (kill_result, wait_result) {
                (Err(error), _) | (_, Err(error)) => Err(error),
                (Ok(()), Ok(_)) => Ok(None),
            }
        }
    };

    let stdout = stdout_task
        .await
        .context("stdout reader task terminated unexpectedly")?
        .context("failed to read command stdout")?;
    let stderr = stderr_task
        .await
        .context("stderr reader task terminated unexpectedly")?
        .context("failed to read command stderr")?;

    Ok(status?.map(|status| Output {
        status,
        stdout,
        stderr,
    }))
}

async fn prepare_play_match_command(
    config: &EmbeddedWorkerConfig,
    adapter: &RefereeAdapter,
    input: &PlayMatchInput,
    worker_path: &Path,
) -> anyhow::Result<(PreparedMatchCommand, Option<ProvisionalReplay>)> {
    let run_commands = input
        .bots
        .iter()
        .map(|bot| {
            let bot_folder_relative =
                PathBuf::from(DIR_BOTS).join(i64::from(bot.bot_id).to_string());
            let dir_param_value = bot_folder_relative
                .to_str()
                .context("bot folder must be utf-8")?;
            Ok(config
                .cmd_run
                .replace("{DIR}", dir_param_value)
                .replace("{LANG}", &bot.language))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let replay = if adapter.match_replay_required() {
        Some(ProvisionalReplay::create(worker_path).await?)
    } else {
        None
    };
    let command = adapter.prepare_match_command(
        input.seed,
        &run_commands,
        replay.as_ref().map(ProvisionalReplay::command_path),
    )?;
    Ok((command, replay))
}

async fn spawn_play_match_command(
    command: PreparedMatchCommand,
    replay: Option<ProvisionalReplay>,
    worker_path: PathBuf,
    input: &PlayMatchInput,
    cancellation_token: &CancellationToken,
) -> anyhow::Result<Option<PlayMatchOutput>> {
    let result_source = command.result_source;
    let Some(cmd_output) = run_command(command.argv, &worker_path, cancellation_token).await?
    else {
        return Ok(None);
    };
    if !cmd_output.status.success() {
        let stderr = String::from_utf8_lossy(&cmd_output.stderr);
        let stdout = String::from_utf8_lossy(&cmd_output.stdout);
        let diagnostic = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        bail!(
            "Error while running match ({}): {}",
            cmd_output.status,
            diagnostic
        );
    }
    let result = match result_source {
        MatchResultSource::CodingameReplay => {
            let replay_path = worker_path.join(
                replay
                    .as_ref()
                    .context("codingame referee did not receive replay artifact")?
                    .command_path(),
            );
            read_codingame_match_result(&replay_path, input.bots.len())?
        }
        MatchResultSource::CommandStdout => {
            serde_json::from_slice::<CmdPlayMatchStdout>(&cmd_output.stdout)
                .context("play match output should be valid JSON")?
        }
    };
    validate_match_result(&result, input.bots.len())?;
    let replay = match replay {
        Some(replay) => replay.finish().await?,
        None => PendingReplay::default(),
    };
    Ok(Some(PlayMatchOutput {
        seed: input.seed,
        participants: input
            .bots
            .iter()
            .zip(result.ranks)
            .zip(result.errors)
            .map(|((bot, rank), error)| Participant {
                bot_id: bot.bot_id,
                rank,
                error: error == 1,
            })
            .collect(),
        attributes: result
            .attributes
            .into_iter()
            .map(|attribute| to_match_attribute(input, attribute))
            .chain(
                input
                    .bots
                    .iter()
                    .zip(result.scores)
                    .map(|(bot, score)| MatchAttribute {
                        name: "score".to_string(),
                        bot_id: Some(bot.bot_id),
                        turn: None,
                        value: MatchAttributeValue::Integer(score as _),
                    }),
            )
            .collect(),
        replay,
    }))
}

#[derive(Clone)]
pub struct BuildBotInput {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
    pub source_code: SourceCode,
    pub language: Language,
}

#[derive(Debug)]
pub struct BuildBotOutput {
    pub bot_id: BotId,
    pub worker_name: WorkerName,
    pub result: BuildResult,
}

pub struct PlayMatchInput {
    pub bots: Vec<PlayMatchBot>,
    pub seed: i64,
}

#[derive(Clone)]
pub struct PlayMatchBot {
    pub bot_id: BotId,
    pub language: Language,
}

pub struct PlayMatchOutput {
    pub seed: i64,
    pub participants: Vec<Participant>,
    pub attributes: Vec<MatchAttribute>,
    pub replay: PendingReplay,
}

fn to_match_attribute(input: &PlayMatchInput, attr: CmdMatchAttribute) -> MatchAttribute {
    let bot_id = attr.player.map(|p| input.bots[p].bot_id);

    MatchAttribute {
        name: attr.name,
        bot_id,
        turn: attr.turn,
        value: attr.value.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    fn config(cmd_build: String, cmd_play_match: String) -> EmbeddedWorkerConfig {
        EmbeddedWorkerConfig {
            threads: 1,
            referee: RefereeConfig::Command(crate::config::CommandRefereeConfig {
                play_match: cmd_play_match,
                watch_replay: "true".to_string(),

                legacy: false,
            }),
            cmd_build,
            cmd_run: "true".to_string(),
        }
    }

    fn script(directory: &Path, name: &str, body: &str) -> String {
        let path = directory.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        format!("sh {}", shell_words::quote(&path.to_string_lossy()))
    }
    #[cfg(unix)]
    fn executable_script(directory: &Path, name: &str, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let path = directory.join(name);
        fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    fn build_input(bot_id: i64) -> BuildBotInput {
        BuildBotInput {
            bot_id: BotId::from(bot_id),
            worker_name: WorkerName::embedded(),
            source_code: "source".to_string().try_into().unwrap(),
            language: "rust".to_string().try_into().unwrap(),
        }
    }

    fn match_input(seed: i64) -> PlayMatchInput {
        PlayMatchInput {
            bots: vec![
                PlayMatchBot {
                    bot_id: BotId::from(1),
                    language: "rust".to_string().try_into().unwrap(),
                },
                PlayMatchBot {
                    bot_id: BotId::from(2),
                    language: "rust".to_string().try_into().unwrap(),
                },
            ],
            seed,
        }
    }

    #[tokio::test]
    async fn reconciliation_returns_only_builds_that_must_be_reset() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("bots/1")).unwrap();
        fs::create_dir_all(directory.path().join("bots/5")).unwrap();
        let StartedWorker { worker, supervisor } = start_embedded_worker(
            directory.path(),
            config("true".to_string(), "true".to_string()),
        )
        .unwrap();

        let mut running = Build::new(BotId::from(1), WorkerName::embedded());
        running.make_running();
        let mut missing_success = Build::new(BotId::from(2), WorkerName::embedded());
        missing_success.make_running();
        missing_success.make_finished(BuildResult::Success);
        let mut failed = Build::new(BotId::from(3), WorkerName::embedded());
        failed.make_running();
        failed.make_finished(BuildResult::Failure {
            stderr: "expected".to_string(),
        });
        let pending = Build::new(BotId::from(4), WorkerName::embedded());
        let mut present_success = Build::new(BotId::from(5), WorkerName::embedded());
        present_success.make_running();
        present_success.make_finished(BuildResult::Success);

        let resets = worker
            .reconcile_builds(&[running, missing_success, failed, pending, present_success])
            .into_reset_builds();

        assert_eq!(
            resets,
            vec![
                BuildKey {
                    bot_id: BotId::from(1),
                    worker_name: WorkerName::embedded(),
                },
                BuildKey {
                    bot_id: BotId::from(2),
                    worker_name: WorkerName::embedded(),
                },
            ]
        );
        supervisor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn build_completions_preserve_success_and_failure_correlation() {
        let directory = tempfile::tempdir().unwrap();
        let build_command = script(
            directory.path(),
            "build.sh",
            "case \"$1\" in *2) echo rejected >&2; exit 1;; *) exit 0;; esac",
        );
        let StartedWorker { worker, supervisor } = start_embedded_worker(
            directory.path(),
            config(format!("{build_command} {{DIR}}"), "true".to_string()),
        )
        .unwrap();

        worker.submit(Work::Build(build_input(1))).await.unwrap();
        worker.submit(Work::Build(build_input(2))).await.unwrap();

        let Completion::Build(first) = worker.next().await.unwrap() else {
            panic!("expected a build completion");
        };
        let Completion::Build(second) = worker.next().await.unwrap() else {
            panic!("expected a build completion");
        };
        assert_eq!(first.bot_id, BotId::from(1));
        assert_eq!(first.worker_name, WorkerName::embedded());
        assert!(matches!(first.result, BuildResult::Success));
        assert_eq!(second.bot_id, BotId::from(2));
        assert_eq!(second.worker_name, WorkerName::embedded());
        assert!(matches!(
            second.result,
            BuildResult::Failure { ref stderr } if stderr.contains("rejected")
        ));
        supervisor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn match_completions_use_the_interface_and_keep_replays_distinct() {
        let directory = tempfile::tempdir().unwrap();
        let match_command = script(
            directory.path(),
            "match.sh",
            "printf replay > \"$1\"\nprintf '%s\\n' '{\"ranks\":[0,1],\"errors\":[0,0]}'",
        );
        let StartedWorker { worker, supervisor } = start_embedded_worker(
            directory.path(),
            config(
                "true".to_string(),
                format!("{match_command} {{REPLAY_PATH}}"),
            ),
        )
        .unwrap();

        worker.submit(Work::Match(match_input(42))).await.unwrap();
        worker.submit(Work::Match(match_input(42))).await.unwrap();
        let Completion::Match {
            input: first_input,
            output: first,
        } = worker.next().await.unwrap()
        else {
            panic!("expected a match completion");
        };
        let Completion::Match {
            input: second_input,
            output: second,
        } = worker.next().await.unwrap()
        else {
            panic!("expected a match completion");
        };

        assert_eq!(first_input.seed, 42);
        assert_eq!(second_input.seed, 42);
        assert_eq!(first.participants.len(), 2);
        assert_eq!(second.participants.len(), 2);
        assert_ne!(first.replay, second.replay);
        supervisor.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn malformed_match_output_is_an_observable_terminal_failure() {
        let directory = tempfile::tempdir().unwrap();
        let match_command = script(
            directory.path(),
            "malformed-match.sh",
            "printf replay > \"$1\"\nprintf not-json",
        );
        let StartedWorker { worker, supervisor } = start_embedded_worker(
            directory.path(),
            config(
                "true".to_string(),
                format!("{match_command} {{REPLAY_PATH}}"),
            ),
        )
        .unwrap();

        worker.submit(Work::Match(match_input(7))).await.unwrap();
        let failure = tokio::time::timeout(Duration::from_secs(2), supervisor.failed())
            .await
            .expect("worker failure should be observable");
        assert!(failure.to_string().contains("valid JSON"));
        assert!(matches!(
            worker.next().await,
            Err(WorkerUnavailable::Failed(_))
        ));
        assert!(supervisor.shutdown().await.is_err());
        assert_eq!(
            fs::read_dir(directory.path().join("replays"))
                .unwrap()
                .count(),
            0
        );
    }
    #[tokio::test]
    async fn referee_adapters_preserve_player_argument_contracts() {
        let directory = tempfile::tempdir().unwrap();
        let input = match_input(42);
        let command_config = config("true".to_string(), "runner {P1} {P2} {SEED}".to_string());
        let command_adapter = RefereeAdapter::from(&command_config.referee);
        let (command, replay) =
            prepare_play_match_command(&command_config, &command_adapter, &input, directory.path())
                .await
                .unwrap();
        assert_eq!(command.argv, ["runner", "true", "true", "42"]);
        assert!(matches!(
            command.result_source,
            MatchResultSource::CommandStdout
        ));
        assert!(replay.is_none());

        let jar_config = config("true".to_string(), "true".to_string());
        let jar_adapter = RefereeAdapter::codingame(PathBuf::from("referee.jar"), "java");
        let (command, replay) =
            prepare_play_match_command(&jar_config, &jar_adapter, &input, directory.path())
                .await
                .unwrap();
        assert!(matches!(
            command.result_source,
            MatchResultSource::CodingameReplay
        ));
        assert!(replay.is_some());
        assert_eq!(
            &command.argv[..12],
            [
                "java",
                "--add-opens",
                "java.base/java.lang=ALL-UNNAMED",
                "-jar",
                "referee.jar",
                "-p1",
                "true",
                "-p2",
                "true",
                "-seed",
                "42",
                "-league",
            ]
        );
        assert_eq!(command.argv[12], "19");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codingame_adapter_runs_match_and_preserves_observable_contract() {
        let directory = tempfile::tempdir().unwrap();
        let java = executable_script(
            directory.path(),
            "fake-java.sh",
            r#"replay=''
player_one=''
player_two=''
seed=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -p1) player_one="$2"; shift 2 ;;
    -p2) player_two="$2"; shift 2 ;;
    -seed) seed="$2"; shift 2 ;;
    -l) replay="$2"; shift 2 ;;
    *) shift ;;
  esac
done
test "$player_one" = "run bots/1 --language rust"
test "$player_two" = "run bots/2 --language rust"
test "$seed" = "73"
printf '%s\n' '{"scores":{"0":9,"1":4},"errors":{"0":[null,"[tdata] rounds = 3"],"1":[null]}}' > "$replay""#,
        );
        let mut config = config("true".to_string(), "true".to_string());
        config.cmd_run = "run {DIR} --language {LANG}".to_string();
        let referee =
            RefereeAdapter::codingame(PathBuf::from("fixture.jar"), java.to_string_lossy());
        let StartedWorker { worker, supervisor } =
            start_embedded_worker_with_referee(directory.path(), config, referee).unwrap();

        worker.submit(Work::Match(match_input(73))).await.unwrap();
        let Completion::Match { input, output } = worker.next().await.unwrap() else {
            panic!("expected a match completion");
        };

        assert_eq!(input.seed, 73);
        assert_eq!(output.participants[0].rank, 0);
        assert!(!output.participants[0].error);
        assert_eq!(output.participants[1].rank, 1);
        assert_eq!(
            output
                .attributes
                .iter()
                .find(|attribute| attribute.name == "rounds")
                .and_then(|attribute| attribute.value.integer_value()),
            Some(3)
        );
        assert_eq!(
            fs::read_dir(directory.path().join("replays"))
                .unwrap()
                .count(),
            1
        );
        supervisor.shutdown().await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn codingame_adapter_reports_nonzero_status_and_stdout_diagnostic() {
        let directory = tempfile::tempdir().unwrap();
        let java = executable_script(
            directory.path(),
            "failing-java.sh",
            "echo actionable-on-stdout\nexit 7",
        );
        let config = config("true".to_string(), "true".to_string());
        let referee =
            RefereeAdapter::codingame(PathBuf::from("fixture.jar"), java.to_string_lossy());
        let StartedWorker { worker, supervisor } =
            start_embedded_worker_with_referee(directory.path(), config, referee).unwrap();

        worker.submit(Work::Match(match_input(9))).await.unwrap();
        let failure = supervisor.failed().await;

        assert!(failure.to_string().contains("exit status: 7"));
        assert!(failure.to_string().contains("actionable-on-stdout"));
        assert!(supervisor.shutdown().await.is_err());
    }

    #[test]
    fn codingame_replay_conversion_preserves_results_and_attributes() {
        let directory = tempfile::tempdir().unwrap();
        let replay = directory.path().join("replay.json");
        fs::write(
            &replay,
            r#"{
                "scores": {"0": 10, "1": -1},
                "errors": {
                    "0": [null, "[TDATA][12] map_size = 16\n[PDATA] final_score = 10"],
                    "1": [null, "[pdata] lower_case = kept"]
                }
            }"#,
        )
        .unwrap();

        let result = read_codingame_match_result(&replay, 2).unwrap();

        assert_eq!(result.scores, [10, -1]);
        assert_eq!(result.ranks, [0, 1]);
        assert_eq!(result.errors, [0, 1]);
        assert_eq!(result.attributes.len(), 3);
        assert_eq!(result.attributes[0].name, "map_size");
        assert_eq!(result.attributes[0].player, None);
        assert_eq!(result.attributes[0].turn, Some(12));
        assert_eq!(result.attributes[1].name, "final_score");
        assert_eq!(result.attributes[1].player, Some(0));
        assert_eq!(result.attributes[2].name, "lower_case");
        assert_eq!(result.attributes[2].player, Some(1));
    }

    #[test]
    fn codingame_replay_conversion_rejects_wrong_score_count() {
        let directory = tempfile::tempdir().unwrap();
        let replay = directory.path().join("replay.json");
        fs::write(&replay, r#"{"scores":{"0":10},"errors":{}}"#).unwrap();

        let error = read_codingame_match_result(&replay, 2).unwrap_err();

        assert!(error.to_string().contains("1 scores for 2 bots"));
    }

    #[test]
    fn command_result_rejects_wrong_score_count() {
        let result = CmdPlayMatchStdout {
            scores: vec![10],
            ranks: vec![0, 1],
            errors: vec![0, 0],
            attributes: Vec::new(),
        };

        let error = validate_match_result(&result, 2).unwrap_err();

        assert!(error.to_string().contains("1 scores for 2 bots"));
    }

    #[test]
    fn command_result_rejects_attribute_for_missing_player() {
        let result = CmdPlayMatchStdout {
            scores: Vec::new(),
            ranks: vec![0, 1],
            errors: vec![0, 0],
            attributes: vec![CmdMatchAttribute {
                name: "score".to_string(),
                player: Some(2),
                turn: None,
                value: "10".to_string(),
            }],
        };

        let error = validate_match_result(&result, 2).unwrap_err();

        assert!(error.to_string().contains("missing player 2"));
    }

    #[tokio::test]
    async fn saturation_waits_and_shutdown_releases_submitters() {
        let directory = tempfile::tempdir().unwrap();
        let match_command = script(directory.path(), "blocking-match.sh", "exec sleep 30");
        let StartedWorker { worker, supervisor } =
            start_embedded_worker(directory.path(), config("true".to_string(), match_command))
                .unwrap();

        for seed in 0..5 {
            worker.submit(Work::Match(match_input(seed))).await.unwrap();
        }
        let blocked = worker.submit(Work::Match(match_input(5)));
        tokio::pin!(blocked);
        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut blocked)
                .await
                .is_err()
        );

        let shutdown = supervisor.shutdown();
        let (submission_result, shutdown_result) =
            tokio::time::timeout(Duration::from_secs(2), async {
                tokio::join!(blocked, shutdown)
            })
            .await
            .expect("shutdown should reap running work");
        assert!(matches!(
            submission_result,
            Err(WorkerUnavailable::ShuttingDown)
        ));
        shutdown_result.unwrap();
    }
}
