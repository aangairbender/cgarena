use std::sync::{Arc, Mutex};

use sqlx::SqlitePool;
use tokio_util::sync::CancellationToken;

use crate::{
    db::Database,
    domain::{ComputedStats, Leaderboard, Match},
    ranking::Ranker,
};

pub struct AsyncLeaderboard {
    pub leaderboard: Leaderboard,
    ranker: Arc<Ranker>,
    pool: SqlitePool,
    status: Arc<Mutex<LeaderboardStatus>>,
    live_matches: Vec<Arc<Match>>,
}

impl Drop for AsyncLeaderboard {
    fn drop(&mut self) {
        let status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Live(_) => {}
            LeaderboardStatus::Computing(ref token) => token.cancel(),
        }
    }
}

impl AsyncLeaderboard {
    pub fn new(leaderboard: Leaderboard, ranker: Arc<Ranker>, pool: SqlitePool) -> Self {
        Self {
            leaderboard,
            ranker,
            pool,
            status: Arc::new(Mutex::new(
                LeaderboardStatus::Live(ComputedStats::default()),
            )),
            live_matches: vec![],
        }
    }

    pub fn recalculate(&self) {
        let mut status = self.status.lock().unwrap();
        if let LeaderboardStatus::Computing(ref token) = *status {
            token.cancel();
        }

        let token = CancellationToken::new();
        *status = LeaderboardStatus::Computing(token.clone());
        drop(status);

        let status_inner = Arc::clone(&self.status);
        let ranker = Arc::clone(&self.ranker);
        let filter = self.leaderboard.filter.clone();
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let mut stats = ComputedStats::default();
            let attrs = filter.needed_attributes();
            let matches = Database::fetch_matches_with_attrs(&pool, &attrs).await;
            for m in &matches {
                if filter.matches(m) {
                    stats.recalc_after_match(&ranker, m);
                }
            }
            if !token.is_cancelled() {
                let mut status = status_inner.lock().unwrap();
                *status = LeaderboardStatus::Live(stats);
            }
        });
    }

    pub fn stats(&self) -> Option<ComputedStats> {
        let status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Live(ref computed_stats) => Some(computed_stats.clone()),
            LeaderboardStatus::Computing(_) => None,
        }
    }

    pub fn record_for_later(&mut self, m: Arc<Match>) {
        self.live_matches.push(m);
    }

    pub fn catch_up_with_live_matches(&mut self) {
        let mut status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Live(ref mut computed_stats) => {
                for m in self.live_matches.drain(..) {
                    if self.leaderboard.filter.matches(&m) {
                        computed_stats.recalc_after_match(&self.ranker, &m);
                    }
                }
            }
            LeaderboardStatus::Computing(_) => {}
        }
    }
}

pub enum LeaderboardStatus {
    Live(ComputedStats),
    Computing(CancellationToken),
}
