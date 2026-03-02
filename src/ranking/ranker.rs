use crate::config::RankingConfig;
use crate::domain::{BotId, Match, Rating, WinrateStats};
use crate::ranking::algorithms::{bradley_terry, elo, openskill, trueskill};
use crate::ranking::{BatchAlgorithm, OnlineAlgorithm};
use itertools::Itertools;
use std::collections::HashMap;

pub struct Ranker {
    strategy: RankingStrategy,
}

enum RankingStrategy {
    Online(Box<dyn OnlineAlgorithm + Sync + Send>),
    Batch(Box<dyn BatchAlgorithm + Sync + Send>),
}

#[derive(Copy, Clone)]
pub enum RankingStrategyKind {
    Online,
    Batch,
}

impl Ranker {
    pub fn new(config: RankingConfig) -> Ranker {
        let strategy = match config {
            RankingConfig::OpenSkill(c) => {
                RankingStrategy::Online(Box::new(openskill::OpenSkill::new(c)))
            }
            RankingConfig::TrueSkill(c) => {
                RankingStrategy::Online(Box::new(trueskill::Trueskill::new(c)))
            }
            RankingConfig::Elo(c) => RankingStrategy::Online(Box::new(elo::Elo::new(c))),
            RankingConfig::BradleyTerry(c) => {
                RankingStrategy::Batch(Box::new(bradley_terry::BradleyTerry::new(c)))
            }
        };
        Self { strategy }
    }

    pub fn support_multi_team(&self) -> bool {
        match &self.strategy {
            RankingStrategy::Online(algorithm) => algorithm.supports_multi_team(),
            RankingStrategy::Batch(algorithm) => algorithm.supports_multi_team(),
        }
    }

    pub fn default_rating(&self) -> Rating {
        match &self.strategy {
            RankingStrategy::Online(algorithm) => algorithm.default_rating(),
            RankingStrategy::Batch(algorithm) => algorithm.default_rating(),
        }
    }

    pub fn strategy_kind(&self) -> RankingStrategyKind {
        match &self.strategy {
            RankingStrategy::Online(_) => RankingStrategyKind::Online,
            RankingStrategy::Batch(_) => RankingStrategyKind::Batch,
        }
    }

    pub fn recalc_rating_batch(
        &self,
        winrate_stats: &HashMap<(BotId, BotId), WinrateStats>,
    ) -> HashMap<BotId, Rating> {
        let RankingStrategy::Batch(algorithm) = &self.strategy else {
            panic!("recalc_rating_batch called on non-batch strategy")
        };

        algorithm.recalc_batch(winrate_stats)
    }

    pub fn recalc_rating(&self, ratings: &mut HashMap<BotId, Rating>, m: &Match) {
        let RankingStrategy::Online(algorithm) = &self.strategy else {
            panic!("recalc_rating called on non-online strategy")
        };

        let default_rating = self.default_rating();

        let ps = m
            .participants
            .iter()
            .map(|p| {
                (
                    ratings.get(&p.bot_id).copied().unwrap_or(default_rating),
                    p.rank,
                )
            })
            .collect_vec();

        let new_ratings = algorithm.recalc_ratings(&ps);

        m.participants
            .iter()
            .zip_eq(new_ratings)
            .for_each(|(p, new_rating)| {
                ratings.insert(p.bot_id, new_rating);
            });
    }
}
