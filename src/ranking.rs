use crate::config::RankingConfig;
use crate::domain::{BotId, Match, Rating};
use itertools::Itertools;
use skillratings::MultiTeamOutcome;
use std::collections::HashMap;

pub fn recalc_rating<'a>(
    config: &RankingConfig,
    ratings: &mut HashMap<BotId, Rating>,
    matches: impl Iterator<Item = &'a Match>,
) {
    for m in matches {
        let ps = m
            .participants
            .iter()
            .map(|p| (ratings.get(&p.bot_id).copied().unwrap_or_default(), p.rank))
            .collect_vec();

        let new_ratings = recalc_ratings(&ps, &config);

        m.participants
            .iter()
            .zip_eq(new_ratings)
            .for_each(|(p, new_rating)| {
                ratings.insert(p.bot_id, new_rating);
            });
    }
}

fn recalc_ratings(input: &[(Rating, u8)], config: &RankingConfig) -> Vec<Rating> {
    match config {
        RankingConfig::OpenSkill => recalc_ratings_openskill(input),
    }
}

fn recalc_ratings_openskill(input: &[(Rating, u8)]) -> Vec<Rating> {
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

    let new_ratings = weng_lin_multi_team(&teams_and_ranks, &WengLinConfig::default());

    new_ratings.into_iter().map(|r| r[0].into()).collect_vec()
}
