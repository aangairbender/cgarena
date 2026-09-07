use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Context};
use rand::random;
use serde::{Deserialize, Serialize};

use crate::config::GameConfig;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvaluationConfig {
    pub enabled_on_start: Option<bool>,
    #[serde(default = "generate_seed_sequence_key")]
    pub seed_sequence_key: u32,
    pub stages: Vec<EvaluationStageConfig>,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            enabled_on_start: Some(true),
            seed_sequence_key: generate_seed_sequence_key(),
            stages: vec![EvaluationStageConfig {
                name: "Generated sequence".to_string(),
                seed_source: SeedSourceConfig::Generated,
                coverage: CoveragePolicyConfig::PerBenchmark { target: 100 },
                min_players: None,
                max_players: None,
            }],
        }
    }
}

impl EvaluationConfig {
    pub fn validate(&self, game: &GameConfig) -> anyhow::Result<()> {
        if self.stages.is_empty() {
            bail!("evaluation.stages must contain at least one stage");
        }
        for (index, stage) in self.stages.iter().enumerate() {
            stage
                .validate(game)
                .with_context(|| format!("evaluation stage {} is invalid", index + 1))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvaluationStageConfig {
    pub name: String,
    pub seed_source: SeedSourceConfig,
    pub coverage: CoveragePolicyConfig,
    pub min_players: Option<u32>,
    pub max_players: Option<u32>,
}

impl EvaluationStageConfig {
    fn validate(&self, game: &GameConfig) -> anyhow::Result<()> {
        if self.name.trim().is_empty() {
            bail!("name must not be blank");
        }
        let min_players = self.min_players.unwrap_or(game.min_players);
        let max_players = self.max_players.unwrap_or(game.max_players);
        if min_players < game.min_players || max_players > game.max_players {
            bail!("player-count range must stay within the game range");
        }
        if min_players > max_players {
            bail!("max_players must not be less than min_players");
        }
        if matches!(&self.seed_source, SeedSourceConfig::Curated { seeds } if seeds.is_empty()) {
            bail!("curated seed source must contain at least one seed");
        }
        let target = self.coverage.target();
        if target == 0 {
            bail!("coverage target must be at least 1");
        }
        if let CoveragePolicyConfig::WeightedTotal { weights, .. } = &self.coverage {
            if weights
                .values()
                .any(|weight| !weight.is_finite() || *weight <= 0.0)
            {
                bail!("weighted coverage overrides must be positive finite numbers");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SeedSourceConfig {
    #[serde(alias = "generated_static")]
    Generated,
    Curated {
        seeds: Vec<i64>,
    },
    FreshRandom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoveragePolicyConfig {
    PerBenchmark {
        target: u64,
    },
    Total {
        target: u64,
    },
    WeightedTotal {
        target: u64,
        #[serde(default)]
        weights: BTreeMap<i64, f64>,
    },
}

impl CoveragePolicyConfig {
    pub fn target(&self) -> u64 {
        match self {
            Self::PerBenchmark { target }
            | Self::Total { target }
            | Self::WeightedTotal { target, .. } => *target,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EvaluationSeedSequence {
    LegacySuite(Vec<i64>),
    Deterministic { key: u32 },
}

impl EvaluationSeedSequence {
    fn seed_at(&self, sequence: u64) -> i64 {
        match self {
            Self::LegacySuite(seeds) => seeds[(sequence % seeds.len() as u64) as usize],
            Self::Deterministic { key } => deterministic_match_seed(*key, sequence),
        }
    }
}

pub fn generate_seed_sequence_key() -> u32 {
    random()
}

fn deterministic_match_seed(key: u32, sequence: u64) -> i64 {
    let mut value =
        u64::from(key).wrapping_add(sequence.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) as i64
}

#[derive(Clone, Debug)]
pub struct EvaluationPlanRevision {
    pub id: i64,
    pub seed_sequence: EvaluationSeedSequence,
    pub stages: Vec<EvaluationStageRevision>,
}

#[derive(Clone, Debug)]
pub struct EvaluationStageRevision {
    pub id: i64,
    pub config: EvaluationStageConfig,
}

#[derive(Default)]
pub struct ScheduledEvaluationCounts {
    pub matches_by_stage_candidate: HashMap<(i64, crate::domain::BotId), u64>,
    pub encounters: HashMap<(i64, crate::domain::BotId, crate::domain::BotId), u64>,
    pub matches_by_player_count: HashMap<(i64, crate::domain::BotId, u32), u64>,
}

pub struct EvaluationMatch {
    pub candidate_id: crate::domain::BotId,
    pub stage_revision_id: i64,
    pub bot_ids: Vec<crate::domain::BotId>,
    pub seed: i64,
}

pub fn schedule_candidate(
    game: &GameConfig,
    plan: &EvaluationPlanRevision,
    candidate_id: crate::domain::BotId,
    benchmarks: &[crate::domain::BotId],
    progress: &HashMap<i64, crate::db::StageProgress>,
    queued: &ScheduledEvaluationCounts,
) -> Vec<EvaluationMatch> {
    for stage in &plan.stages {
        let empty_progress = crate::db::StageProgress::default();
        let progress = progress.get(&stage.id).unwrap_or(&empty_progress);
        let queued_matches = queued
            .matches_by_stage_candidate
            .get(&(stage.id, candidate_id))
            .copied()
            .unwrap_or_default();
        let encounter_count = |benchmark_id| {
            progress
                .encounters
                .get(&benchmark_id)
                .copied()
                .unwrap_or_default()
                + queued
                    .encounters
                    .get(&(stage.id, candidate_id, benchmark_id))
                    .copied()
                    .unwrap_or_default()
        };
        let complete = match &stage.config.coverage {
            CoveragePolicyConfig::PerBenchmark { target } => {
                !benchmarks.is_empty()
                    && benchmarks
                        .iter()
                        .all(|benchmark_id| encounter_count(*benchmark_id) >= *target)
            }
            CoveragePolicyConfig::Total { target }
            | CoveragePolicyConfig::WeightedTotal { target, .. } => {
                progress.matches + queued_matches >= *target
            }
        };
        if complete {
            continue;
        }

        let min_players = stage.config.min_players.unwrap_or(game.min_players);
        let max_players = stage.config.max_players.unwrap_or(game.max_players);
        let Some(player_count) = (min_players..=max_players)
            .filter(|count| benchmarks.len() + 1 >= *count as usize)
            .min_by_key(|count| {
                progress
                    .matches_by_player_count
                    .get(count)
                    .copied()
                    .unwrap_or_default()
                    + queued
                        .matches_by_player_count
                        .get(&(stage.id, candidate_id, *count))
                        .copied()
                        .unwrap_or_default()
            })
        else {
            return Vec::new();
        };

        let mut opponents = benchmarks.to_vec();
        opponents.sort_by(|left, right| match &stage.config.coverage {
            CoveragePolicyConfig::WeightedTotal { weights, .. } => {
                let left_weight = weights.get(&i64::from(*left)).copied().unwrap_or(1.0);
                let right_weight = weights.get(&i64::from(*right)).copied().unwrap_or(1.0);
                ((encounter_count(*left) + 1) as f64 / left_weight)
                    .total_cmp(&((encounter_count(*right) + 1) as f64 / right_weight))
                    .then_with(|| i64::from(*left).cmp(&i64::from(*right)))
            }
            _ => encounter_count(*left)
                .cmp(&encounter_count(*right))
                .then_with(|| i64::from(*left).cmp(&i64::from(*right))),
        });
        opponents.truncate(player_count as usize - 1);

        let sequence = progress.matches + queued_matches;
        let seed = match &stage.config.seed_source {
            SeedSourceConfig::Generated => plan.seed_sequence.seed_at(sequence),
            SeedSourceConfig::Curated { seeds } => seeds[sequence as usize % seeds.len()],
            SeedSourceConfig::FreshRandom => random(),
        };
        let mut bot_ids = Vec::with_capacity(player_count as usize);
        bot_ids.push(candidate_id);
        bot_ids.extend(opponents);
        if game.symmetric {
            return vec![EvaluationMatch {
                candidate_id,
                stage_revision_id: stage.id,
                bot_ids,
                seed,
            }];
        }
        let count = bot_ids.len();
        return bot_ids
            .into_iter()
            .permutations(count)
            .map(|bot_ids| EvaluationMatch {
                candidate_id,
                stage_revision_id: stage.id,
                bot_ids,
                seed,
            })
            .collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::StageProgress, domain::BotId};

    fn stage(id: i64, coverage: CoveragePolicyConfig) -> EvaluationStageRevision {
        EvaluationStageRevision {
            id,
            config: EvaluationStageConfig {
                name: format!("Stage {id}"),
                seed_source: SeedSourceConfig::Generated,
                coverage,
                min_players: None,
                max_players: None,
            },
        }
    }

    fn game(symmetric: bool, max_players: u32) -> GameConfig {
        GameConfig {
            min_players: 2,
            max_players,
            symmetric,
        }
    }

    #[test]
    fn per_benchmark_coverage_schedules_the_largest_deficit() {
        let candidate = BotId::from(1);
        let benchmarks = [BotId::from(2), BotId::from(3)];
        let plan = EvaluationPlanRevision {
            id: 1,
            seed_sequence: EvaluationSeedSequence::LegacySuite(vec![11, 22]),
            stages: vec![stage(10, CoveragePolicyConfig::PerBenchmark { target: 2 })],
        };
        let progress = HashMap::from([(
            10,
            StageProgress {
                encounters: HashMap::from([(benchmarks[0], 1)]),
                ..Default::default()
            },
        )]);

        let scheduled = schedule_candidate(
            &game(true, 2),
            &plan,
            candidate,
            &benchmarks,
            &progress,
            &Default::default(),
        );

        assert_eq!(scheduled.len(), 1);
        assert_eq!(scheduled[0].bot_ids, vec![candidate, benchmarks[1]]);
        assert_eq!(scheduled[0].stage_revision_id, 10);
    }

    #[test]
    fn completed_stage_advances_and_balances_player_counts() {
        let candidate = BotId::from(1);
        let benchmarks = [BotId::from(2), BotId::from(3)];
        let plan = EvaluationPlanRevision {
            id: 1,
            seed_sequence: EvaluationSeedSequence::LegacySuite(vec![11, 22, 33]),
            stages: vec![
                stage(10, CoveragePolicyConfig::Total { target: 1 }),
                stage(20, CoveragePolicyConfig::Total { target: 10 }),
            ],
        };
        let progress = HashMap::from([
            (
                10,
                StageProgress {
                    matches: 1,
                    ..Default::default()
                },
            ),
            (
                20,
                StageProgress {
                    matches_by_player_count: HashMap::from([(2, 5), (3, 1)]),
                    ..Default::default()
                },
            ),
        ]);

        let scheduled = schedule_candidate(
            &game(true, 3),
            &plan,
            candidate,
            &benchmarks,
            &progress,
            &Default::default(),
        );

        assert_eq!(scheduled[0].stage_revision_id, 20);
        assert_eq!(scheduled[0].bot_ids.len(), 3);
        assert_eq!(
            scheduled[0]
                .bot_ids
                .iter()
                .filter(|id| **id == candidate)
                .count(),
            1
        );
    }

    #[test]
    fn asymmetric_games_schedule_every_position_permutation() {
        let candidate = BotId::from(1);
        let benchmarks = [BotId::from(2), BotId::from(3)];
        let plan = EvaluationPlanRevision {
            id: 1,
            seed_sequence: EvaluationSeedSequence::LegacySuite(vec![11]),
            stages: vec![stage(10, CoveragePolicyConfig::Total { target: 1 })],
        };

        let scheduled = schedule_candidate(
            &game(false, 3),
            &plan,
            candidate,
            &benchmarks,
            &HashMap::new(),
            &Default::default(),
        );

        assert_eq!(scheduled.len(), 2);
        assert!(scheduled.iter().all(|scheduled| {
            scheduled
                .bot_ids
                .iter()
                .filter(|id| **id == candidate)
                .count()
                == 1
        }));
    }

    #[test]
    fn deterministic_seed_sequence_is_stable_beyond_one_hundred_matches() {
        let sequence = EvaluationSeedSequence::Deterministic { key: 123_456 };
        let same_sequence = EvaluationSeedSequence::Deterministic { key: 123_456 };
        let seeds = (0..=100)
            .map(|index| sequence.seed_at(index))
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(seeds.len(), 101);
        assert_eq!(sequence.seed_at(0), 4_172_122_716_518_060_777);
        assert_eq!(sequence.seed_at(42), 4_497_731_464_451_674_930);
        assert_eq!(sequence.seed_at(42), same_sequence.seed_at(42));
        assert_ne!(sequence.seed_at(0), sequence.seed_at(100));
    }

    #[test]
    fn legacy_seed_suites_keep_their_cyclic_sequence() {
        let sequence = EvaluationSeedSequence::LegacySuite(vec![11, 22]);

        assert_eq!(sequence.seed_at(0), 11);
        assert_eq!(sequence.seed_at(1), 22);
        assert_eq!(sequence.seed_at(2), 11);
    }
}
