use crate::arena_commands::*;
use crate::async_leaderboard::AsyncLeaderboard;
use crate::config::{GameConfig, LeaderboardsConfig, MatchmakingConfig, RankingConfig};
use crate::domain::*;
use crate::match_retrieval::MatchRetrieval;
use crate::matchmaking;
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
    matchmaking_config: MatchmakingConfig,
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

    let mut arena = Arena::new(
        game_config,
        matchmaking_config,
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
                    ArenaCommand::EnableMatchmaking(command) if !command.enabled
                ) {
                    submission = None;
                    scheduled_work = Vec::new().into_iter();
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
    matchmaking_config: MatchmakingConfig,
    uncertainty_coefficient: f64,
    pool: SqlitePool,
    match_retrieval: MatchRetrieval,
    replay_artifacts: ReplayArtifacts,
    bots: Vec<Bot>,
    builds: Vec<Build>,
    ranker: Arc<Ranker>,
    global_leaderboard: AsyncLeaderboard,
    custom_leaderboards: Vec<AsyncLeaderboard>,
    scheduled_matches_total: HashMap<BotId, u64>,
    scheduled_matches_vs: HashMap<(BotId, BotId), u64>,
    matchmaking_enabled: bool,
}

impl Arena {
    fn new(
        game_config: GameConfig,
        matchmaking_config: MatchmakingConfig,
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
            uncertainty_coefficient: leaderboards_config.uncertainty_coefficient.unwrap_or(3.0),
            matchmaking_enabled: matchmaking_config.enabled_on_start.unwrap_or(true),
            matchmaking_config,
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
            scheduled_matches_total: Default::default(),
            scheduled_matches_vs: Default::default(),
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn load_from_db(&mut self) -> anyhow::Result<()> {
        self.bots = db::fetch_bots(&self.pool)
            .await
            .context("Cannot fetch bots")?;
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

        if self.builds.iter().any(Build::is_running) || !self.matchmaking_enabled {
            return Vec::new();
        }

        self.perform_matchmaking()
            .into_iter()
            .map(Work::Match)
            .collect()
    }

    #[instrument(skip(self), level = "debug")]
    async fn prepare_build_work(&mut self) -> Vec<BuildBotInput> {
        let mut inputs = Vec::new();
        for bot in &mut self.bots {
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

    fn cmd_enable_matchmaking(&mut self, enabled: bool) {
        self.matchmaking_enabled = enabled;
    }

    async fn cmd_create_bot(
        &mut self,
        name: BotName,
        source_code: SourceCode,
        language: Language,
    ) -> CreateBotResult {
        if self.bots.iter().any(|b| b.name == name) {
            return CreateBotResult::DuplicateName;
        }
        let mut bot = Bot::new(name, source_code, language);
        db::persist_bot(&self.pool, &mut bot)
            .await
            .expect("Cannot persist bot to DB");
        let bot_overview = self.render_bot_overview(&bot);
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

    async fn cmd_delete_bot(&mut self, id: BotId) {
        // builds and participations are deleted by foreign keys; matches by the DB trigger
        self.replay_artifacts
            .delete_bot(id)
            .await
            .expect("Cannot delete bot and its replay artifacts");
        self.bots.retain(|bot| bot.id != id);
        self.builds.retain(|b| b.bot_id != id);
        self.recalculate_computed_full();
    }

    async fn cmd_fetch_status(&mut self) -> FetchStatusResult {
        let bots = self
            .bots
            .iter()
            .map(|bot| self.render_bot_overview(bot))
            .collect_vec();

        let leaderboards =
            std::iter::once(self.render_leaderboard_overview(&self.global_leaderboard))
                .chain(
                    self.custom_leaderboards
                        .iter()
                        .map(|lb| self.render_leaderboard_overview(lb)),
                )
                .collect_vec();

        let matchmaking_enabled = self.matchmaking_enabled;

        FetchStatusResult {
            bots,
            leaderboards,
            matchmaking_enabled,
        }
    }

    fn render_bot_overview(&self, bot: &Bot) -> BotOverview {
        BotOverview {
            id: bot.id,
            name: bot.name.clone(),
            language: bot.language.clone(),
            matches_played: self
                .global_leaderboard
                .stats()
                .map(|s| s.matches_played(bot.id))
                .unwrap_or_default(),
            matches_with_error: self
                .global_leaderboard
                .stats()
                .map(|s| s.matches_with_error(bot.id))
                .unwrap_or_default(),
            builds: self
                .builds
                .iter()
                .filter(|b| b.bot_id == bot.id)
                .cloned()
                .collect(),
            created_at: bot.created_at,
        }
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
                    .cmd_create_bot(command.name, command.source_code, command.language)
                    .await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::DeleteBot(command) => {
                let res = self.cmd_delete_bot(command.id).await;
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
            ArenaCommand::EnableMatchmaking(command) => {
                self.cmd_enable_matchmaking(command.enabled);
                if command.response.send(()).is_err() {
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
    fn perform_matchmaking(&mut self) -> Vec<PlayMatchInput> {
        // hardcoded for now
        let match_batch_size: usize = 20;
        let mut scheduled = Vec::new();

        while scheduled.len() < match_batch_size {
            let new_matches = self.schedule_match();
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

    fn schedule_match(&self) -> Vec<PlayMatchInput> {
        let Some(stats) = self.global_leaderboard.stats() else {
            return vec![];
        };

        let ready_bot_ids = self
            .bots
            .iter()
            .map(|b| b.id)
            .filter(|id| self.is_bot_ready_for_playing(*id))
            .collect_vec();

        let candidates = ready_bot_ids
            .iter()
            .map(|&id| matchmaking::Candidate {
                id,
                rating: self.rating(&stats, id).score(self.uncertainty_coefficient),
                matches_total: {
                    let played = stats.matches_played(id);
                    let queued = self.scheduled_matches_total.get(&id).copied().unwrap_or(0);
                    played + queued
                },
                matches_vs: ready_bot_ids
                    .iter()
                    .filter(|&opp_id| id != *opp_id)
                    .map(|opp_id| {
                        let played = stats.matches_played_vs(id, *opp_id);
                        let queued = self
                            .scheduled_matches_vs
                            .get(&(id, *opp_id))
                            .copied()
                            .unwrap_or(0);
                        (*opp_id, played + queued)
                    })
                    .collect(),
            })
            .collect_vec();

        let matches =
            matchmaking::create_match(&self.game_config, &self.matchmaking_config, &candidates);

        matches
            .into_iter()
            .map(|m| PlayMatchInput {
                bots: m
                    .bot_ids
                    .into_iter()
                    .map(|id| PlayMatchBot {
                        bot_id: id,
                        language: self
                            .bots
                            .iter()
                            .find(|b| b.id == id)
                            .unwrap()
                            .language
                            .clone(),
                    })
                    .collect_vec(),
                seed: m.seed,
            })
            .collect_vec()
    }

    fn record_scheduled_match(&mut self, input: &PlayMatchInput) {
        for bot in &input.bots {
            *self.scheduled_matches_total.entry(bot.bot_id).or_default() += 1;
        }

        for bot in &input.bots {
            for opp in &input.bots {
                if bot.bot_id != opp.bot_id {
                    *self
                        .scheduled_matches_vs
                        .entry((bot.bot_id, opp.bot_id))
                        .or_default() += 1;
                }
            }
        }
    }

    fn forget_scheduled_match(&mut self, input: &PlayMatchInput) {
        for bot in &input.bots {
            let entry = self
                .scheduled_matches_total
                .get_mut(&bot.bot_id)
                .expect("finished match must have been scheduled");
            *entry -= 1;
            if *entry == 0 {
                self.scheduled_matches_total.remove(&bot.bot_id);
            }
        }

        for bot in &input.bots {
            for opponent in &input.bots {
                if bot.bot_id == opponent.bot_id {
                    continue;
                }

                let entry = self
                    .scheduled_matches_vs
                    .get_mut(&(bot.bot_id, opponent.bot_id))
                    .expect("finished match pair must have been scheduled");
                *entry -= 1;
                if *entry == 0 {
                    self.scheduled_matches_vs
                        .remove(&(bot.bot_id, opponent.bot_id));
                }
            }
        }
    }

    fn recalculate_computed_full(&self) {
        self.global_leaderboard.recalculate();
        for lb in &self.custom_leaderboards {
            lb.recalculate();
        }
    }
}
