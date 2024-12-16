use crate::config::RankingConfig;
use crate::db::Database;
use crate::domain::{Match, Rating};
use itertools::Itertools;
use skillratings::MultiTeamOutcome;
use std::sync::Arc;
use tracing::warn;

#[derive(Clone)]
pub struct Ranking {
    config: Arc<RankingConfig>,
    db: Database,
}

impl Ranking {
    pub fn new(config: Arc<RankingConfig>, db: Database) -> Self {
        Self { config, db }
    }

    pub async fn update_rating(&self, r#match: &Match) {
        let mut participant_stats = Vec::with_capacity(r#match.participants.len());
        for p in &r#match.participants {
            let Some(stats) = self.db.fetch_bot_stats(p.bot_id).await else {
                warn!("Can't process result of match. Bot stats is missing");
                return;
            };
            participant_stats.push((p, stats));
        }

        let input = participant_stats
            .iter()
            .map(|(p, s)| (s.rating, p.rank))
            .collect_vec();

        let new_ratings = match self.config.as_ref() {
            &RankingConfig::OpenSkill => recalc_ratings_openskill(&input),
        };

        for ((p, mut s), r) in participant_stats.into_iter().zip(new_ratings) {
            s.matches_played += 1;
            if p.error {
                s.matches_with_error += 1;
            }
            s.rating = r;
            self.db.upsert_bot_stats(p.bot_id, s).await;
        }
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
