use crate::api::models::BuildResponse;
use crate::arena::BotOverview;
use crate::arena::FetchStatusResult;
use crate::arena::LeaderboardItem;
use crate::arena::LeaderboardOverview;
use crate::domain::BotId;
use crate::domain::WinrateStats;
use chrono::DateTime;
use chrono::Local;
use serde::Serialize;

#[derive(Serialize)]
pub struct FetchStatusResponse {
    pub bots: Vec<BotOverviewResponse>,
    pub leaderboards: Vec<LeaderboardOverviewResponse>,
}

impl From<FetchStatusResult> for FetchStatusResponse {
    fn from(value: FetchStatusResult) -> Self {
        FetchStatusResponse {
            bots: value.bots.into_iter().map(Into::into).collect(),
            leaderboards: value.leaderboards.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct LeaderboardOverviewResponse {
    pub id: i64,
    pub name: String,
    pub filter: String,
    pub status: &'static str,
    pub error: Option<String>,
    pub items: Vec<LeaderboardItemResponse>,
    pub winrate_stats: Vec<WinrateStatsResponse>,
    pub total_matches: u64,
}

impl From<LeaderboardOverview> for LeaderboardOverviewResponse {
    fn from(value: LeaderboardOverview) -> Self {
        LeaderboardOverviewResponse {
            id: value.id.into(),
            name: value.name.into(),
            filter: value.filter,
            status: match value.status {
                crate::arena::LeaderboardStatus::Live => "live",
                crate::arena::LeaderboardStatus::Computing => "computing",
                crate::arena::LeaderboardStatus::Error(_) => "error",
            },
            error: match value.status {
                crate::arena::LeaderboardStatus::Error(e) => Some(e),
                _ => None,
            },
            items: value.items.into_iter().map(Into::into).collect(),
            winrate_stats: value.winrate_stats.into_iter().map(Into::into).collect(),
            total_matches: value.total_matches,
        }
    }
}

#[derive(Serialize)]
pub struct WinrateStatsResponse {
    pub bot_id: i64,
    pub opponent_bot_id: i64,
    pub wins: u64,
    pub draws: u64,
    pub loses: u64,
}

impl From<((BotId, BotId), WinrateStats)> for WinrateStatsResponse {
    fn from(((a, b), value): ((BotId, BotId), WinrateStats)) -> Self {
        WinrateStatsResponse {
            bot_id: a.into(),
            opponent_bot_id: b.into(),
            wins: value.wins,
            draws: value.draws,
            loses: value.loses,
        }
    }
}

#[derive(Serialize)]
pub struct LeaderboardItemResponse {
    pub id: i64,
    pub rank: usize,
    pub rating_mu: f64,
    pub rating_sigma: f64,
}

impl From<LeaderboardItem> for LeaderboardItemResponse {
    fn from(item: LeaderboardItem) -> Self {
        LeaderboardItemResponse {
            id: item.id.into(),
            rank: item.rank,
            rating_mu: item.rating.mu,
            rating_sigma: item.rating.sigma,
        }
    }
}

#[derive(Serialize)]
pub struct BotOverviewResponse {
    pub id: i64,
    pub name: String,
    pub language: String,
    pub matches_played: u64,
    pub matches_with_error: u64,
    pub builds: Vec<BuildResponse>,
    pub created_at: String,
}

impl From<BotOverview> for BotOverviewResponse {
    fn from(v: BotOverview) -> Self {
        BotOverviewResponse {
            id: v.id.into(),
            name: v.name.to_string(),
            language: v.language.to_string(),
            matches_played: v.matches_played,
            matches_with_error: v.matches_with_error,
            builds: v.builds.into_iter().map(|b| b.into()).collect(),
            created_at: DateTime::<Local>::from(v.created_at)
                .format("%d/%m/%Y %H:%M")
                .to_string(),
        }
    }
}
