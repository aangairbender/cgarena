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
            RankingConfig::Elo => Box::new(elo::Elo),
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

        fn support_multi_team(&self) -> bool {
            true
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

        fn support_multi_team(&self) -> bool {
            true
        }
    }
}

mod elo {
    use crate::{domain::Rating, ranking::Algorithm};
    use skillratings::{elo::*, Outcomes};

    pub struct Elo;

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

            let (new_p1, new_p2) = elo(&p1, &p2, &outcome, &EloConfig::default());
            vec![new_p1.into(), new_p2.into()]
        }

        fn support_multi_team(&self) -> bool {
            false
        }
    }
}
