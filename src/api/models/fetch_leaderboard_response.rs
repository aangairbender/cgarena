use crate::arena::FetchLeaderboardResult;

use crate::api::models::LeaderboardBotOverviewResponse;
use crate::api::models::LeaderboardItemResponse;
use serde::Serialize;

#[derive(Serialize)]
pub struct FetchLeaderboardResponse {
    pub bot_overview: LeaderboardBotOverviewResponse,
    pub items: Vec<LeaderboardItemResponse>,
}

impl From<FetchLeaderboardResult> for FetchLeaderboardResponse {
    fn from(value: FetchLeaderboardResult) -> Self {
        FetchLeaderboardResponse {
            bot_overview: value.bot_overview.into(),
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}
