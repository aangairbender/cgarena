use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio_util::sync::CancellationToken;

use crate::{
    domain::{ComputedStats, Leaderboard, Match},
    match_retrieval::MatchRetrieval,
    ranking::Ranker,
};

pub struct AsyncLeaderboard {
    pub leaderboard: Leaderboard,
    ranker: Arc<Ranker>,
    match_retrieval: MatchRetrieval,
    status: Arc<Mutex<LeaderboardStatus>>,
    live_matches: Vec<Arc<Match>>,
}

impl Drop for AsyncLeaderboard {
    fn drop(&mut self) {
        let status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Live(_) => {}
            LeaderboardStatus::Computing(ref token) => token.cancel(),
            LeaderboardStatus::Error(_, _) => {}
        }
    }
}

impl AsyncLeaderboard {
    pub fn new(
        leaderboard: Leaderboard,
        ranker: Arc<Ranker>,
        match_retrieval: MatchRetrieval,
    ) -> Self {
        Self {
            leaderboard,
            ranker,
            match_retrieval,
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
        let match_retrieval = self.match_retrieval.clone();
        tokio::spawn(async move {
            let matches = match_retrieval.leaderboard_matches(&filter).await;

            match matches {
                Ok(matches) => {
                    let matches = matches.iter().collect::<Vec<_>>();
                    let mut stats = ComputedStats::default();
                    stats.recalc_after_matches(&ranker, &matches);
                    if !token.is_cancelled() {
                        let mut status = status_inner.lock().unwrap();
                        *status = LeaderboardStatus::Live(stats);
                    }
                }
                Err(e) => {
                    if !token.is_cancelled() {
                        let mut status = status_inner.lock().unwrap();
                        *status = LeaderboardStatus::Error(e, Instant::now());
                    }
                }
            }
        });
    }

    pub fn stats(&self) -> Option<ComputedStats> {
        let status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Live(ref computed_stats) => Some(computed_stats.clone()),
            LeaderboardStatus::Computing(_) => None,
            LeaderboardStatus::Error(_, _) => None,
        }
    }

    pub fn error(&self) -> Option<String> {
        let status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Error(ref e, _) => Some(e.to_string()),
            _ => None,
        }
    }

    pub fn record_for_later(&mut self, m: Arc<Match>) {
        self.live_matches.push(m);
    }

    pub fn catch_up_with_live_matches(&mut self) {
        let mut status = self.status.lock().unwrap();
        match *status {
            LeaderboardStatus::Live(ref mut computed_stats) => {
                let live_matches = std::mem::take(&mut self.live_matches);
                let filtered = live_matches
                    .iter()
                    .map(|m| m.as_ref())
                    .filter(|m| self.leaderboard.filter.matches(m))
                    .collect::<Vec<_>>();
                computed_stats.recalc_after_matches(&self.ranker, &filtered);
            }
            LeaderboardStatus::Computing(_) => {}
            LeaderboardStatus::Error(_, at) => {
                if Instant::now() > at + Duration::from_secs(3) {
                    self.recalculate();
                }
            }
        }
    }
}

pub enum LeaderboardStatus {
    Live(ComputedStats),
    Computing(CancellationToken),
    Error(anyhow::Error, Instant),
}
