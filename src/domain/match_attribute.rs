use crate::domain::BotId;

pub struct MatchAttribute {
    pub name: String,
    pub bot_id: Option<BotId>,
    pub turn: Option<u16>,
    pub value: String,
}
