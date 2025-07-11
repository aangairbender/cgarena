use crate::attribute_index::AttributeIndex;
use crate::config::{GameConfig, MatchmakingConfig, RankingConfig};
use crate::db::Database;
use crate::domain::{
    Bot, BotId, BotName, Build, Language, Leaderboard, LeaderboardId, LeaderboardName, Match,
    MatchFilter, Rating, SourceCode, WorkerName,
};
use crate::matchmaking;
use crate::ranking::Ranker;
use crate::worker::{BuildBotInput, PlayMatchBot, PlayMatchInput, WorkerHandle};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::{instrument, warn};

pub enum ArenaCommand {
    CreateBot(CreateBotCommand),
    DeleteBot(DeleteBotCommand),
    RenameBot(RenameBotCommand),
    FetchLeaderboard(FetchLeaderboardCommand),
    FetchBots(FetchBotsCommand),
    CreateLeaderboard(CreateLeaderboardCommand),
    DeleteLeaderboard(DeleteLeaderboardCommand),
    RenameLeaderboard(RenameLeaderboardCommand),
}

pub struct CreateLeaderboardCommand {
    pub name: LeaderboardName,
    pub filter: MatchFilter,
    pub response: oneshot::Sender<LeaderboardId>,
}

pub struct RenameLeaderboardCommand {
    pub id: LeaderboardId,
    pub new_name: LeaderboardName,
    pub response: oneshot::Sender<RenameLeaderboardResult>,
}

pub enum RenameLeaderboardResult {
    Renamed,
    NotFound,
}

pub struct DeleteLeaderboardCommand {
    pub id: LeaderboardId,
    pub response: oneshot::Sender<()>,
}

pub struct FetchBotsCommand {
    pub response: oneshot::Sender<Vec<BotMinimal>>,
}

pub struct RenameBotCommand {
    pub id: BotId,
    pub new_name: BotName,
    pub response: oneshot::Sender<RenameBotResult>,
}

pub enum RenameBotResult {
    Renamed(BotMinimal),
    DuplicateName,
    NotFound,
}

#[derive(PartialEq, Eq, Debug)]
pub struct BotMinimal {
    pub id: BotId,
    pub name: BotName,
}

pub struct CreateBotCommand {
    pub name: BotName,
    pub source_code: SourceCode,
    pub language: Language,
    pub response: oneshot::Sender<CreateBotResult>,
}

pub enum CreateBotResult {
    Created(BotMinimal),
    DuplicateName,
}

pub struct DeleteBotCommand {
    pub id: BotId,
    pub response: oneshot::Sender<()>,
}

pub struct FetchLeaderboardCommand {
    pub bot_id: BotId,
    pub response: oneshot::Sender<Option<FetchLeaderboardResult>>,
}

pub struct FetchLeaderboardResult {
    pub bot_overview: LeaderboardBotOverview,
    pub items: Vec<LeaderboardItem>,
    pub attribute_index: AttributeIndex,
}

pub struct LeaderboardBotOverview {
    pub id: BotId,
    pub name: BotName,
    pub language: Language,
    pub rating: Rating,
    pub matches_played: u64,
    pub matches_with_error: u64,
    pub builds: Vec<Build>,
}

pub struct LeaderboardItem {
    pub id: BotId,
    pub rank: usize,
    pub name: BotName,
    pub rating: Rating,
    pub wins: usize,
    pub loses: usize,
    pub draws: usize,
    pub created_at: DateTime<Utc>,
}

pub async fn run(
    game_config: GameConfig,
    matchmaking_config: MatchmakingConfig,
    ranking_config: RankingConfig,
    db: Database,
    worker_handle: WorkerHandle,
    mut commands_rx: Receiver<ArenaCommand>,
    cancellation_token: CancellationToken,
) {
    let ranker = Ranker::new(ranking_config);
    let mut arena = Arena::new(game_config, matchmaking_config, ranker, db, worker_handle);

    arena.load_from_db().await;
    arena.reset_stale_builds().await;
    arena.recalculate_computed_full();

    loop {
        // 0. check cancellation
        if cancellation_token.is_cancelled() {
            break;
        }

        // 1. handle commands
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

        arena.do_chores().await;
    }
}

struct Arena {
    game_config: GameConfig,
    matchmaking_config: MatchmakingConfig,
    db: Database,
    bots: Vec<Bot>,
    matches: Vec<Match>,
    builds: Vec<Build>,
    worker_handle: WorkerHandle,
    ranker: Ranker,
    global_leaderboard: Leaderboard,
    custom_leaderboards: Vec<Leaderboard>,
    attribute_index: AttributeIndex,
    match_queue: VecDeque<PlayMatchInput>,
}

impl Arena {
    fn new(
        game_config: GameConfig,
        matchmaking_config: MatchmakingConfig,
        ranker: Ranker,
        db: Database,
        worker_handle: WorkerHandle,
    ) -> Self {
        Self {
            game_config,
            matchmaking_config,
            db,
            worker_handle,
            ranker,
            bots: Default::default(),
            matches: Default::default(),
            builds: Default::default(),
            global_leaderboard: Leaderboard::global(),
            custom_leaderboards: Default::default(),
            attribute_index: Default::default(),
            match_queue: Default::default(),
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn do_chores(&mut self) {
        // 2. run builds
        self.run_builds().await;

        // 4. perform matchmaking
        self.perform_matchmaking();

        // 5. process finished matches
        self.process_finished_matches().await;

        // 7. (future) update views
    }

    #[instrument(skip(self))]
    pub async fn load_from_db(&mut self) {
        self.bots = self.db.fetch_bots().await;
        self.matches = self.db.fetch_matches().await;
        self.builds = self.db.fetch_builds().await;
        self.custom_leaderboards = self.db.fetch_leaderboards().await;
    }

    pub async fn reset_stale_builds(&mut self) {
        // any running builds should be reset on startup
        for build in &mut self.builds {
            if build.is_running() {
                build.reset();
                self.db.persist_build(build).await;
            }
        }

        // validate successful builds
        for build in &mut self.builds {
            let still_valid = self.worker_handle.known_bot_ids.contains(&build.bot_id);

            if build.was_finished_successfully() && !still_valid {
                build.reset();
                self.db.persist_build(build).await;
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
                self.db.persist_build(build).await;
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
                .find(|b| b.bot_id == output.bot_id && b.worker_name == output.worker_name)
                .expect("Finished build should already exist in a running state");

            build.make_finished(output.result);
            self.db.persist_build(build).await;
        }
    }

    #[instrument(skip(self, source_code))]
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
        self.db.persist_bot(&mut bot).await;
        let bot_minimal = BotMinimal {
            id: bot.id,
            name: bot.name.clone(),
        };
        self.bots.push(bot);
        CreateBotResult::Created(bot_minimal)
    }

    #[instrument(skip(self), level = "debug")]
    async fn cmd_rename_bot(&mut self, id: BotId, new_name: BotName) -> RenameBotResult {
        if self.bots.iter().any(|b| b.id != id && b.name == new_name) {
            return RenameBotResult::DuplicateName;
        }

        let Some(bot) = self.bots.iter_mut().find(|b| b.id == id) else {
            return RenameBotResult::NotFound;
        };

        bot.name = new_name;
        self.db.persist_bot(bot).await;
        let bot_minimal = BotMinimal {
            id: bot.id,
            name: bot.name.clone(),
        };
        RenameBotResult::Renamed(bot_minimal)
    }

    #[instrument(skip(self))]
    async fn cmd_delete_bot(&mut self, id: BotId) {
        // builds would be automatically deleted by foreign link constraint
        // participations would be automatically deleted by foreign link constraint
        // matches would be automatically delete by db trigger
        self.db.delete_bot(id).await;
        self.bots.retain(|bot| bot.id != id);
        self.matches
            .retain(|m| !m.participants.iter().any(|p| p.bot_id == id));
        self.builds.retain(|b| b.bot_id != id);
        self.recalculate_computed_full();
    }

    #[instrument(skip(self), level = "debug")]
    async fn cmd_fetch_bots(&mut self) -> Vec<BotMinimal> {
        let mut bots = self
            .bots
            .iter()
            .map(|b| BotMinimal {
                id: b.id,
                name: b.name.clone(),
            })
            .collect_vec();
        // sort+rev so that bot with the biggest id is first in the list
        bots.sort_by_key::<i64, _>(|b| b.id.into());
        bots.reverse();
        bots
    }

    #[instrument(skip(self), level = "debug")]
    async fn cmd_fetch_leaderboard(&mut self, target_id: BotId) -> Option<FetchLeaderboardResult> {
        let target = self.bots.iter().find(|b| b.id == target_id)?;

        let bot_overview = LeaderboardBotOverview {
            id: target.id,
            name: target.name.clone(),
            language: target.language.clone(),
            rating: self.rating(target.id),
            matches_played: self.global_leaderboard.stats.matches_played(target.id),
            matches_with_error: self.global_leaderboard.stats.matches_with_error(target.id),
            builds: self
                .builds
                .iter()
                .filter(|b| b.bot_id == target_id)
                .cloned()
                .collect(),
        };

        let mut items = Vec::with_capacity(self.bots.len());
        for bot in &self.bots {
            let rating = self.rating(bot.id);
            let stronger_bots_cnt = self
                .bots
                .iter()
                .filter(|b| rating.score() < self.rating(b.id).score())
                .count();

            let mut wins = 0;
            let mut loses = 0;
            let mut draws = 0;

            if target_id != bot.id {
                for m in &self.matches {
                    let Some(p_target) = m.participants.iter().find(|p| p.bot_id == target_id)
                    else {
                        continue;
                    };
                    let Some(p_current) = m.participants.iter().find(|p| p.bot_id == bot.id) else {
                        continue;
                    };

                    match p_target.rank.cmp(&p_current.rank) {
                        Ordering::Less => wins += 1,
                        Ordering::Equal => draws += 1,
                        Ordering::Greater => loses += 1,
                    }
                }
            }

            let item = LeaderboardItem {
                id: bot.id,
                rank: 1 + stronger_bots_cnt,
                name: bot.name.clone(),
                rating,
                wins,
                loses,
                draws,
                created_at: bot.created_at,
            };
            items.push(item);
        }
        items.sort_by_key(|i| i.rank);
        Some(FetchLeaderboardResult {
            bot_overview,
            items,
            attribute_index: self.attribute_index.clone(),
        })
    }

    async fn cmd_create_leaderboard(
        &mut self,
        name: LeaderboardName,
        filter: MatchFilter,
    ) -> LeaderboardId {
        let mut leaderboard = Leaderboard::new(name, filter);
        self.db.persist_leaderboard(&mut leaderboard).await;
        let id = leaderboard.id;
        self.custom_leaderboards.push(leaderboard);
        id
    }

    async fn cmd_rename_leaderboard(
        &mut self,
        id: LeaderboardId,
        new_name: LeaderboardName,
    ) -> RenameLeaderboardResult {
        let Some(leaderboard) = self.custom_leaderboards.iter_mut().find(|w| w.id == id) else {
            return RenameLeaderboardResult::NotFound;
        };

        leaderboard.name = new_name;
        self.db.persist_leaderboard(leaderboard).await;
        RenameLeaderboardResult::Renamed
    }

    async fn cmd_delete_leaderboard(&mut self, id: LeaderboardId) {
        self.db.delete_leaderboard(id).await;
        self.custom_leaderboards.retain(|w| w.id != id);
    }

    fn rating(&self, id: BotId) -> Rating {
        self.global_leaderboard
            .stats
            .rating(id)
            .unwrap_or_else(|| self.ranker.default_rating())
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
            ArenaCommand::FetchBots(command) => {
                let res = self.cmd_fetch_bots().await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
            ArenaCommand::FetchLeaderboard(command) => {
                let res = self.cmd_fetch_leaderboard(command.bot_id).await;
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
            ArenaCommand::RenameLeaderboard(command) => {
                let res = self
                    .cmd_rename_leaderboard(command.id, command.new_name)
                    .await;
                if command.response.send(res).is_err() {
                    warn!("Failed to send response to client");
                }
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub fn perform_matchmaking(&mut self) {
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
            match self.worker_handle.match_tx.try_reserve() {
                Ok(permit) => {
                    permit.send(input);
                }
                Err(_) => {
                    self.match_queue.push_front(input);
                    break;
                }
            }
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

            let mut new_match = Match::new(output.seed, output.participants, output.attributes);
            self.db.persist_match(&mut new_match).await;
            self.matches.push(new_match);

            self.global_leaderboard
                .process(&self.ranker, self.matches.last().unwrap());
            for leaderboard in &mut self.custom_leaderboards {
                leaderboard.process(&self.ranker, self.matches.last().unwrap());
            }
            self.attribute_index.process(self.matches.last().unwrap());
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
        let candidates = self
            .bots
            .iter()
            .map(|b| b.id)
            .filter(|id| self.is_bot_ready_for_playing(*id))
            .map(|id| matchmaking::Candidate {
                id,
                matches_played: self.global_leaderboard.stats.matches_played(id),
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

    #[instrument(skip(self))]
    fn recalculate_computed_full(&mut self) {
        self.global_leaderboard.reset();
        self.attribute_index.reset();
        for m in &self.matches {
            self.global_leaderboard.process(&self.ranker, m);
            for leaderboard in &mut self.custom_leaderboards {
                leaderboard.process(&self.ranker, m);
            }
            self.attribute_index.process(m);
        }
    }
}
