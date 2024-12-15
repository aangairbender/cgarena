#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct MatchId(i64);

impl MatchId {
    pub const UNINITIALIZED: MatchId = MatchId(0);
}

impl From<i64> for MatchId {
    fn from(id: i64) -> Self {
        assert_ne!(id, Self::UNINITIALIZED.0);
        Self(id)
    }
}

impl From<MatchId> for i64 {
    fn from(id: MatchId) -> i64 {
        id.0
    }
}
