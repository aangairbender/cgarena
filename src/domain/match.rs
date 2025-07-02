use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::{BotId, MatchId};

// only successfully finished matches would be stored in DB
pub struct Match {
    pub id: MatchId,
    pub seed: i64,
    pub participants: Vec<Participant>,
    pub attributes: MatchAttributes,
}

pub struct Participant {
    pub bot_id: BotId,
    pub rank: u8,
    pub error: bool,
}

impl Match {
    pub fn new(seed: i64, participants: Vec<Participant>, attributes: MatchAttributes) -> Match {
        Self {
            id: MatchId::UNINITIALIZED,
            seed,
            participants,
            attributes,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct MatchAttributes {
    pub common: TargetMatchAttributes,
    pub participants: Vec<TargetMatchAttributes>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct TargetMatchAttributes {
    pub global: HashMap<String, MatchAttributeValue>,
    pub turns: Vec<HashMap<String, MatchAttributeValue>>,
}

#[derive(Serialize, Deserialize)]
pub struct MatchAttributeValue {
    pub kind: MatchAttributeValueKind,
    pub str_value: String,
    pub f64_value: f64,
}

#[derive(Serialize, Deserialize)]
pub enum MatchAttributeValueKind {
    Number,
    String,
}