use crate::domain::{BotId, MatchAttribute, MatchId};
use std::path::PathBuf;

// only successfully finished matches would be stored in DB
pub struct Match {
    pub id: MatchId,
    pub seed: i64,
    pub participants: Vec<Participant>,
    pub attributes: Vec<MatchAttribute>,
    pub replay_path: Option<PathBuf>,
}

#[derive(Clone)]
pub struct Participant {
    pub bot_id: BotId,
    pub rank: u8,
    pub error: bool,
}

impl Match {
    pub fn new(
        seed: i64,
        participants: Vec<Participant>,
        attributes: Vec<MatchAttribute>,
        replay_path: Option<PathBuf>,
    ) -> Match {
        Self {
            id: MatchId::UNINITIALIZED,
            seed,
            participants,
            attributes,
            replay_path,
        }
    }
}
