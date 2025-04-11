use crate::arena::LeaderboardItem;
use chrono::{DateTime, Local};
use serde::Serialize;

#[derive(Serialize)]
pub struct LeaderboardItemResponse {
    pub id: i64,
    pub rank: usize,
    pub name: String,
    pub rating_mu: f64,
    pub rating_sigma: f64,
    pub wins: usize,
    pub loses: usize,
    pub draws: usize,
    pub created_at: String,
}

impl From<LeaderboardItem> for LeaderboardItemResponse {
    fn from(item: LeaderboardItem) -> Self {
        LeaderboardItemResponse {
            id: item.id.into(),
            rank: item.rank,
            name: item.name.into(),
            rating_mu: item.rating.mu,
            rating_sigma: item.rating.sigma,
            wins: item.wins,
            loses: item.loses,
            draws: item.draws,
            created_at: DateTime::<Local>::from(item.created_at)
                .format("%d/%m/%Y %H:%M")
                .to_string(),
        }
    }
}
