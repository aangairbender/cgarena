use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::oneshot;

use crate::domain::*;

pub enum ArenaCommand {
    CreateBot(CreateBotCommand),
    DeleteBot(DeleteBotCommand),
    RenameBot(RenameBotCommand),
    FetchStatus(FetchStatusCommand),
    CreateLeaderboard(CreateLeaderboardCommand),
    DeleteLeaderboard(DeleteLeaderboardCommand),
    PatchLeaderboard(PatchLeaderboardCommand),
    Chart(ChartCommand),
}

pub struct ChartCommand {
    pub filter: MatchFilter,
    pub attribute_name: String,
    pub response: oneshot::Sender<ChartOverview>,
}

pub struct ChartOverview {
    pub items: Vec<ChartItem>,
    pub total_matches: u64,
}

pub struct ChartItem {
    pub bot_id: BotId,
    pub data: Vec<ChartTurnData>,
}

pub struct ChartTurnData {
    pub turn: u16,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

pub struct CreateLeaderboardCommand {
    pub name: LeaderboardName,
    pub filter: MatchFilter,
    pub response: oneshot::Sender<LeaderboardOverview>,
}

pub struct PatchLeaderboardCommand {
    pub id: LeaderboardId,
    pub name: LeaderboardName,
    pub filter: MatchFilter,
    pub response: oneshot::Sender<PatchLeaderboardResult>,
}

pub enum PatchLeaderboardResult {
    OK,
    NotFound,
}

pub struct DeleteLeaderboardCommand {
    pub id: LeaderboardId,
    pub response: oneshot::Sender<()>,
}

pub struct RenameBotCommand {
    pub id: BotId,
    pub new_name: BotName,
    pub response: oneshot::Sender<RenameBotResult>,
}

pub enum RenameBotResult {
    Renamed,
    DuplicateName,
    NotFound,
}

pub struct CreateBotCommand {
    pub name: BotName,
    pub source_code: SourceCode,
    pub language: Language,
    pub response: oneshot::Sender<CreateBotResult>,
}

pub enum CreateBotResult {
    Created(BotOverview),
    DuplicateName,
}

pub struct DeleteBotCommand {
    pub id: BotId,
    pub response: oneshot::Sender<()>,
}

pub struct FetchStatusCommand {
    pub response: oneshot::Sender<FetchStatusResult>,
}

pub struct FetchStatusResult {
    pub bots: Vec<BotOverview>,
    pub leaderboards: Vec<LeaderboardOverview>,
}

pub struct BotOverview {
    pub id: BotId,
    pub name: BotName,
    pub language: Language,
    pub matches_played: u64,
    pub matches_with_error: u64,
    pub builds: Vec<Build>,
    pub created_at: DateTime<Utc>,
}

pub struct LeaderboardOverview {
    pub id: LeaderboardId,
    pub name: LeaderboardName,
    pub filter: String,
    pub status: LeaderboardStatus,
    pub items: Vec<LeaderboardItem>,
    pub winrate_stats: HashMap<(BotId, BotId), WinrateStats>,
    pub total_matches: u64,
    pub example_seeds: Vec<i64>,
}

pub enum LeaderboardStatus {
    Live,
    Computing,
    Error(String),
}

pub struct LeaderboardItem {
    pub id: BotId,
    pub rank: usize,
    pub rating: Rating,
}
