use crate::arena_commands::*;
use crate::async_leaderboard::AsyncLeaderboard;
use crate::config::{GameConfig, MatchmakingConfig, RankingConfig};
use crate::domain::*;
use crate::matchmaking;
use crate::ranking::Ranker;
use crate::worker::{BuildBotInput, PlayMatchBot, PlayMatchInput, WorkerHandle};
use crate::{chart, db};
use anyhow::{bail, Context};
use itertools::Itertools;
use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::{TryRecvError, TrySendError};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, instrument, warn};

pub async fn run(
    game_config: GameConfig,
    matchmaking_config: MatchmakingConfig,
    ranking_config: RankingConfig,
    pool: SqlitePool,
    worker_handle: WorkerHandle,
    mut commands_rx: Receiver<ArenaCommand>,
    cancellation_token: CancellationToken,
) -> anyhow::Result<JoinHandle<()>> {
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Cannot run db migrations")?;

    let ranker = Ranker::new(ranking_config);
    if game_config.max_players > 2 && !ranker.support_multi_team() {
        bail!("Configured ranking algorithm only supports 2 player games");
    }

    let mut arena = Arena::new(game_config, matchmaking_config, ranker, pool, worker_handle);

    arena
        .load_from_db()
        .await
        .context("Cannot load initial data from db")?;
    arena.reset_stale_builds().await;
    arena.recalculate_computed_full();

    let task_handle = tokio::spawn(async move {
        loop {
            // check cancellation
            if cancellation_token.is_cancelled() {
                break;
            }

            // handle commands
            let disconnected = loop {
                match commands_rx.try_recv() {
                    Ok(cmd) => {
                        arena.handle_command(cmd).await;
                    }
                    Err(TryRecvError::Empty) => break false,
                    Err(TryRecvError::Disconnected) => break true,
                }
            };
            if disconnected {
                break;
            }

            // time to let api return responses to clients
            tokio::time::sleep(Duration::from_millis(50)).await;

            if let Err(e) = arena.do_chores().await {
                eprintln!("Arena error: {:#}", e);
                eprintln!("Check logs for more details");
                break;
            }
        }
    });
    Ok(task_handle)
}

struct Arena {
    game_config: GameConfig,
    matchmaking_config: MatchmakingConfig,
    pool: SqlitePool,
    bots: Vec<Bot>,
    builds: Vec<Build>,
    worker_handle: WorkerHandle,
    ranker: Arc<Ranker>,
    global_leaderboard: AsyncLeaderboard,
    custom_leaderboards: Vec<AsyncLeaderboard>,
    match_queue: VecDeque<PlayMatchInput>,
    matchmaking_enabled: bool,
}

impl Arena {
    fn new(
        game_config: GameConfig,
        matchmaking_config: MatchmakingConfig,
        ranker: Ranker,
        pool: SqlitePool,
        worker_handle: WorkerHandle,
    ) -> Self {
        let ranker = Arc::new(ranker);
        Self {
            game_config,
            matchmaking_config,
            pool: pool.clone(),
            worker_handle,
            ranker: Arc::clone(&ranker),
            bots: Default::default(),
            builds: Default::default(),
            global_leaderboard: AsyncLeaderboard::new(Leaderboard::global(), ranker, pool),
            custom_leaderboards: Default::default(),
            match_queue: Default::default(),
            matchmaking_enabled: true,
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
            .map(|lb| AsyncLeaderboard::new(lb, Arc::clone(&self.ranker), self.pool.clone()))
            .collect();
        Ok(())
    }

    pub async fn do_chores(&mut self) -> anyhow::Result<()> {
        self.run_builds().await;

        if self.matchmaking_enabled {
            self.perform_matchmaking()?;
        }

        self.process_finished_matches().await;

        self.let_leaderboards_catchup_with_live_matches();

        Ok(())
    }

    pub async fn reset_stale_builds(&mut self) {
        // any running builds should be reset on startup
        for build in &mut self.builds {
            if build.is_running() {
                build.reset();
                db::persist_build(&self.pool, build)
                    .await
                    .expect("Cannot persist build to DB");
            }
        }

        // validate successful builds
        for build in &mut self.builds {
            let still_valid = self.worker_handle.known_bot_ids.contains(&build.bot_id);

            if build.was_finished_successfully() && !still_valid {
                build.reset();
                db::persist_build(&self.pool, build)
                    .await
                    .expect("Cannot persist build to DB");
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn run_builds(&mut self) {
        let mut inputs = Vec::new();
        for bot in &mut self.bots {
            for worker_name in std::iter::once(WorkerName::embedded()) {
                let existing_build = self
                    .builds
                    .iter_mut()
                    .find(|b| b.bot_id == bot.id && b.worker_name == worker_name);

                let build = match existing_build {
                    Some(build) if build.is_pending() => build,
                    None => {
                        self.builds.push(Build::new(bot.id, worker_name.clone()));
                        self.builds.last_mut().unwrap()
                    }
                    _ => continue,
                };

                build.make_running();
                db::persist_build(&self.pool, build)
                    .await
                    .expect("Cannot persist build to DB");
                inputs.push(BuildBotInput {
                    bot_id: bot.id,
                    worker_name: worker_name.clone(),
                    source_code: bot.source_code.clone(),
                    language: bot.language.clone(),
                })
            }
        }

        for input in inputs {
            let output = self.worker_handle.build_bot(input).await;
            if !self.bots.iter_mut().any(|b| b.id == output.bot_id) {
                warn!(
                    "Obtained build result for non-existent bot, skipping. {:?}",
                    output
                );
                continue;
            }

            let build = self
                .builds
                .iter_mut()
                .find(|b| b.bot_id == output.bot_id && b.worker_name == output.worker_name);

            let Some(build) = build else {
                warn!("Obtained build result for non-existent build, skipping");
                continue;
            };

            build.make_finished(output.result);
            db::persist_build(&self.pool, build)
                .await
                .expect("Cannot persist build to DB");
        }
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
        // builds would be automatically deleted by foreign link constraint
        // participations would be automatically deleted by foreign link constraint
        // matches would be automatically delete by db trigger
        db::delete_bot(&self.pool, id)
            .await
            .expect("Cannot delete bot from DB");
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
            .map(|bot| LeaderboardItem {
                id: bot.id,
                rank: self.rank(&stats, bot.id),
                rating: self.rating(&stats, bot.id),
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

        let lb = AsyncLeaderboard::new(leaderboard, Arc::clone(&self.ranker), self.pool.clone());
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
            .filter(|b| my_rating.score() < self.rating(stats, b.id).score())
            .count();
        stronger_bots_cnt
    }

    fn cmd_chart(&self, cmd: ChartCommand) {
        let ChartCommand {
            filter,
            attribute_name,
            response,
        } = cmd;
        let pool = self.pool.clone();

        tokio::spawn(async move {
            let res = chart::visualize(filter, attribute_name, pool).await;
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
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub fn perform_matchmaking(&mut self) -> anyhow::Result<()> {
        // hardcoded for now
        let mm_match_queue_size_threshold: usize = 20;

        while self.match_queue.len() < mm_match_queue_size_threshold {
            let new_matches = self.schedule_match();
            if new_matches.is_empty() {
                break;
            }
            self.match_queue.extend(new_matches);
        }

        while let Some(input) = self.match_queue.pop_front() {
            match self.worker_handle.match_tx.try_send(input) {
                Ok(_) => {}
                Err(TrySendError::Full(input)) => {
                    self.match_queue.push_front(input);
                    break;
                }
                Err(TrySendError::Closed(input)) => {
                    self.match_queue.push_front(input);
                    bail!("Cannot schedule a match, worker is closed.");
                }
            }
        }

        Ok(())
    }

    #[instrument(skip(self), level = "debug")]
    pub fn let_leaderboards_catchup_with_live_matches(&mut self) {
        self.global_leaderboard.catch_up_with_live_matches();
        for async_lb in &mut self.custom_leaderboards {
            async_lb.catch_up_with_live_matches();
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn process_finished_matches(&mut self) {
        while let Ok(output) = self.worker_handle.match_result_rx.try_recv() {
            // validation
            if output
                .participants
                .iter()
                .any(|p| self.bots.iter().all(|b| b.id != p.bot_id))
            {
                warn!(
                    "Match participant was deleted while match was running, ignoring match results"
                );
                continue;
            }

            let attributes = output
                .attributes
                .into_iter()
                .unique_by(|a| (a.name.clone(), a.bot_id, a.turn))
                .collect();

            let mut new_match = Match::new(output.seed, output.participants, attributes);

            new_match.attributes.retain(|attr| attr.name != "seed");
            new_match.attributes.push(MatchAttribute {
                name: "seed".to_string(),
                bot_id: None,
                turn: None,
                value: MatchAttributeValue::Integer(output.seed),
            });

            new_match.attributes.retain(|attr| attr.name != "index");
            new_match.attributes.retain(|attr| attr.name != "error");
            for (index, p) in new_match.participants.iter().enumerate() {
                new_match.attributes.push(MatchAttribute {
                    name: "index".to_string(),
                    bot_id: Some(p.bot_id),
                    turn: None,
                    value: MatchAttributeValue::Integer(index as _),
                });

                if p.error {
                    new_match.attributes.push(MatchAttribute {
                        name: "error".to_string(),
                        bot_id: Some(p.bot_id),
                        turn: None,
                        value: MatchAttributeValue::Integer(1),
                    });
                }
            }

            if self.game_config.min_players != self.game_config.max_players {
                new_match
                    .attributes
                    .retain(|attr| attr.name != "player_count");
                new_match.attributes.push(MatchAttribute {
                    name: "player_count".to_string(),
                    bot_id: None,
                    turn: None,
                    value: MatchAttributeValue::Integer(new_match.participants.len() as _),
                });
            }

            db::persist_match(&self.pool, &mut new_match)
                .await
                .expect("Cannot persist match to DB");

            let m = Arc::new(new_match);

            self.global_leaderboard.record_for_later(Arc::clone(&m));
            for leaderboard in &mut self.custom_leaderboards {
                leaderboard.record_for_later(Arc::clone(&m));
            }
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

        let candidates = self
            .bots
            .iter()
            .map(|b| b.id)
            .filter(|id| self.is_bot_ready_for_playing(*id))
            .map(|id| matchmaking::Candidate {
                id,
                matches_played: stats.matches_played(id),
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

    fn recalculate_computed_full(&self) {
        self.global_leaderboard.recalculate();
        for lb in &self.custom_leaderboards {
            lb.recalculate();
        }
    }
}
