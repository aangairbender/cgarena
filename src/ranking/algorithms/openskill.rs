use crate::domain::Rating;
use itertools::Itertools;
use serde::Deserialize;
use serde::Serialize;
use skillratings::MultiTeamOutcome;

use crate::ranking::Algorithm;
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
    fn supports_multi_team(&self) -> bool {
        true
    }

    fn default_rating(&self) -> Rating {
        WengLinRating::default().into()
    }

    fn recalc_ratings(&self, input: &[(Rating, u8)]) -> Vec<Rating> {
        let teams: Vec<Vec<WengLinRating>> = input.iter().map(|w| vec![w.0.into()]).collect_vec();
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
}
