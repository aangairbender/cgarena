use crate::domain::Rating;
use crate::ranking::Algorithm;
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
    fn support_multi_team(&self) -> bool {
        false
    }

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
}
