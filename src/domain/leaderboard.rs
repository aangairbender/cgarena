use crate::domain::{LeaderboardId, LeaderboardName, MatchFilter};

pub struct Leaderboard {
    pub id: LeaderboardId,
    pub name: LeaderboardName,
    pub filter: MatchFilter,
}

impl Leaderboard {
    pub fn new(name: LeaderboardName, filter: MatchFilter) -> Leaderboard {
        Leaderboard {
            id: LeaderboardId::UNINITIALIZED,
            name,
            filter,
        }
    }

    pub fn global() -> Leaderboard {
        Leaderboard {
            id: LeaderboardId::UNINITIALIZED,
            name: LeaderboardName::global(),
            filter: MatchFilter::accept_all(),
        }
    }
}
