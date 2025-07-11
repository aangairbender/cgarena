#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct LeaderboardId(i64);

impl LeaderboardId {
    pub const UNINITIALIZED: LeaderboardId = LeaderboardId(0);
}

impl From<i64> for LeaderboardId {
    fn from(id: i64) -> Self {
        assert_ne!(id, Self::UNINITIALIZED.0);
        Self(id)
    }
}

impl From<LeaderboardId> for i64 {
    fn from(id: LeaderboardId) -> i64 {
        id.0
    }
}
