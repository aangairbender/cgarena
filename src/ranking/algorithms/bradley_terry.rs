use crate::domain::{BotId, Rating, WinrateStats};
use crate::ranking::{Algorithm, BatchAlgorithm};
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Config {
    max_iter: Option<usize>,
}

pub struct BradleyTerry {
    config: Config,
}

impl BradleyTerry {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl Algorithm for BradleyTerry {
    fn supports_multi_team(&self) -> bool {
        true
    }

    fn default_rating(&self) -> Rating {
        Rating::new(0f64, 0f64)
    }
}

impl BatchAlgorithm for BradleyTerry {
    fn recalc_batch(
        &self,
        winrate_stats: &HashMap<(BotId, BotId), WinrateStats>,
    ) -> HashMap<BotId, Rating> {
        bradley_terry_bayesian(winrate_stats, self.config.max_iter.unwrap_or(50))
    }
}

#[derive(Clone)]
struct Pair {
    i: usize,
    j: usize,
    wins_ij: f64,
    wins_ji: f64,
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

// ------------------------------------------------------------
// Gradient & Hessian (Bayesian Bradley–Terry)
// ------------------------------------------------------------
fn compute_grad_hess(
    n: usize,
    pairs: &[Pair],
    s: &DVector<f64>,
    tau: f64, // prior stddev in natural scale
) -> (DVector<f64>, DMatrix<f64>) {
    let mut grad = DVector::<f64>::zeros(n);
    let mut hess = DMatrix::<f64>::zeros(n, n);

    for pair in pairs {
        let i = pair.i;
        let j = pair.j;

        let w_ij = pair.wins_ij;
        let w_ji = pair.wins_ji;

        let d = s[i] - s[j];
        let p = sigmoid(d);
        let total = w_ij + w_ji;

        // Likelihood gradient
        let g = w_ij * (1.0 - p) - w_ji * p;
        grad[i] += g;
        grad[j] -= g;

        // Likelihood Hessian
        let weight = total * p * (1.0 - p);

        hess[(i, i)] -= weight;
        hess[(j, j)] -= weight;
        hess[(i, j)] += weight;
        hess[(j, i)] += weight;
    }

    // Gaussian prior: s_i ~ N(0, tau²)
    let prior_prec = 1.0 / (tau * tau);

    for i in 0..n {
        grad[i] -= s[i] * prior_prec;
        hess[(i, i)] -= prior_prec;
    }

    (grad, hess)
}

// ------------------------------------------------------------
// Fit MAP using Newton method
// ------------------------------------------------------------
fn fit_map(n: usize, pairs: &[Pair], tau: f64, max_iter: usize) -> (DVector<f64>, DMatrix<f64>) {
    let mut s = DVector::<f64>::zeros(n);

    for _ in 0..max_iter {
        let (grad, hess) = compute_grad_hess(n, pairs, &s, tau);

        let fisher = -&hess;
        let step = fisher.lu().solve(&grad).expect("Newton step failed");

        s += &step;

        if step.amax() < 1e-8 {
            break;
        }
    }

    let (_, hess) = compute_grad_hess(n, pairs, &s, tau);
    let fisher = -hess;

    let covariance = fisher.try_inverse().expect("Hessian inversion failed");

    (s, covariance)
}

// ------------------------------------------------------------
// Public API
// ------------------------------------------------------------
fn bradley_terry_bayesian(
    winrate_stats: &HashMap<(BotId, BotId), WinrateStats>,
    max_iter: usize,
) -> HashMap<BotId, Rating> {
    // 🚨 Handle empty dataset
    if winrate_stats.is_empty() {
        return HashMap::new();
    }

    let scale = 400.0 / std::f64::consts::LN_10;

    // --------------------------------------------------------
    // Build index
    // --------------------------------------------------------
    let mut bots = HashMap::<BotId, usize>::new();
    for (a, b) in winrate_stats.keys() {
        if !bots.contains_key(a) {
            let idx = bots.len();
            bots.insert(*a, idx);
        }

        if !bots.contains_key(b) {
            let idx = bots.len();
            bots.insert(*b, idx);
        }
    }

    let n = bots.len();

    // --------------------------------------------------------
    // Build pair list
    // --------------------------------------------------------
    let mut pairs = Vec::<Pair>::new();

    for ((a, b), stats) in winrate_stats {
        let i = bots[a];
        let j = bots[b];

        let wins_ij = stats.wins as f64 + 0.5 * stats.draws as f64;
        let wins_ji = stats.loses as f64 + 0.5 * stats.draws as f64;

        pairs.push(Pair {
            i,
            j,
            wins_ij,
            wins_ji,
        });
    }

    // --------------------------------------------------------
    // 1️⃣ Initial tau (weak prior)
    // --------------------------------------------------------
    let tau_elo = 400.0;
    let mut tau = tau_elo / scale;

    // --------------------------------------------------------
    // 2️⃣ First fit
    // --------------------------------------------------------
    let (mut s, mut covariance) = fit_map(n, &pairs, tau, max_iter);

    // --------------------------------------------------------
    // 3️⃣ Empirical Bayes auto-tuning of tau
    // --------------------------------------------------------
    let mean_s = s.iter().sum::<f64>() / n as f64;

    let mut var_s = 0.0;
    for i in 0..n {
        var_s += (s[i] - mean_s).powi(2);
    }
    var_s /= n as f64;

    let mut avg_post_var = 0.0;
    for i in 0..n {
        avg_post_var += covariance[(i, i)];
    }
    avg_post_var /= n as f64;

    let tau_new = (var_s + avg_post_var).sqrt();

    // Avoid collapse
    tau = tau_new.max(1e-6);

    // --------------------------------------------------------
    // 4️⃣ Refit with tuned tau
    // --------------------------------------------------------
    let (s2, covariance2) = fit_map(n, &pairs, tau, max_iter);

    s = s2;
    covariance = covariance2;

    // --------------------------------------------------------
    // 5️⃣ Convert to Elo
    // --------------------------------------------------------
    let base = 1500.0;
    let mut result = HashMap::new();

    for (bot, idx) in bots {
        let mu = base + s[idx] * scale;
        let variance = covariance[(idx, idx)].max(0.0);
        let sigma = variance.sqrt() * scale;

        result.insert(bot, Rating { mu, sigma });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_stats(w: u64, l: u64, d: u64) -> WinrateStats {
        WinrateStats {
            wins: w,
            loses: l,
            draws: d,
        }
    }

    fn run(stats_map: HashMap<(i64, i64), WinrateStats>) -> HashMap<i64, Rating> {
        let stats = stats_map
            .into_iter()
            .map(|(key, value)| ((key.0.into(), key.1.into()), value))
            .collect();
        let res = bradley_terry_bayesian(&stats, 50);
        return res
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect();
    }

    #[test]
    fn empty_dataset_returns_empty() {
        let stats = HashMap::new();
        let result = run(stats);

        assert!(result.is_empty());
    }

    #[test]
    fn single_match_two_bots() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(1, 0, 0));

        let result = run(stats);

        assert_eq!(result.len(), 2);

        let a = result.get(&1).unwrap();
        let b = result.get(&2).unwrap();

        assert!(a.mu > b.mu);
    }

    #[test]
    fn symmetric_results_equal_rating() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(10, 10, 0));

        let result = run(stats);

        let a = result.get(&1).unwrap();
        let b = result.get(&2).unwrap();

        assert!((a.mu - b.mu).abs() < 1e-6);
    }

    #[test]
    fn draw_only_matches() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(0, 0, 100));

        let result = run(stats);

        let a = result.get(&1).unwrap();
        let b = result.get(&2).unwrap();

        assert!((a.mu - b.mu).abs() < 1e-6);
    }

    #[test]
    fn one_sided_matches() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(100, 0, 0));

        let result = run(stats);

        let a = result.get(&1).unwrap();
        let b = result.get(&2).unwrap();

        assert!(a.mu > b.mu);
        assert!(a.mu - b.mu > 50.0);
    }

    #[test]
    fn transitive_strength() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(20, 0, 0));
        stats.insert((2, 3), create_stats(20, 0, 0));

        let result = run(stats);

        let r1 = result.get(&1).unwrap();
        let r2 = result.get(&2).unwrap();
        let r3 = result.get(&3).unwrap();

        assert!(r1.mu > r2.mu);
        assert!(r2.mu > r3.mu);
    }

    #[test]
    fn disconnected_groups_do_not_crash() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(10, 0, 0));
        stats.insert((3, 4), create_stats(10, 0, 0));

        let result = run(stats);

        assert_eq!(result.len(), 4);
    }

    #[test]
    fn zero_game_pair_is_ignored() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(0, 0, 0));

        let result = run(stats);

        let a = result.get(&1).unwrap();
        let b = result.get(&2).unwrap();

        assert!((a.mu - b.mu).abs() < 1e-6);
    }

    #[test]
    fn large_match_counts_do_not_overflow() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(1_000_000, 0, 0));

        let result = run(stats);

        let a = result.get(&1).unwrap();
        let b = result.get(&2).unwrap();

        assert!(a.mu > b.mu);
    }

    #[test]
    fn chain_of_bots_produces_ordered_ranking() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(50, 0, 0));
        stats.insert((2, 3), create_stats(50, 0, 0));
        stats.insert((3, 4), create_stats(50, 0, 0));
        stats.insert((4, 5), create_stats(50, 0, 0));

        let result = run(stats);

        for i in 1..5 {
            let a = result.get(&i).unwrap();
            let b = result.get(&(i + 1)).unwrap();

            assert!(a.mu > b.mu);
        }
    }

    #[test]
    fn ratings_are_finite() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(5, 5, 5));
        stats.insert((2, 3), create_stats(5, 5, 5));
        stats.insert((3, 1), create_stats(5, 5, 5));

        let result = run(stats);

        for r in result.values() {
            assert!(r.mu.is_finite());
            assert!(r.sigma.is_finite());
        }
    }

    #[test]
    fn uncertainty_is_positive() {
        let mut stats = HashMap::new();

        stats.insert((1, 2), create_stats(1, 0, 0));

        let result = run(stats);

        for r in result.values() {
            assert!(r.sigma >= 0.0);
        }
    }
}
