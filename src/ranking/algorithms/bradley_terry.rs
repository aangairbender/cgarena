use crate::domain::{BotId, Rating, WinrateStats};
use crate::ranking::{Algorithm, BatchAlgorithm};
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize)]
pub struct Config {
    lambda: Option<f64>,
    max_iter: Option<usize>,
    tol: Option<f64>,
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
        false
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
        compute_bradley_terry_ratings(
            winrate_stats,
            self.config.lambda.unwrap_or(1e-4),
            self.config.max_iter.unwrap_or(50),
            self.config.tol.unwrap_or(1e-8),
        )
    }
}

fn compute_bradley_terry_ratings(
    winrate_stats: &HashMap<(BotId, BotId), WinrateStats>,
    lambda: f64,
    max_iter: usize,
    tol: f64,
) -> HashMap<BotId, Rating> {
    // --------------------------------------------------
    // 1. Collect unique bots
    // --------------------------------------------------
    let mut bot_set = HashSet::new();
    for ((a, b), _) in winrate_stats.iter() {
        bot_set.insert(*a);
        bot_set.insert(*b);
    }

    let mut bots: Vec<_> = bot_set.into_iter().collect();
    bots.sort_by_key(|b| i64::from(*b)); // deterministic ordering

    let n = bots.len();

    let mut index = HashMap::new();
    for (i, bot) in bots.iter().enumerate() {
        index.insert(*bot, i);
    }

    // --------------------------------------------------
    // 2. Convert to internal pair representation
    // --------------------------------------------------

    let mut pairs = Vec::new();

    for ((a, b), stats) in winrate_stats.iter() {
        let i = index[a];
        let j = index[b];

        if i == j {
            continue;
        }

        // Treat draw as 0.5 win for each
        let wins_ij = stats.wins as f64 + 0.5 * stats.draws as f64;
        let wins_ji = stats.loses as f64 + 0.5 * stats.draws as f64;

        pairs.push(Pair {
            i,
            j,
            wins_ij,
            wins_ji,
        });
    }

    // --------------------------------------------------
    // 3. Newton optimization
    // --------------------------------------------------
    let mut s = DVector::<f64>::zeros(n);

    for _ in 0..max_iter {
        let (grad, hess) = compute_grad_hess(n, &pairs, &s, lambda);

        let delta = match hess.clone().lu().solve(&grad) {
            Some(d) => d,
            None => break,
        };

        s -= &delta;

        // enforce zero mean
        let mean = s.iter().sum::<f64>() / n as f64;
        for v in s.iter_mut() {
            *v -= mean;
        }

        if delta.norm() < tol {
            break;
        }
    }

    // --------------------------------------------------
    // 4. Final Hessian for uncertainty
    // --------------------------------------------------
    let (_, hess) = compute_grad_hess(n, &pairs, &s, lambda);

    // Covariance ≈ (-H)^(-1)
    let fisher = -hess;

    let covariance = fisher.try_inverse().expect("Hessian inversion failed");

    // --------------------------------------------------
    // 5. Convert to Elo scale
    // --------------------------------------------------
    let scale = 400.0 / std::f64::consts::LN_10;

    let mut result = HashMap::new();

    for (bot, idx) in index.iter() {
        let mu = s[*idx] * scale;

        let variance = covariance[(*idx, *idx)];
        let sigma = variance.sqrt() * scale;

        result.insert(*bot, Rating { mu, sigma });
    }

    result
}

fn compute_grad_hess(
    n: usize,
    pairs: &[Pair],
    s: &DVector<f64>,
    lambda: f64,
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

        // gradient
        let g = w_ij * (1.0 - p) - w_ji * p;
        grad[i] += g;
        grad[j] -= g;

        // hessian
        let weight = total * p * (1.0 - p);

        hess[(i, i)] -= weight;
        hess[(j, j)] -= weight;
        hess[(i, j)] += weight;
        hess[(j, i)] += weight;
    }

    // regularization
    for i in 0..n {
        grad[i] -= 2.0 * lambda * s[i];
        hess[(i, i)] -= 2.0 * lambda;
    }

    (grad, hess)
}

struct Pair {
    i: usize,
    j: usize,
    wins_ij: f64,
    wins_ji: f64,
}

fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        let z = (-x).exp();
        1.0 / (1.0 + z)
    } else {
        let z = x.exp();
        z / (1.0 + z)
    }
}
