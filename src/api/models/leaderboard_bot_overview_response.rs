use crate::api::models::BuildResponse;
use crate::arena::LeaderboardBotOverview;
use serde::Serialize;

#[derive(Serialize)]
pub struct LeaderboardBotOverviewResponse {
    pub id: i64,
    pub name: String,
    pub language: String,
    pub rating_mu: f64,
    pub rating_sigma: f64,
    pub matches_played: u64,
    pub matches_with_error: u64,
    pub builds: Vec<BuildResponse>,
}

impl From<LeaderboardBotOverview> for LeaderboardBotOverviewResponse {
    fn from(v: LeaderboardBotOverview) -> Self {
        LeaderboardBotOverviewResponse {
            id: v.id.into(),
            name: v.name.to_string(),
            language: v.language.to_string(),
            rating_mu: v.rating.mu,
            rating_sigma: v.rating.sigma,
            matches_played: v.matches_played,
            matches_with_error: v.matches_with_error,
            builds: v.builds.into_iter().map(|b| b.into()).collect(),
        }
    }
}
