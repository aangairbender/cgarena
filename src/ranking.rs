use crate::config::RankingConfig;
use crate::domain::{BotId, Match, Rating};
use itertools::Itertools;
use std::collections::HashMap;

pub struct Ranker {
    algorithm: Box<dyn Algorithm + Sync + Send>,
}

impl Ranker {
    pub fn new(config: RankingConfig) -> Ranker {
        let algorithm: Box<dyn Algorithm + Sync + Send> = match config {
            RankingConfig::OpenSkill => Box::new(openskill::OpenSkill),
            RankingConfig::TrueSkill => Box::new(trueskill::Trueskill),
        };
        Self { algorithm }
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

trait Algorithm {
    fn default_rating(&self) -> Rating;
    fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating>;
}

mod openskill {
    use crate::domain::Rating;
    use crate::ranking::Algorithm;
    use itertools::Itertools;
    use skillratings::MultiTeamOutcome;

    use skillratings::weng_lin::*;

    impl From<Rating> for WengLinRating {
        fn from(rating: Rating) -> Self {
            Self {
                rating: rating.mu,
                uncertainty: rating.sigma,
            }
        }
    }

    impl From<WengLinRating> for Rating {
        fn from(rating: WengLinRating) -> Self {
            Rating {
                mu: rating.rating,
                sigma: rating.uncertainty,
            }
        }
    }

    pub struct OpenSkill;

    impl Algorithm for OpenSkill {
        fn default_rating(&self) -> Rating {
            WengLinRating::default().into()
        }

        fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating> {
            let teams: Vec<Vec<WengLinRating>> =
                input.iter().map(|w| vec![w.0.into()]).collect_vec();
            let ranks = input
                .iter()
                .map(|w| MultiTeamOutcome::new(w.1 as usize))
                .collect_vec();

            let teams_and_ranks = teams
                .iter()
                .zip_eq(ranks)
                .map(|(t, r)| (t.as_slice(), r))
                .collect_vec();

            let new_ratings = weng_lin_multi_team(&teams_and_ranks, &WengLinConfig::default());

            new_ratings.into_iter().map(|r| r[0].into()).collect_vec()
        }
    }
}

mod trueskill {
    use crate::{domain::Rating, ranking::Algorithm};
    use itertools::Itertools;
    use skillratings::{trueskill::*, MultiTeamOutcome};

    pub struct Trueskill;

    impl From<Rating> for TrueSkillRating {
        fn from(rating: Rating) -> Self {
            Self {
                rating: rating.mu,
                uncertainty: rating.sigma,
            }
        }
    }

    impl From<TrueSkillRating> for Rating {
        fn from(rating: TrueSkillRating) -> Self {
            Rating {
                mu: rating.rating,
                sigma: rating.uncertainty,
            }
        }
    }

    impl Algorithm for Trueskill {
        fn default_rating(&self) -> Rating {
            TrueSkillRating::default().into()
        }

        fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating> {
            let teams: Vec<Vec<TrueSkillRating>> = input.iter().map(|w| vec![w.0.into()]).collect();
            let ranks = input
                .iter()
                .map(|w| MultiTeamOutcome::new(w.1 as usize))
                .collect_vec();

            let teams_and_ranks = teams
                .iter()
                .zip_eq(ranks)
                .map(|(t, r)| (t.as_slice(), r))
                .collect_vec();

            let new_ratings = trueskill_multi_team(&teams_and_ranks, &TrueSkillConfig::default());

            new_ratings.into_iter().map(|r| r[0].into()).collect_vec()
        }
    }
}
