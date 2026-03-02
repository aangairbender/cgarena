use crate::domain::Rating;
use crate::ranking::{Algorithm, OnlineAlgorithm};
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
    fn supports_multi_team(&self) -> bool {
        true
    }

    fn default_rating(&self) -> Rating {
        TrueSkillRating::default().into()
    }
}

impl OnlineAlgorithm for Trueskill {
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
}
