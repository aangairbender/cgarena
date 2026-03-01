use crate::config::RankingConfig;
use crate::domain::{BotId, Match, Rating};
use crate::ranking::algorithms::{elo, openskill, trueskill};
use crate::ranking::Algorithm;
use itertools::Itertools;
use std::collections::HashMap;

pub struct Ranker {
    algorithm: Box<dyn Algorithm + Sync + Send>,
}

impl Ranker {
    pub fn new(config: RankingConfig) -> Ranker {
        let algorithm: Box<dyn Algorithm + Sync + Send> = match config {
            RankingConfig::OpenSkill(c) => Box::new(openskill::OpenSkill::new(c)),
            RankingConfig::TrueSkill(c) => Box::new(trueskill::Trueskill::new(c)),
            RankingConfig::Elo(c) => Box::new(elo::Elo::new(c)),
        };
        Self { algorithm }
    }

    pub fn support_multi_team(&self) -> bool {
        self.algorithm.supports_multi_team()
    }

    pub fn default_rating(&self) -> Rating {
        self.algorithm.default_rating()
    }

    pub fn recalc_rating(&self, ratings: &mut HashMap<BotId, Rating>, m: &Match) {
        let ps = m
            .participants
            .iter()
            .map(|p| {
                (
                    ratings
                        .get(&p.bot_id)
                        .copied()
                        .unwrap_or_else(|| self.algorithm.default_rating()),
                    p.rank,
                )
            })
            .collect_vec();

        let new_ratings = self.algorithm.recalc_ratings(&ps);

        m.participants
            .iter()
            .zip_eq(new_ratings)
            .for_each(|(p, new_rating)| {
                ratings.insert(p.bot_id, new_rating);
            });
    }
}
