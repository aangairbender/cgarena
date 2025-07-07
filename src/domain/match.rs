use crate::domain::{BotId, MatchAttribute, MatchId};

// only successfully finished matches would be stored in DB
pub struct Match {
    pub id: MatchId,
    pub seed: i64,
    pub participants: Vec<Participant>,
    pub attributes: Vec<MatchAttribute>,
}

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
    ) -> Match {
        Self {
            id: MatchId::UNINITIALIZED,
            seed,
            participants,
            attributes,
        }
    }
}
