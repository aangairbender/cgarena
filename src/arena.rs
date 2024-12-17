use crate::config::Config;
use crate::db::Database;
use crate::domain::{Bot, BotId, BotName, Build, Language, Match, Rating, SourceCode, WorkerName};
use crate::embedded_worker::{BuildBotInput, EmbeddedWorker, PlayMatchBot, PlayMatchInput};
use crate::ranking;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::Receiver;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tracing::warn;

pub enum ArenaCommand {
    CreateBot(CreateBotCommand),
    DeleteBot(DeleteBotCommand),
    FetchLeaderboard(FetchLeaderboardCommand),
    FetchBots(FetchBotsCommand),
}

pub struct FetchBotsCommand {
    pub response: oneshot::Sender<Vec<BotMinimal>>,
}

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
    Created(BotId),
    DuplicateName,
}

pub struct DeleteBotCommand {
    pub id: BotId,
}

pub struct FetchLeaderboardCommand {
    pub bot_id: BotId,
    pub response: oneshot::Sender<Option<FetchLeaderboardResult>>,
}

pub struct FetchLeaderboardResult {
    pub bot_overview: LeaderboardBotOverview,
    pub items: Vec<LeaderboardItem>,
}

pub struct LeaderboardBotOverview {
    pub id: BotId,
    pub name: BotName,
    pub language: Language,
    pub rating: Rating,
    pub matches_played: usize,
    pub matches_with_error: usize,
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
    config: Config,
    db: Database,
    worker: EmbeddedWorker,
    mut commands_rx: Receiver<ArenaCommand>,
    cancellation_token: CancellationToken,
) {
    let mut arena = Arena::new(config, db, worker);

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
                Ok(cmd) => arena.handle_command(cmd).await,
                Err(TryRecvError::Empty) => break false,
                Err(TryRecvError::Disconnected) => break true,
            }
        };
        if disconnected {
            break;
        }

        // 2. run builds
        arena.run_builds().await;

        // 4. perform matchmaking
        arena.perform_matchmaking();

        // 5. process finished matches
        arena.process_finished_matches().await;

        // 7. (future) update views
    }
}

struct Arena {
    config: Config,
    db: Database,
    bots: Vec<Bot>,
    matches: Vec<Match>,
    builds: Vec<Build>,
    worker: EmbeddedWorker,
    computed_stats: ComputedStats,
    match_queue: VecDeque<PlayMatchInput>,
}

impl Arena {
    pub fn new(config: Config, db: Database, worker: EmbeddedWorker) -> Self {
        Self {
            config,
            db,
            worker,
            bots: Default::default(),
            matches: Default::default(),
            builds: Default::default(),
            computed_stats: Default::default(),
            match_queue: Default::default(),
        }
    }

    pub async fn load_from_db(&mut self) {
        self.bots = self.db.fetch_bots().await;
        self.matches = self.db.fetch_matches().await;
        self.builds = self.db.fetch_builds().await;
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
            let still_valid = self.worker.is_build_valid(build.bot_id).await;

            if build.was_finished_successfully() && !still_valid {
                build.reset();
                self.db.persist_build(build).await;
            }
        }
    }

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
            let output = self.worker.build_bot(input).await;
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

    pub async fn handle_command(&mut self, command: ArenaCommand) {
        match command {
            ArenaCommand::CreateBot(command) => {
                let mut bot = Bot::new(command.name, command.source_code, command.language);
                self.db.persist_bot(&mut bot).await;
                self.bots.push(bot);
            }
            ArenaCommand::DeleteBot(command) => {
                // matches should be automatically deleted by foreign link constraint
                // builds should be automatically deleted by foreign link constraint
                self.db.delete_bot(command.id).await;
                self.bots.retain(|bot| bot.id != command.id);
                self.matches
                    .retain(|m| !m.participants.iter().any(|p| p.bot_id == command.id));
                self.builds.retain(|b| b.bot_id != command.id);
                self.recalculate_computed_full();
            }
            ArenaCommand::FetchBots(command) => {
                let res = self
                    .bots
                    .iter()
                    .map(|b| BotMinimal {
                        id: b.id,
                        name: b.name.clone(),
                    })
                    .collect_vec();
                let _ = command.response.send(res);
            }
            ArenaCommand::FetchLeaderboard(command) => {
                let target_id = command.bot_id;
                let Some(target) = self.bots.iter().find(|b| b.id == target_id) else {
                    let _ = command.response.send(None);
                    return;
                };

                let bot_overview = LeaderboardBotOverview {
                    id: target.id,
                    name: target.name.clone(),
                    language: target.language.clone(),
                    rating: self.computed_stats.rating(target.id),
                    matches_played: self.computed_stats.matches_played(target.id),
                    matches_with_error: self.computed_stats.matches_with_error(target.id),
                };

                let mut items = Vec::with_capacity(self.bots.len());
                for bot in &self.bots {
                    let rating = self.computed_stats.rating(bot.id);
                    let stronger_bots_cnt = self
                        .bots
                        .iter()
                        .filter(|b| rating.score() < self.computed_stats.rating(b.id).score())
                        .count();

                    let mut wins = 0;
                    let mut loses = 0;
                    let mut draws = 0;

                    if target_id != bot.id {
                        for m in &self.matches {
                            let Some(p_target) =
                                m.participants.iter().find(|p| p.bot_id == target_id)
                            else {
                                continue;
                            };
                            let Some(p_current) =
                                m.participants.iter().find(|p| p.bot_id == bot.id)
                            else {
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
                let res = FetchLeaderboardResult {
                    bot_overview,
                    items,
                };
                let _ = command.response.send(Some(res));
            }
        }
    }

    pub fn perform_matchmaking(&mut self) {
        let mm_match_queue_size_threshold = self.worker.config.threads as usize * 2;

        while self.match_queue.len() < mm_match_queue_size_threshold {
            let Some(new_matches) = self.schedule_match() else {
                break;
            };
            self.match_queue.extend(new_matches);
        }

        while let Some(input) = self.match_queue.pop_front() {
            match self.worker.match_tx.try_reserve() {
                Ok(permit) => {
                    permit.send(input);
                }
                Err(_) => self.match_queue.push_front(input),
            }
        }
    }

    pub async fn process_finished_matches(&mut self) {
        while let Ok(output) = self.worker.match_result_rx.try_recv() {
            let mut new_match = Match::new(output.seed, output.participants);
            self.db.persist_match(&mut new_match).await;
            self.matches.push(new_match);

            self.computed_stats
                .recalc_after_matches(&self.config, self.matches.last().into_iter());
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

    fn schedule_match(&self) -> Option<Vec<PlayMatchInput>> {
        let mut rng = thread_rng();

        let bot_ids = self
            .bots
            .iter()
            .map(|b| b.id)
            .filter(|id| self.is_bot_ready_for_playing(*id))
            .collect_vec();

        if bot_ids.len() < self.config.game.min_players as usize {
            return None;
        }

        let bot_ids_min_matches = bot_ids
            .iter()
            .copied()
            .filter(|id| {
                self.computed_stats.matches_played(*id) < self.config.matchmaking.min_matches as _
            })
            .collect::<Vec<_>>();

        let first_bot_id = if !bot_ids_min_matches.is_empty()
            && rng.gen::<f64>() < self.config.matchmaking.min_matches_preference
        {
            bot_ids_min_matches[rng.gen_range(0..bot_ids_min_matches.len())]
        } else {
            bot_ids[rng.gen_range(0..bot_ids.len())]
        };

        let n_players =
            rng.gen_range(self.config.game.min_players..=self.config.game.max_players) as usize;
        let mut players = Vec::with_capacity(n_players);
        players.push(first_bot_id);
        while players.len() < n_players {
            let next_bot_id = loop {
                let candidate_id = bot_ids[rng.gen_range(0..bot_ids.len())];
                if !players.contains(&candidate_id) {
                    break candidate_id;
                }
            };

            players.push(next_bot_id);
        }
        players.shuffle(&mut rng);
        let scheduled_match = PlayMatchInput {
            seed: rng.gen(),
            bots: players
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
                .collect(),
        };

        let res = if self.config.game.symmetric {
            vec![scheduled_match]
        } else {
            let n = scheduled_match.bots.len();
            scheduled_match
                .bots
                .into_iter()
                .permutations(n)
                .map(|p| PlayMatchInput {
                    seed: scheduled_match.seed,
                    bots: p,
                })
                .collect()
        };
        Some(res)
    }

    fn recalculate_computed_full(&mut self) {
        self.computed_stats.clear();
        self.computed_stats
            .recalc_after_matches(&self.config, self.matches.iter());
    }
}

#[derive(Default)]
struct ComputedStats {
    ratings: HashMap<BotId, Rating>,
    matches_played: HashMap<BotId, usize>,
    matches_with_error: HashMap<BotId, usize>,
}

impl ComputedStats {
    pub fn clear(&mut self) {
        *self = Default::default();
    }

    pub fn recalc_after_matches<'a>(
        &mut self,
        config: &Config,
        matches: impl Iterator<Item = &'a Match> + Clone,
    ) {
        // rating
        ranking::recalc_rating(&config.ranking, &mut self.ratings, matches.clone());

        // matches_played and matches_with_error
        for m in matches.clone() {
            for p in &m.participants {
                self.matches_played
                    .entry(p.bot_id)
                    .and_modify(|w| *w += 1)
                    .or_insert(1);

                if p.error {
                    self.matches_with_error
                        .entry(p.bot_id)
                        .and_modify(|w| *w += 1)
                        .or_insert(1);
                }
            }
        }
    }

    pub fn rating(&self, id: BotId) -> Rating {
        self.ratings.get(&id).cloned().unwrap_or_default()
    }

    pub fn matches_played(&self, id: BotId) -> usize {
        self.matches_played.get(&id).copied().unwrap_or_default()
    }

    pub fn matches_with_error(&self, id: BotId) -> usize {
        self.matches_with_error
            .get(&id)
            .copied()
            .unwrap_or_default()
    }
}
