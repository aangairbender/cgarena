#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct BotId(i64);

impl BotId {
    pub const UNINITIALIZED: BotId = BotId(0);
}

impl From<i64> for BotId {
    fn from(id: i64) -> Self {
        assert_ne!(id, Self::UNINITIALIZED.0);
        Self(id)
    }
}

impl From<BotId> for i64 {
    fn from(id: BotId) -> i64 {
        id.0
    }
}
