use crate::arena_commands::*;
use crate::async_leaderboard::AsyncLeaderboard;
use crate::config::{GameConfig, LeaderboardsConfig, RankingConfig};
use crate::domain::*;
use crate::evaluation::{
    self, CoveragePolicyConfig, EvaluationConfig, EvaluationPlanRevision, ScheduledEvaluationCounts,
};
use crate::match_retrieval::MatchRetrieval;
use crate::ranking::Ranker;
use crate::replay_artifact::ReplayArtifacts;
use crate::worker::{
    BuildBotInput, BuildBotOutput, BuildReconciliation, Completion, PlayMatchBot, PlayMatchInput,
    PlayMatchOutput, Work, Worker, WorkerUnavailable,
};
use crate::{chart, db};
use anyhow::{bail, Context};
use itertools::Itertools;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio::time::{Instant, MissedTickBehavior};
use tokio_util::sync::CancellationToken;
use tracing::{error, instrument, warn};

pub async fn run(
    game_config: GameConfig,
    evaluation_config: EvaluationConfig,
    leaderboards_config: LeaderboardsConfig,
    ranking_config: RankingConfig,
    pool: SqlitePool,
    arena_path: PathBuf,
    worker: Worker,
    commands_rx: Receiver<ArenaCommand>,
    cancellation_token: CancellationToken,
) -> anyhow::Result<JoinHandle<anyhow::Result<()>>> {
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Cannot run db migrations")?;

    let ranker = Ranker::new(ranking_config);
    if game_config.max_players > 2 && !ranker.support_multi_team() {
        bail!("Configured ranking algorithm only supports 2 player games");
    }
    let current_plan = db::ensure_evaluation_plan(&pool, &evaluation_config).await?;

    let mut arena = Arena::new(
        game_config,
        evaluation_config,
        current_plan,
        leaderboards_config,
        ranker,
        arena_path,
        pool,
    );

    arena
        .load_from_db()
        .await
        .context("Cannot load initial data from db")?;
    let reconciliation = worker.reconcile_builds(&arena.builds);
    arena.apply_build_reconciliation(reconciliation).await;
    arena.recalculate_computed_full();

    Ok(tokio::spawn(run_loop(
        arena,
        worker,
        commands_rx,
        cancellation_token,
    )))
}

async fn run_loop(
    mut arena: Arena,
    worker: Worker,
    mut commands_rx: Receiver<ArenaCommand>,
    cancellation_token: CancellationToken,
) -> anyhow::Result<()> {
    let period = Duration::from_millis(50);
    let mut chores = tokio::time::interval_at(Instant::now() + period, period);
    chores.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut scheduled_work = Vec::<Work>::new().into_iter();
    let mut submission = None;

    loop {
        if submission.is_none() {
            if let Some(work) = scheduled_work.next() {
                submission = Some(Box::pin(worker.submit(work)));
            }
        }

        tokio::select! {
            _ = cancellation_token.cancelled() => return Ok(()),
            result = async {
                submission
                    .as_mut()
                    .expect("guarded submission must exist")
                    .await
            }, if submission.is_some() => {
                submission = None;
                match result {
                    Ok(()) => {}
                    Err(WorkerUnavailable::ShuttingDown) => return Ok(()),
                    Err(WorkerUnavailable::Failed(failure)) => return Err(failure.into()),
                }
            }
            completion = worker.next() => {
                match completion {
                    Ok(completion) => arena.handle_worker_completion(completion).await,
                    Err(WorkerUnavailable::ShuttingDown) => return Ok(()),
                    Err(WorkerUnavailable::Failed(failure)) => return Err(failure.into()),
                }
            }
            command = commands_rx.recv() => {
                let Some(command) = command else {
                    return Ok(());
                };
                if matches!(
                    &command,
                    ArenaCommand::SetEvaluationScheduling(command) if !command.enabled
                ) {
                    submission = None;
                    scheduled_work = Vec::new().into_iter();
                }
                if matches!(
                    &command,
                    ArenaCommand::DeleteBot(_) | ArenaCommand::ChangeBotRole(_)
                ) {
                    submission = None;
                    scheduled_work = Vec::new().into_iter();
                    arena.scheduled_evaluations = Default::default();
                }
                arena.handle_command(command).await;
            }
            _ = chores.tick() => {
                arena.let_leaderboards_catchup_with_live_matches();
                if submission.is_none() && scheduled_work.len() == 0 {
                    scheduled_work = arena.prepare_worker_work().await.into_iter();
                }
            }
        }
    }
}

struct Arena {
    game_config: GameConfig,
    current_plan: EvaluationPlanRevision,
    plans: HashMap<i64, EvaluationPlanRevision>,
    uncertainty_coefficient: f64,
    pool: SqlitePool,
    match_retrieval: MatchRetrieval,
    replay_artifacts: ReplayArtifacts,
    bots: Vec<Bot>,
    builds: Vec<Build>,
    ranker: Arc<Ranker>,
    global_leaderboard: AsyncLeaderboard,
    custom_leaderboards: Vec<AsyncLeaderboard>,
    scheduled_evaluations: ScheduledEvaluationCounts,
    evaluation_scheduling_enabled: bool,
}

impl Arena {
    fn new(
        game_config: GameConfig,
        evaluation_config: EvaluationConfig,
        current_plan: EvaluationPlanRevision,
        leaderboards_config: LeaderboardsConfig,
        ranker: Ranker,
        arena_path: PathBuf,
        pool: SqlitePool,
    ) -> Self {
        let ranker = Arc::new(ranker);
        let match_retrieval = MatchRetrieval::new(pool.clone());
        let replay_artifacts = ReplayArtifacts::new(pool.clone(), arena_path);
        Self {
            game_config,
            current_plan,
            plans: Default::default(),
            uncertainty_coefficient: leaderboards_config.uncertainty_coefficient.unwrap_or(3.0),
            evaluation_scheduling_enabled: evaluation_config.enabled_on_start.unwrap_or(true),
            replay_artifacts,
            pool,
            match_retrieval: match_retrieval.clone(),
            ranker: Arc::clone(&ranker),
            bots: Default::default(),
            builds: Default::default(),
            global_leaderboard: AsyncLeaderboard::new(
                Leaderboard::global(),
                ranker,
                match_retrieval,
            ),
            custom_leaderboards: Default::default(),
            scheduled_evaluations: Default::default(),
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn load_from_db(&mut self) -> anyhow::Result<()> {
        self.bots = db::fetch_bots(&self.pool)
            .await
            .context("Cannot fetch bots")?;
        self.plans
            .insert(self.current_plan.id, self.current_plan.clone());
        let plan_ids = self
            .bots
            .iter()
            .filter_map(|bot| bot.evaluation_plan_revision_id)
            .unique()
            .collect_vec();
        for plan_id in plan_ids {
            if !self.plans.contains_key(&plan_id) {
                self.plans.insert(
                    plan_id,
                    db::fetch_evaluation_plan(&self.pool, plan_id)
                        .await
                        .with_context(|| {
                            format!("Cannot load evaluation plan revision {plan_id}")
                        })?,
                );
            }
        }
        self.builds = db::fetch_builds(&self.pool)
            .await
            .context("Cannot fetch builds")?;
        self.custom_leaderboards = db::fetch_leaderboards(&self.pool)
            .await
            .context("Cannot fetch leaderboards")?
            .into_iter()
            .map(|lb| {
                AsyncLeaderboard::new(lb, Arc::clone(&self.ranker), self.match_retrieval.clone())
            })
            .collect();
        Ok(())
    }

    async fn apply_build_reconciliation(&mut self, reconciliation: BuildReconciliation) {
        for key in reconciliation.into_reset_builds() {
            let build = self
                .builds
                .iter_mut()
                .find(|build| build.bot_id == key.bot_id && build.worker_name == key.worker_name)
                .expect("worker reconciliation must reference a supplied build");
            build.reset();
            db::persist_build(&self.pool, build)
                .await
                .expect("Cannot persist build to DB");
        }
    }

    async fn prepare_worker_work(&mut self) -> Vec<Work> {
        let builds = self.prepare_build_work().await;
        if !builds.is_empty() {
            return builds.into_iter().map(Work::Build).collect();
        }

        if self.builds.iter().any(Build::is_running) || !self.evaluation_scheduling_enabled {
            return Vec::new();
        }

        self.perform_evaluation()
            .await
            .into_iter()
            .map(Work::Match)
            .collect()
    }

    #[instrument(skip(self), level = "debug")]
    async fn prepare_build_work(&mut self) -> Vec<BuildBotInput> {
        let mut inputs = Vec::new();
        for bot in &mut self.bots {
            if !bot.role.is_active() {
                continue;
            }
            let worker_name = WorkerName::embedded();
            let existing_build = self
                .builds
                .iter_mut()
                .find(|build| build.bot_id == bot.id && build.worker_name == worker_name);

            let build = match existing_build {
                Some(build) if build.is_pending() => build,
                None => {
                    self.builds.push(Build::new(bot.id, worker_name.clone()));
                    self.builds.last_mut().expect("build was just inserted")
                }
                _ => continue,
            };

            build.make_running();
            db::persist_build(&self.pool, build)
                .await
                .expect("Cannot persist build to DB");
            inputs.push(BuildBotInput {
                bot_id: bot.id,
                worker_name,
                source_code: bot.source_code.clone(),
                language: bot.language.clone(),
            });
        }
        inputs
    }

    async fn handle_worker_completion(&mut self, completion: Completion) {
        match completion {
            Completion::Build(output) => self.finish_build(output).await,
            Completion::Match { input, output } => {
                self.process_finished_match(&input, output).await;
            }
        }
    }

    async fn finish_build(&mut self, output: BuildBotOutput) {
        if !self.bots.iter().any(|bot| bot.id == output.bot_id) {
            warn!(
                "Obtained build result for non-existent bot, skipping. {:?}",
                output
            );
            return;
        }

        let build = self
            .builds
            .iter_mut()
            .find(|build| build.bot_id == output.bot_id && build.worker_name == output.worker_name);
        let Some(build) = build else {
            warn!("Obtained build result for non-existent build, skipping");
            return;
        };

        build.make_finished(output.result);
        db::persist_build(&self.pool, build)
            .await
            .expect("Cannot persist build to DB");
    }

    async fn cmd_fetch_bot_source_code(&mut self, id: BotId) -> Option<BotSourceCode> {
        let bot = self.bots.iter_mut().find(|b| b.id == id)?;

        Some(BotSourceCode {
            language: bot.language.clone(),
            source_code: bot.source_code.clone(),
        })
    }

    fn cmd_set_evaluation_scheduling(&mut self, enabled: bool) {
        self.evaluation_scheduling_enabled = enabled;
    }

    async fn cmd_create_bot(
        &mut self,
        name: BotName,
        source_code: SourceCode,
        language: Language,
        role: BotRole,
    ) -> CreateBotResult {
        if self.bots.iter().any(|b| b.name == name) {
            return CreateBotResult::DuplicateName;
        }
        let plan_revision_id = (role == BotRole::Candidate).then_some(self.current_plan.id);
        let mut bot = Bot::new(name, source_code, language, role, plan_revision_id);
        db::persist_bot(&self.pool, &mut bot)
            .await
            .expect("Cannot persist bot to DB");
        let bot_overview = self.render_bot_overview(&bot, None);
        self.bots.push(bot);
        CreateBotResult::Created(bot_overview)
    }

    async fn cmd_rename_bot(&mut self, id: BotId, new_name: BotName) -> RenameBotResult {
        if self.bots.iter().any(|b| b.id != id && b.name == new_name) {
            return RenameBotResult::DuplicateName;
        }

        let Some(bot) = self.bots.iter_mut().find(|b| b.id == id) else {
            return RenameBotResult::NotFound;
        };

        bot.name = new_name;
        db::persist_bot(&self.pool, bot)
            .await
            .expect("Cannot persist bot to DB");
        RenameBotResult::Renamed
    }

    async fn cmd_reject_candidate(&mut self, id: BotId) -> BotRoleTransitionResult {
        let Some(bot) = self.bots.iter().find(|bot| bot.id == id) else {
            return BotRoleTransitionResult::NotFound;
        };
        if bot.role != BotRole::Candidate {
            return BotRoleTransitionResult::InvalidState;
        }
        self.replay_artifacts
            .delete_bot(id)
            .await
            .expect("Cannot delete candidate and its replay artifacts");
        self.bots.retain(|bot| bot.id != id);
        self.builds.retain(|build| build.bot_id != id);
        self.recalculate_computed_full();
        BotRoleTransitionResult::Changed
    }

    async fn cmd_change_bot_role(
        &mut self,
        id: BotId,
        transition: BotRoleTransition,
    ) -> BotRoleTransitionResult {
        let Some(bot) = self.bots.iter_mut().find(|bot| bot.id == id) else {
            return BotRoleTransitionResult::NotFound;
        };
        bot.role = match (bot.role, transition) {
            (BotRole::Candidate, BotRoleTransition::Promote) => BotRole::Benchmark,
            (BotRole::Benchmark, BotRoleTransition::Archive) => BotRole::ArchivedBenchmark,
            _ => return BotRoleTransitionResult::InvalidState,
        };
        db::persist_bot(&self.pool, bot)
            .await
            .expect("Cannot persist bot lifecycle state");
        BotRoleTransitionResult::Changed
    }

    async fn cmd_fetch_status(&mut self) -> FetchStatusResult {
        let stored_bots = self.bots.clone();
        let mut bots = Vec::with_capacity(stored_bots.len());
        for bot in &stored_bots {
            let evaluation = self.render_candidate_evaluation(bot).await;
            bots.push(self.render_bot_overview(bot, evaluation));
        }

        let leaderboards =
            std::iter::once(self.render_leaderboard_overview(&self.global_leaderboard))
                .chain(
                    self.custom_leaderboards
                        .iter()
                        .map(|leaderboard| self.render_leaderboard_overview(leaderboard)),
                )
                .collect_vec();

        FetchStatusResult {
            bots,
            leaderboards,
            evaluation_scheduling_enabled: self.evaluation_scheduling_enabled,
        }
    }

    fn render_bot_overview(
        &self,
        bot: &Bot,
        evaluation: Option<CandidateEvaluationOverview>,
    ) -> BotOverview {
        BotOverview {
            id: bot.id,
            name: bot.name.clone(),
            language: bot.language.clone(),
            role: bot.role,
            evaluation_plan_revision_id: bot.evaluation_plan_revision_id,
            evaluation,
            matches_played: self
                .global_leaderboard
                .stats()
                .map(|stats| stats.matches_played(bot.id))
                .unwrap_or_default(),
            matches_with_error: self
                .global_leaderboard
                .stats()
                .map(|stats| stats.matches_with_error(bot.id))
                .unwrap_or_default(),
            builds: self
                .builds
                .iter()
                .filter(|build| build.bot_id == bot.id)
                .cloned()
                .collect(),
            created_at: bot.created_at,
        }
    }

    async fn render_candidate_evaluation(
        &mut self,
        bot: &Bot,
    ) -> Option<CandidateEvaluationOverview> {
        let plan_id = bot.evaluation_plan_revision_id?;
        let plan = self.plans.get(&plan_id)?;
        let benchmarks = self
            .bots
            .iter()
            .filter(|bot| bot.role == BotRole::Benchmark)
            .map(|bot| bot.id)
            .collect_vec();
        let mut stages = Vec::with_capacity(plan.stages.len());
        for stage in &plan.stages {
            let progress = db::fetch_stage_progress(&self.pool, bot.id, stage.id)
                .await
                .expect("Cannot fetch evaluation progress");
            let complete = match &stage.config.coverage {
                CoveragePolicyConfig::PerBenchmark { target } => {
                    !benchmarks.is_empty()
                        && benchmarks.iter().all(|id| {
                            progress.encounters.get(id).copied().unwrap_or_default() >= *target
                        })
                }
                CoveragePolicyConfig::Total { target }
                | CoveragePolicyConfig::WeightedTotal { target, .. } => progress.matches >= *target,
            };
            let coverage = match stage.config.coverage {
                CoveragePolicyConfig::PerBenchmark { .. } => "per_benchmark",
                CoveragePolicyConfig::Total { .. } => "total",
                CoveragePolicyConfig::WeightedTotal { .. } => "weighted_total",
            };
            stages.push(EvaluationStageOverview {
                id: stage.id,
                name: stage.config.name.clone(),
                coverage,
                target: stage.config.coverage.target(),
                matches: progress.matches,
                candidate_errors: progress.candidate_errors,
                complete,
                benchmark_encounters: benchmarks
                    .iter()
                    .map(|id| {
                        (
                            *id,
                            progress.encounters.get(id).copied().unwrap_or_default(),
                        )
                    })
                    .collect(),
            });
        }
        Some(CandidateEvaluationOverview {
            complete: stages.iter().all(|stage| stage.complete),
            stages,
        })
    }

    fn render_leaderboard_overview(&self, async_lb: &AsyncLeaderboard) -> LeaderboardOverview {
        let leaderboard = &async_lb.leaderboard;

        let Some(stats) = async_lb.stats() else {
            return LeaderboardOverview {
                id: leaderboard.id,
                name: leaderboard.name.clone(),
                filter: leaderboard.filter.to_string(),
                status: async_lb
                    .error()
                    .map(LeaderboardStatus::Error)
                    .unwrap_or(LeaderboardStatus::Computing),
                items: Default::default(),
                winrate_stats: Default::default(),
                total_matches: 0,
                example_seeds: vec![],
            };
        };

        let items = self
            .bots
            .iter()
            .filter(|bot| bot.role.is_active())
            .map(|bot| {
                let rating = self.rating(&stats, bot.id);
                LeaderboardItem {
                    id: bot.id,
                    rank: self.rank(&stats, bot.id),
                    rating,
                    rating_ordinal: rating.score(self.uncertainty_coefficient),
                }
            })
            .sorted_by_key(|item| item.rank)
            .collect_vec();

        let winrate_stats = stats.winrate_stats_snapshot();

        LeaderboardOverview {
            id: leaderboard.id,
            name: leaderboard.name.clone(),
            filter: leaderboard.filter.to_string(),
            status: LeaderboardStatus::Live,
            items,
            winrate_stats,
            total_matches: stats.total_matches(),
            example_seeds: stats.example_seeds().to_vec(),
        }
    }

    async fn cmd_create_leaderboard(
        &mut self,
        name: LeaderboardName,
        filter: MatchFilter,
    ) -> LeaderboardOverview {
        let mut leaderboard = Leaderboard::new(name, filter);
        db::persist_leaderboard(&self.pool, &mut leaderboard)
            .await
            .expect("Cannot persist leaderboard to DB");

        let lb = AsyncLeaderboard::new(
            leaderboard,
            Arc::clone(&self.ranker),
            self.match_retrieval.clone(),
        );
        lb.recalculate();
        let overview = self.render_leaderboard_overview(&lb);
        self.custom_leaderboards.push(lb);
        overview
    }

    async fn cmd_patch_leaderboard(
        &mut self,
        id: LeaderboardId,
        name: LeaderboardName,
        filter: MatchFilter,
    ) -> PatchLeaderboardResult {
        let Some(async_lb) = self
            .custom_leaderboards
            .iter_mut()
            .find(|w| w.leaderboard.id == id)
        else {
            return PatchLeaderboardResult::NotFound;
        };

        let leaderboard = &mut async_lb.leaderboard;

        let old_filter_str = leaderboard.filter.to_string();
        let new_filter_str = filter.to_string();

        leaderboard.name = name;
        leaderboard.filter = filter.clone();

        db::persist_leaderboard(&self.pool, leaderboard)
            .await
            .expect("Cannot persist leaderboard to DB");

        if old_filter_str != new_filter_str {
            async_lb.recalculate();
        }

        PatchLeaderboardResult::OK
    }

    async fn cmd_delete_leaderboard(&mut self, id: LeaderboardId) {
        db::delete_leaderboard(&self.pool, id)
            .await
            .expect("Cannot delete leaderboard from DB");
        self.custom_leaderboards.retain(|w| w.leaderboard.id != id);
    }

    fn rating(&self, stats: &ComputedStats, id: BotId) -> Rating {
        stats
            .rating(id)
            .unwrap_or_else(|| self.ranker.default_rating())
    }

    fn rank(&self, stats: &ComputedStats, id: BotId) -> usize {
        let my_rating = self.rating(stats, id);
        let stronger_bots_cnt = self
            .bots
            .iter()
            .filter(|b| {
                my_rating.score(self.uncertainty_coefficient)
                    < self.rating(stats, b.id).score(self.uncertainty_coefficient)
            })
            .count();
        stronger_bots_cnt
    }

    fn cmd_chart(&self, cmd: ChartCommand) {
        let ChartCommand {
            filter,
            attribute_name,
            response,
        } = cmd;
        let match_retrieval = self.match_retrieval.clone();

        tokio::spawn(async move {
            let res = chart::visualize(filter, attribute_name, match_retrieval).await;
            match res {
                Ok(overview) => {
                    let _ = response.send(overview);
                }
                Err(e) => {
                    error!("Failed to visualize chart: {}", e);
                }
            };
        });
    }

    pub async fn handle_command(&mut self, command: ArenaCommand) {
        match command {
            ArenaCommand::CreateBot(command) => {
                let res = self
                    .cmd_create_bot(
                        command.name,
                        command.source_code,
                        command.language,
                        command.role,
                    )
                    .await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::DeleteBot(command) => {
                let res = self.cmd_reject_candidate(command.id).await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::RenameBot(command) => {
                let res = self.cmd_rename_bot(command.id, command.new_name).await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::FetchStatus(command) => {
                let res = self.cmd_fetch_status().await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::CreateLeaderboard(command) => {
                let res = self
                    .cmd_create_leaderboard(command.name, command.filter)
                    .await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::DeleteLeaderboard(command) => {
                let res = self.cmd_delete_leaderboard(command.id).await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::PatchLeaderboard(command) => {
                let res = self
                    .cmd_patch_leaderboard(command.id, command.name, command.filter)
                    .await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::Chart(chart_command) => {
                // this one is a bit special
                self.cmd_chart(chart_command);
            }
            ArenaCommand::FetchBotSourceCode(command) => {
                let res = self.cmd_fetch_bot_source_code(command.id).await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::SetEvaluationScheduling(command) => {
                self.cmd_set_evaluation_scheduling(command.enabled);
                if command.response.send(()).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::ChangeBotRole(command) => {
                let result = self
                    .cmd_change_bot_role(command.id, command.transition)
                    .await;
                if command.response.send(result).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::FetchMatches(command) => {
                let res = self.match_retrieval.page(command.request).await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    async fn perform_evaluation(&mut self) -> Vec<PlayMatchInput> {
        const MATCH_BATCH_SIZE: usize = 20;
        let mut scheduled = Vec::new();
        while scheduled.len() < MATCH_BATCH_SIZE {
            let new_matches = self.schedule_evaluation_match().await;
            if new_matches.is_empty() {
                break;
            }
            for input in &new_matches {
                self.record_scheduled_match(input);
            }
            scheduled.extend(new_matches);
        }
        scheduled
    }

    #[instrument(skip(self), level = "debug")]
    pub fn let_leaderboards_catchup_with_live_matches(&mut self) {
        self.global_leaderboard.catch_up_with_live_matches();
        for async_lb in &mut self.custom_leaderboards {
            async_lb.catch_up_with_live_matches();
        }
    }

    #[instrument(skip(self, input, output), level = "debug")]
    async fn process_finished_match(&mut self, input: &PlayMatchInput, output: PlayMatchOutput) {
        self.forget_scheduled_match(input);
        let PlayMatchOutput {
            seed,
            participants,
            attributes,
            replay,
        } = output;

        if participants
            .iter()
            .any(|participant| self.bots.iter().all(|bot| bot.id != participant.bot_id))
        {
            warn!("Match participant was deleted while match was running, ignoring match results");
            return;
        }

        let attributes = attributes
            .into_iter()
            .unique_by(|attribute| (attribute.name.clone(), attribute.bot_id, attribute.turn))
            .collect();

        let mut new_match = Match::new(seed, participants, attributes, None);
        new_match.candidate_bot_id = input.candidate_bot_id;
        new_match.evaluation_stage_revision_id = input.evaluation_stage_revision_id;

        new_match
            .attributes
            .retain(|attribute| attribute.name != "seed");
        new_match.attributes.push(MatchAttribute {
            name: "seed".to_string(),
            bot_id: None,
            turn: None,
            value: MatchAttributeValue::Integer(seed),
        });

        new_match
            .attributes
            .retain(|attribute| attribute.name != "index");
        new_match
            .attributes
            .retain(|attribute| attribute.name != "error");
        new_match
            .attributes
            .retain(|attribute| attribute.name != "rank");

        for (index, participant) in new_match.participants.iter().enumerate() {
            new_match.attributes.push(MatchAttribute {
                name: "index".to_string(),
                bot_id: Some(participant.bot_id),
                turn: None,
                value: MatchAttributeValue::Integer(index as _),
            });

            new_match.attributes.push(MatchAttribute {
                name: "rank".to_string(),
                bot_id: Some(participant.bot_id),
                turn: None,
                value: MatchAttributeValue::Integer(participant.rank as _),
            });

            if participant.error {
                new_match.attributes.push(MatchAttribute {
                    name: "error".to_string(),
                    bot_id: Some(participant.bot_id),
                    turn: None,
                    value: MatchAttributeValue::Integer(1),
                });
            }
        }

        if self.game_config.min_players != self.game_config.max_players {
            new_match
                .attributes
                .retain(|attribute| attribute.name != "player_count");
            new_match.attributes.push(MatchAttribute {
                name: "player_count".to_string(),
                bot_id: None,
                turn: None,
                value: MatchAttributeValue::Integer(new_match.participants.len() as _),
            });
        }

        if let Err(error) = self
            .replay_artifacts
            .persist_match(replay, &mut new_match)
            .await
        {
            panic!("Cannot persist match to DB: {error:#}");
        }

        let new_match = Arc::new(new_match);
        self.global_leaderboard
            .record_for_later(Arc::clone(&new_match));
        for leaderboard in &mut self.custom_leaderboards {
            leaderboard.record_for_later(Arc::clone(&new_match));
        }
    }

    fn is_bot_ready_for_playing(&self, id: BotId) -> bool {
        for worker_name in std::iter::once(WorkerName::embedded()) {
            let ready = self
                .builds
                .iter()
                .find(|b| b.bot_id == id && b.worker_name == worker_name)
                .map(|b| b.was_finished_successfully())
                .unwrap_or(false);

            if !ready {
                return false;
            }
        }
        true
    }

    async fn schedule_evaluation_match(&self) -> Vec<PlayMatchInput> {
        let benchmarks = self
            .bots
            .iter()
            .filter(|bot| bot.role == BotRole::Benchmark && self.is_bot_ready_for_playing(bot.id))
            .map(|bot| bot.id)
            .collect_vec();
        for candidate in self
            .bots
            .iter()
            .filter(|bot| bot.role == BotRole::Candidate && self.is_bot_ready_for_playing(bot.id))
        {
            let Some(plan_id) = candidate.evaluation_plan_revision_id else {
                continue;
            };
            let Some(plan) = self.plans.get(&plan_id) else {
                continue;
            };
            let mut progress = HashMap::with_capacity(plan.stages.len());
            for stage in &plan.stages {
                progress.insert(
                    stage.id,
                    db::fetch_stage_progress(&self.pool, candidate.id, stage.id)
                        .await
                        .expect("Cannot fetch evaluation progress"),
                );
            }
            let matches = evaluation::schedule_candidate(
                &self.game_config,
                plan,
                candidate.id,
                &benchmarks,
                &progress,
                &self.scheduled_evaluations,
            );
            if !matches.is_empty() {
                return matches
                    .into_iter()
                    .map(|scheduled| PlayMatchInput {
                        bots: scheduled
                            .bot_ids
                            .into_iter()
                            .map(|id| PlayMatchBot {
                                bot_id: id,
                                language: self
                                    .bots
                                    .iter()
                                    .find(|bot| bot.id == id)
                                    .expect("scheduled bot must exist")
                                    .language
                                    .clone(),
                            })
                            .collect(),
                        seed: scheduled.seed,
                        candidate_bot_id: Some(scheduled.candidate_id),
                        evaluation_stage_revision_id: Some(scheduled.stage_revision_id),
                    })
                    .collect();
            }
        }
        Vec::new()
    }

    fn record_scheduled_match(&mut self, input: &PlayMatchInput) {
        let (Some(candidate_id), Some(stage_id)) =
            (input.candidate_bot_id, input.evaluation_stage_revision_id)
        else {
            return;
        };
        *self
            .scheduled_evaluations
            .matches_by_stage_candidate
            .entry((stage_id, candidate_id))
            .or_default() += 1;
        *self
            .scheduled_evaluations
            .matches_by_player_count
            .entry((stage_id, candidate_id, input.bots.len() as u32))
            .or_default() += 1;
        for opponent in input.bots.iter().filter(|bot| bot.bot_id != candidate_id) {
            *self
                .scheduled_evaluations
                .encounters
                .entry((stage_id, candidate_id, opponent.bot_id))
                .or_default() += 1;
        }
    }

    fn forget_scheduled_match(&mut self, input: &PlayMatchInput) {
        let (Some(candidate_id), Some(stage_id)) =
            (input.candidate_bot_id, input.evaluation_stage_revision_id)
        else {
            return;
        };
        decrement_count(
            &mut self.scheduled_evaluations.matches_by_stage_candidate,
            &(stage_id, candidate_id),
        );
        decrement_count(
            &mut self.scheduled_evaluations.matches_by_player_count,
            &(stage_id, candidate_id, input.bots.len() as u32),
        );
        for opponent in input.bots.iter().filter(|bot| bot.bot_id != candidate_id) {
            decrement_count(
                &mut self.scheduled_evaluations.encounters,
                &(stage_id, candidate_id, opponent.bot_id),
            );
        }
    }

    fn recalculate_computed_full(&self) {
        self.global_leaderboard.recalculate();
        for lb in &self.custom_leaderboards {
            lb.recalculate();
        }
    }
}

fn decrement_count<K: Eq + std::hash::Hash>(counts: &mut HashMap<K, u64>, key: &K) {
    let Some(count) = counts.get_mut(key) else {
        return;
    };
    *count -= 1;
    let remove = *count == 0;
    if remove {
        counts.remove(key);
    }
}
