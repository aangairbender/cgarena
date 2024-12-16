use crate::domain::{BotId, MatchId};

// only successfully finished matches would be stored in DB
pub struct Match {
    pub id: MatchId,
    pub seed: i64,
    pub participants: Vec<Participant>,
}

pub struct Participant {
    pub bot_id: BotId,
    pub rank: u8,
    pub error: bool,
}

impl Match {
    pub fn new(seed: i64, participants: Vec<Participant>) -> Match {
        Self {
            id: MatchId::UNINITIALIZED,
            seed,
            participants,
        }
    }
}
