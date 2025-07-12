use crate::{
    domain::{ComputedStats, LeaderboardId, LeaderboardName, Match, MatchFilter},
    ranking::Ranker,
};

pub struct Leaderboard {
    pub id: LeaderboardId,
    pub name: LeaderboardName,
    pub filter: MatchFilter,
    pub stats: ComputedStats,
}

impl Leaderboard {
    pub fn new(name: LeaderboardName, filter: MatchFilter) -> Leaderboard {
        Leaderboard {
            id: LeaderboardId::UNINITIALIZED,
            name,
            filter,
            stats: Default::default(),
        }
    }

    pub fn global() -> Leaderboard {
        Leaderboard {
            id: LeaderboardId::UNINITIALIZED,
            name: LeaderboardName::global(),
            filter: MatchFilter::accept_all(),
            stats: Default::default(),
        }
    }

    pub fn process(&mut self, ranker: &Ranker, m: &Match) {
        if self.filter.matches(m) {
            self.stats.recalc_after_match(ranker, m);
        }
    }

    pub fn reset(&mut self) {
        self.stats.clear();
    }
}
