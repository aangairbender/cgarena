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
            RankingConfig::OpenSkill(c) => Box::new(openskill::OpenSkill::new(c)),
            RankingConfig::TrueSkill(c) => Box::new(trueskill::Trueskill::new(c)),
            RankingConfig::Elo(c) => Box::new(elo::Elo::new(c)),
        };
        Self { algorithm }
    }

    pub fn support_multi_team(&self) -> bool {
        self.algorithm.support_multi_team()
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
    fn support_multi_team(&self) -> bool;
    fn default_rating(&self) -> Rating;
    fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating>;
}

pub mod openskill {
    use crate::domain::Rating;
    use crate::ranking::Algorithm;
    use itertools::Itertools;
    use serde::Deserialize;
    use serde::Serialize;
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

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub beta: Option<f64>,
        pub uncertainty_tolerance: Option<f64>,
    }

    impl From<Config> for WengLinConfig {
        fn from(value: Config) -> Self {
            let default = WengLinConfig::default();
            WengLinConfig {
                beta: value.beta.unwrap_or(default.beta),
                uncertainty_tolerance: value
                    .uncertainty_tolerance
                    .unwrap_or(default.uncertainty_tolerance),
            }
        }
    }

    pub struct OpenSkill {
        config: WengLinConfig,
    }

    impl OpenSkill {
        pub fn new(config: Config) -> Self {
            Self {
                config: config.into(),
            }
        }
    }

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

            let new_ratings = weng_lin_multi_team(&teams_and_ranks, &self.config);

            new_ratings.into_iter().map(|r| r[0].into()).collect_vec()
        }

        fn support_multi_team(&self) -> bool {
            true
        }
    }
}

pub mod trueskill {
    use crate::{domain::Rating, ranking::Algorithm};
    use itertools::Itertools;
    use serde::{Deserialize, Serialize};
    use skillratings::{trueskill::*, MultiTeamOutcome};

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub draw_probability: Option<f64>,
        pub beta: Option<f64>,
        pub default_dynamics: Option<f64>,
    }

    impl From<Config> for TrueSkillConfig {
        fn from(value: Config) -> Self {
            let default = TrueSkillConfig::default();
            TrueSkillConfig {
                draw_probability: value.draw_probability.unwrap_or(default.draw_probability),
                beta: value.beta.unwrap_or(default.beta),
                default_dynamics: value.default_dynamics.unwrap_or(default.default_dynamics),
            }
        }
    }

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

    pub struct Trueskill {
        config: TrueSkillConfig,
    }

    impl Trueskill {
        pub fn new(config: Config) -> Self {
            Self {
                config: config.into(),
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

            let new_ratings = trueskill_multi_team(&teams_and_ranks, &self.config);

            new_ratings.into_iter().map(|r| r[0].into()).collect_vec()
        }

        fn support_multi_team(&self) -> bool {
            true
        }
    }
}

pub mod elo {
    use crate::{domain::Rating, ranking::Algorithm};
    use serde::{Deserialize, Serialize};
    use skillratings::{elo::*, Outcomes};

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub k: Option<f64>,
    }

    impl From<Config> for EloConfig {
        fn from(value: Config) -> Self {
            let default = EloConfig::default();
            EloConfig {
                k: value.k.unwrap_or(default.k),
            }
        }
    }

    impl From<Rating> for EloRating {
        fn from(rating: Rating) -> Self {
            Self { rating: rating.mu }
        }
    }

    impl From<EloRating> for Rating {
        fn from(rating: EloRating) -> Self {
            Rating {
                mu: rating.rating,
                sigma: 0.0,
            }
        }
    }

    pub struct Elo {
        config: EloConfig,
    }

    impl Elo {
        pub fn new(config: Config) -> Self {
            Self {
                config: config.into(),
            }
        }
    }

    impl Algorithm for Elo {
        fn default_rating(&self) -> Rating {
            EloRating::default().into()
        }

        fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating> {
            assert_eq!(input.len(), 2);
            let p1 = input[0].0.into();
            let p2 = input[1].0.into();
            let outcome = match input[0].1.cmp(&input[1].1) {
                std::cmp::Ordering::Less => Outcomes::WIN,
                std::cmp::Ordering::Equal => Outcomes::DRAW,
                std::cmp::Ordering::Greater => Outcomes::LOSS,
            };

            let (new_p1, new_p2) = elo(&p1, &p2, &outcome, &self.config);
            vec![new_p1.into(), new_p2.into()]
        }

        fn support_multi_team(&self) -> bool {
            false
        }
    }
}
