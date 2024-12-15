use crate::domain::{BotId, MatchId};

pub struct Match {
    pub id: MatchId,
    pub seed: i64,
    pub bot_ids: Vec<BotId>,
    pub status: MatchStatus,
}

impl Match {
    pub fn new(seed: i64, bot_ids: Vec<BotId>) -> Self {
        Self {
            id: MatchId::UNINITIALIZED,
            seed,
            bot_ids,
            status: MatchStatus::Pending,
        }
    }
}

pub enum MatchStatus {
    Pending,
    Running,
    Finished(MatchResult),
    Error(String),
}

pub struct MatchResult {
    pub ranks: Vec<u8>,
    pub errors: Vec<bool>,
}
