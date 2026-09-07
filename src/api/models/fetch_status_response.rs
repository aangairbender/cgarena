use crate::api::models::BuildResponse;
use crate::arena_commands::{
    BotOverview, CandidateEvaluationOverview, EvaluationStageOverview, FetchStatusResult,
    LeaderboardItem, LeaderboardOverview, LeaderboardStatus,
};
use crate::domain::{BotId, BotRole, WinrateStats};
use chrono::DateTime;
use chrono::Local;
use serde::Serialize;

#[derive(Serialize)]
pub struct FetchStatusResponse {
    pub bots: Vec<BotOverviewResponse>,
    pub leaderboards: Vec<LeaderboardOverviewResponse>,
    pub evaluation_scheduling_enabled: bool,
}

impl From<FetchStatusResult> for FetchStatusResponse {
    fn from(value: FetchStatusResult) -> Self {
        FetchStatusResponse {
            bots: value.bots.into_iter().map(Into::into).collect(),
            leaderboards: value.leaderboards.into_iter().map(Into::into).collect(),
            evaluation_scheduling_enabled: value.evaluation_scheduling_enabled,
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
    pub example_seeds: Vec<String>,
}

impl From<LeaderboardOverview> for LeaderboardOverviewResponse {
    fn from(value: LeaderboardOverview) -> Self {
        LeaderboardOverviewResponse {
            id: value.id.into(),
            name: value.name.into(),
            filter: value.filter,
            status: match value.status {
                LeaderboardStatus::Live => "live",
                LeaderboardStatus::Computing => "computing",
                LeaderboardStatus::Error(_) => "error",
            },
            error: match value.status {
                LeaderboardStatus::Error(e) => Some(e),
                _ => None,
            },
            items: value.items.into_iter().map(Into::into).collect(),
            winrate_stats: value.winrate_stats.into_iter().map(Into::into).collect(),
            total_matches: value.total_matches,
            example_seeds: value
                .example_seeds
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
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
    pub rating: f64,
    pub rating_mu: f64,
    pub rating_sigma: f64,
}

impl From<LeaderboardItem> for LeaderboardItemResponse {
    fn from(item: LeaderboardItem) -> Self {
        LeaderboardItemResponse {
            id: item.id.into(),
            rank: item.rank,
            rating: item.rating_ordinal,
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
    pub role: BotRole,
    pub evaluation_plan_revision_id: Option<i64>,
    pub evaluation: Option<CandidateEvaluationResponse>,
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
            role: v.role,
            evaluation_plan_revision_id: v.evaluation_plan_revision_id,
            evaluation: v.evaluation.map(Into::into),
            matches_played: v.matches_played,
            matches_with_error: v.matches_with_error,
            builds: v.builds.into_iter().map(|b| b.into()).collect(),
            created_at: DateTime::<Local>::from(v.created_at)
                .format("%d/%m/%Y %H:%M")
                .to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct CandidateEvaluationResponse {
    pub complete: bool,
    pub stages: Vec<EvaluationStageResponse>,
}

impl From<CandidateEvaluationOverview> for CandidateEvaluationResponse {
    fn from(value: CandidateEvaluationOverview) -> Self {
        Self {
            complete: value.complete,
            stages: value.stages.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct EvaluationStageResponse {
    pub id: i64,
    pub name: String,
    pub coverage: &'static str,
    pub target: u64,
    pub matches: u64,
    pub candidate_errors: u64,
    pub complete: bool,
    pub benchmark_encounters: std::collections::HashMap<i64, u64>,
}

impl From<EvaluationStageOverview> for EvaluationStageResponse {
    fn from(value: EvaluationStageOverview) -> Self {
        Self {
            id: value.id,
            name: value.name,
            coverage: value.coverage,
            target: value.target,
            matches: value.matches,
            candidate_errors: value.candidate_errors,
            complete: value.complete,
            benchmark_encounters: value
                .benchmark_encounters
                .into_iter()
                .map(|(id, count)| (id.into(), count))
                .collect(),
        }
    }
}
