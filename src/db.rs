use crate::config::ArenaConfig;
use crate::domain::{
    Bot, BotId, Build, BuildResult, BuildStatus, Leaderboard, LeaderboardId, Match, MatchId,
};
use crate::evaluation::{
    EvaluationConfig, EvaluationPlanRevision, EvaluationSeedSequence, EvaluationStageConfig,
    EvaluationStageRevision,
};
use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use indoc::indoc;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};
use std::path::Path;
use std::time::Duration;
use tracing::warn;

#[derive(sqlx::FromRow)]
struct BotsRow {
    pub id: i64,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
    pub role: String,
    pub evaluation_plan_revision_id: Option<i64>,
}

#[derive(sqlx::FromRow)]
pub struct BuildsRow {
    pub bot_id: i64,
    pub worker_name: String,
    pub status: u8,
    pub result: Option<u8>,
    pub error: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct LeaderboardsRow {
    pub id: i64,
    pub name: String,
    pub filter: String,
}

impl TryFrom<LeaderboardsRow> for Leaderboard {
    type Error = anyhow::Error;

    fn try_from(row: LeaderboardsRow) -> Result<Self, Self::Error> {
        Ok(Leaderboard {
            id: row.id.into(),
            name: row.name.try_into()?,
            filter: row.filter.parse()?,
        })
    }
}

impl TryFrom<BuildsRow> for Build {
    type Error = anyhow::Error;

    fn try_from(row: BuildsRow) -> Result<Self, Self::Error> {
        let status = match (row.status, row.result, row.error) {
            (0, None, None) => BuildStatus::Pending,
            (1, None, None) => BuildStatus::Running,
            (2, Some(0), None) => BuildStatus::Finished(BuildResult::Success),
            (2, Some(1), Some(stderr)) => BuildStatus::Finished(BuildResult::Failure { stderr }),
            _ => bail!("unexpected build status in db"),
        };
        Ok(Build {
            bot_id: row.bot_id.into(),
            worker_name: row.worker_name.try_into()?,
            status,
        })
    }
}

impl TryFrom<BotsRow> for Bot {
    type Error = anyhow::Error;

    fn try_from(bot: BotsRow) -> Result<Self, Self::Error> {
        Ok(Bot {
            id: bot.id.into(),
            name: bot.name.try_into()?,
            source_code: bot.source_code.into(),
            language: bot.language.try_into()?,
            role: bot.role.parse()?,
            evaluation_plan_revision_id: bot.evaluation_plan_revision_id,
            created_at: bot.created_at,
        })
    }
}

const DB_FILE_NAME: &str = "cgarena.db";

pub async fn connect(arena_path: &Path) -> anyhow::Result<SqlitePool> {
    let db_path = arena_path.join(DB_FILE_NAME);

    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .log_slow_statements(log::LevelFilter::Warn, Duration::from_secs(5))
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(opts).await?;
    Ok(pool)
}
pub async fn migrate(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!()
        .run(pool)
        .await
        .context("Cannot run db migrations")
}

/// for tests
#[cfg(test)]
pub async fn in_memory() -> anyhow::Result<SqlitePool> {
    use sqlx::sqlite::SqlitePoolOptions;

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await?;
    Ok(pool)
}
pub async fn fetch_arena_config(pool: &SqlitePool) -> anyhow::Result<Option<ArenaConfig>> {
    let config_json: Option<String> =
        sqlx::query_scalar("SELECT config_json FROM arena_configuration WHERE id = 1")
            .fetch_optional(pool)
            .await?;
    let Some(config_json) = config_json else {
        return Ok(None);
    };
    let config: ArenaConfig = serde_json::from_str(&config_json)
        .context("Stored arena configuration is not valid JSON")?;
    let migrated = config.legacy_matchmaking.is_some();
    let config = config.migrate_legacy();
    if migrated {
        persist_arena_config(pool, &config).await?;
    }
    Ok(Some(config))
}

pub async fn persist_arena_config(pool: &SqlitePool, config: &ArenaConfig) -> anyhow::Result<()> {
    let mut transaction = pool.begin().await?;
    persist_arena_config_transaction(&mut transaction, config).await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn persist_arena_config_transaction(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    config: &ArenaConfig,
) -> anyhow::Result<()> {
    config.validate()?;
    let config_json =
        serde_json::to_string(config).context("Cannot serialize arena configuration")?;
    sqlx::query(
        "INSERT INTO arena_configuration (id, config_json) VALUES (1, $1) \
         ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json",
    )
    .bind(config_json)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
pub async fn ensure_evaluation_plan(
    pool: &SqlitePool,
    evaluation: &EvaluationConfig,
) -> anyhow::Result<EvaluationPlanRevision> {
    let seed_sequence = EvaluationSeedSequence::Deterministic {
        key: evaluation.seed_sequence_key,
    };
    let plan_json = serde_json::to_string(&(&seed_sequence, &evaluation.stages))
        .context("Cannot serialize evaluation plan")?;
    let mut transaction = pool.begin().await?;
    let plan_id: i64 = if let Some(id) =
        sqlx::query_scalar("SELECT id FROM evaluation_plan_revisions WHERE config_json = $1")
            .bind(&plan_json)
            .fetch_optional(&mut *transaction)
            .await?
    {
        id
    } else {
        let id = sqlx::query("INSERT INTO evaluation_plan_revisions (config_json) VALUES ($1)")
            .bind(&plan_json)
            .execute(&mut *transaction)
            .await?
            .last_insert_rowid();
        for (index, stage) in evaluation.stages.iter().enumerate() {
            let stage_json =
                serde_json::to_string(stage).context("Cannot serialize evaluation stage")?;
            sqlx::query(
                "INSERT INTO evaluation_stage_revisions \
                 (plan_revision_id, stage_index, config_json) VALUES ($1, $2, $3)",
            )
            .bind(id)
            .bind(index as i64)
            .bind(stage_json)
            .execute(&mut *transaction)
            .await?;
        }
        id
    };
    transaction.commit().await?;
    fetch_evaluation_plan(pool, plan_id).await
}

pub async fn fetch_evaluation_plan(
    pool: &SqlitePool,
    plan_id: i64,
) -> anyhow::Result<EvaluationPlanRevision> {
    let plan_json: String =
        sqlx::query_scalar("SELECT config_json FROM evaluation_plan_revisions WHERE id = $1")
            .bind(plan_id)
            .fetch_optional(pool)
            .await?
            .with_context(|| format!("evaluation plan revision {plan_id} does not exist"))?;
    let (seed_sequence, _): (EvaluationSeedSequence, Vec<EvaluationStageConfig>) =
        serde_json::from_str(&plan_json).context("Stored evaluation plan is not valid JSON")?;
    let rows: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, config_json FROM evaluation_stage_revisions \
         WHERE plan_revision_id = $1 ORDER BY stage_index",
    )
    .bind(plan_id)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        bail!("evaluation plan revision {plan_id} has no stages");
    }
    let stages = rows
        .into_iter()
        .map(|(id, json)| {
            serde_json::from_str::<EvaluationStageConfig>(&json)
                .map(|config| EvaluationStageRevision { id, config })
                .context("Stored evaluation stage is not valid JSON")
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(EvaluationPlanRevision {
        id: plan_id,
        seed_sequence,
        stages,
    })
}
#[derive(Default)]
pub struct StageProgress {
    pub matches: u64,
    pub encounters: std::collections::HashMap<BotId, u64>,
    pub matches_by_player_count: std::collections::HashMap<u32, u64>,
    pub candidate_errors: u64,
}

pub async fn fetch_stage_progress(
    pool: &SqlitePool,
    candidate_id: BotId,
    stage_revision_id: i64,
) -> anyhow::Result<StageProgress> {
    let candidate_id: i64 = candidate_id.into();
    let matches: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM matches \
         WHERE candidate_bot_id = $1 AND evaluation_stage_revision_id = $2",
    )
    .bind(candidate_id)
    .bind(stage_revision_id)
    .fetch_one(pool)
    .await?;
    let encounter_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT p.bot_id, COUNT(*) FROM matches m \
         JOIN participations p ON p.match_id = m.id \
         WHERE m.candidate_bot_id = $1 AND m.evaluation_stage_revision_id = $2 \
           AND p.bot_id != $1 GROUP BY p.bot_id",
    )
    .bind(candidate_id)
    .bind(stage_revision_id)
    .fetch_all(pool)
    .await?;
    let player_count_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT participant_cnt, COUNT(*) FROM matches \
         WHERE candidate_bot_id = $1 AND evaluation_stage_revision_id = $2 \
         GROUP BY participant_cnt",
    )
    .bind(candidate_id)
    .bind(stage_revision_id)
    .fetch_all(pool)
    .await?;
    let candidate_errors: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM matches m \
         JOIN participations p ON p.match_id = m.id AND p.bot_id = $1 \
         WHERE m.candidate_bot_id = $1 AND m.evaluation_stage_revision_id = $2 \
           AND p.error = 1",
    )
    .bind(candidate_id)
    .bind(stage_revision_id)
    .fetch_one(pool)
    .await?;
    Ok(StageProgress {
        matches: matches as u64,
        encounters: encounter_rows
            .into_iter()
            .map(|(id, count)| (id.into(), count as u64))
            .collect(),
        matches_by_player_count: player_count_rows
            .into_iter()
            .map(|(count, matches)| (count as u32, matches as u64))
            .collect(),
        candidate_errors: candidate_errors as u64,
    })
}

pub async fn persist_bot(pool: &SqlitePool, bot: &mut Bot) -> anyhow::Result<()> {
    if bot.id == BotId::UNINITIALIZED {
        bot.id = insert_bot(pool, bot).await?;
    } else {
        update_bot(pool, bot).await?;
    }
    Ok(())
}

async fn insert_bot(pool: &SqlitePool, bot: &Bot) -> anyhow::Result<BotId> {
    assert_eq!(bot.id, BotId::UNINITIALIZED);
    const SQL: &str = indoc! {"
        INSERT INTO bots (name, source_code, language, role, evaluation_plan_revision_id, created_at) \
        VALUES ($1, $2, $3, $4, $5, $6) \
    "};
    let res = sqlx::query(SQL)
        .bind::<&str>(&bot.name)
        .bind::<&str>(&bot.source_code)
        .bind::<&str>(&bot.language)
        .bind(bot.role.to_string())
        .bind(bot.evaluation_plan_revision_id)
        .bind::<DateTime<Utc>>(bot.created_at)
        .execute(pool)
        .await?;

    Ok(BotId::from(res.last_insert_rowid()))
}

/// only updates mutable fields
async fn update_bot(pool: &SqlitePool, bot: &Bot) -> anyhow::Result<()> {
    assert_ne!(bot.id, BotId::UNINITIALIZED);
    const SQL: &str = indoc! {"
        UPDATE bots SET name = $1, role = $2, evaluation_plan_revision_id = $3 \
        WHERE id = $4"
    };

    let res = sqlx::query(SQL)
        .bind::<&str>(&bot.name)
        .bind(bot.role.to_string())
        .bind(bot.evaluation_plan_revision_id)
        .bind::<i64>(bot.id.into())
        .execute(pool)
        .await?;

    assert_eq!(res.rows_affected(), 1);
    Ok(())
}

pub async fn delete_bot(pool: &SqlitePool, id: BotId) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM bots WHERE id = $1")
        .bind::<i64>(id.into())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn fetch_bots(pool: &SqlitePool) -> anyhow::Result<Vec<Bot>> {
    let bots = sqlx::query_as::<_, BotsRow>("SELECT * from bots")
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|item| {
            let id = item.id;
            Bot::try_from(item)
                .inspect_err(|e| warn!("Invalid db data (bot {}): {}. Skipping.", id, e))
                .ok()
        })
        .collect();
    Ok(bots)
}

pub async fn fetch_builds(pool: &SqlitePool) -> anyhow::Result<Vec<Build>> {
    let builds = sqlx::query_as::<_, BuildsRow>("SELECT * from builds")
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|item| {
            let id = (item.worker_name.clone(), item.bot_id);
            Build::try_from(item)
                .inspect_err(|e| warn!("Invalid db data (build {:?}): {}. Skipping.", id, e))
                .ok()
        })
        .collect();
    Ok(builds)
}

pub async fn persist_build(pool: &SqlitePool, build: &Build) -> anyhow::Result<()> {
    const SQL: &str = indoc! {"
        INSERT OR REPLACE INTO builds (bot_id, worker_name, status, result, error) \
        VALUES ($1, $2, $3, $4, $5) \
    "};

    let (status, result, error) = match &build.status {
        BuildStatus::Pending => (0, None, None),
        BuildStatus::Running => (1, None, None),
        BuildStatus::Finished(BuildResult::Success) => (2, Some(0), None),
        BuildStatus::Finished(BuildResult::Failure { stderr }) => {
            (2, Some(1), Some(stderr.as_ref()))
        }
    };

    sqlx::query(SQL)
        .bind::<i64>(build.bot_id.into())
        .bind::<&str>(&build.worker_name)
        .bind::<u8>(status)
        .bind::<Option<u8>>(result)
        .bind::<Option<&str>>(error)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn persist_match(pool: &SqlitePool, m: &mut Match) -> anyhow::Result<()> {
    assert_eq!(m.id, MatchId::UNINITIALIZED);
    m.id = create_match(pool, m).await?;
    Ok(())
}

pub async fn create_match(pool: &SqlitePool, m: &Match) -> anyhow::Result<MatchId> {
    let mut tx = pool.begin().await?;

    let match_id: MatchId = sqlx::query(
        "INSERT INTO matches \
         (seed, participant_cnt, replay_path, candidate_bot_id, evaluation_stage_revision_id) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind::<i64>(m.seed)
    .bind::<u8>(m.participants.len() as _)
    .bind(m.replay_path.as_ref().and_then(|path| path.to_str()))
    .bind::<Option<i64>>(m.candidate_bot_id.map(Into::into))
    .bind(m.evaluation_stage_revision_id)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid()
    .into();

    for (index, p) in m.participants.iter().enumerate() {
        const SQL: &str = indoc! {
            "INSERT INTO participations (match_id, bot_id, `index`, rank, error) \
                VALUES ($1, $2, $3, $4, $5)"
        };

        sqlx::query(SQL)
            .bind::<i64>(match_id.into())
            .bind::<i64>(p.bot_id.into())
            .bind::<u8>(index as _)
            .bind::<u8>(p.rank)
            .bind::<bool>(p.error)
            .execute(&mut *tx)
            .await?;
    }

    for attr in &m.attributes {
        sqlx::query("INSERT OR IGNORE INTO match_attribute_names (name) VALUES (?)")
            .bind::<&str>(&attr.name)
            .execute(&mut *tx)
            .await?;

        let name_id =
            sqlx::query_as::<_, (i64,)>("SELECT id FROM match_attribute_names WHERE name = ?")
                .bind::<&str>(&attr.name)
                .fetch_one(&mut *tx)
                .await?
                .0;

        let str_value_id = if let Some(str_value) = attr.value.string_value() {
            sqlx::query("INSERT OR IGNORE INTO match_attribute_string_values (value) VALUES (?)")
                .bind::<&str>(str_value)
                .execute(&mut *tx)
                .await?;

            let str_value_id = sqlx::query_as::<_, (i64,)>(
                "SELECT id FROM match_attribute_string_values WHERE value = ?",
            )
            .bind::<&str>(str_value)
            .fetch_one(&mut *tx)
            .await?
            .0;
            Some(str_value_id)
        } else {
            None
        };

        const SQL: &str = indoc! {
            "INSERT INTO match_attributes (name_id, match_id, bot_id, turn, value_int, value_float, value_string_id) \
            VALUES ($1, $2, $3, $4, $5, $6, $7)"
        };

        sqlx::query(SQL)
            .bind::<i64>(name_id)
            .bind::<i64>(match_id.into())
            .bind::<Option<i64>>(attr.bot_id.map(|id| id.into()))
            .bind::<Option<u16>>(attr.turn)
            .bind::<Option<i64>>(attr.value.integer_value())
            .bind::<Option<f64>>(attr.value.float_value())
            .bind::<Option<i64>>(str_value_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(match_id)
}

pub async fn mark_replay_watched(pool: &SqlitePool, id: MatchId) -> anyhow::Result<()> {
    let result = sqlx::query("UPDATE matches SET replay_watched_at = unixepoch() WHERE id = $1")
        .bind::<i64>(id.into())
        .execute(pool)
        .await?;
    assert_eq!(result.rows_affected(), 1);
    Ok(())
}

pub async fn persist_leaderboard(
    pool: &SqlitePool,
    leaderboard: &mut Leaderboard,
) -> anyhow::Result<()> {
    if leaderboard.id == LeaderboardId::UNINITIALIZED {
        leaderboard.id = insert_leaderboard(pool, leaderboard).await?;
    } else {
        update_leaderboard(pool, leaderboard).await?;
    }
    Ok(())
}

async fn insert_leaderboard(
    pool: &SqlitePool,
    leaderboard: &Leaderboard,
) -> anyhow::Result<LeaderboardId> {
    assert_eq!(leaderboard.id, LeaderboardId::UNINITIALIZED);
    const SQL: &str = indoc! {"
        INSERT INTO leaderboards (name, filter) \
        VALUES ($1, $2) \
    "};

    let res = sqlx::query(SQL)
        .bind::<&str>(&leaderboard.name)
        .bind::<&str>(&leaderboard.filter.to_string())
        .execute(pool)
        .await?;

    Ok(LeaderboardId::from(res.last_insert_rowid()))
}

/// only updates mutable fields
async fn update_leaderboard(pool: &SqlitePool, leaderboard: &Leaderboard) -> anyhow::Result<()> {
    assert_ne!(leaderboard.id, LeaderboardId::UNINITIALIZED);
    const SQL: &str = indoc! {"
        UPDATE leaderboards SET name = $1, filter = $2 \
        WHERE id = $3"
    };

    let res = sqlx::query(SQL)
        .bind::<&str>(&leaderboard.name)
        .bind::<&str>(&leaderboard.filter.to_string())
        .bind::<i64>(leaderboard.id.into())
        .execute(pool)
        .await?;

    assert_eq!(res.rows_affected(), 1);
    Ok(())
}

pub async fn delete_leaderboard(pool: &SqlitePool, id: LeaderboardId) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM leaderboards WHERE id = $1")
        .bind::<i64>(id.into())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn fetch_leaderboards(pool: &SqlitePool) -> anyhow::Result<Vec<Leaderboard>> {
    let leaderboards = sqlx::query_as::<_, LeaderboardsRow>("SELECT * from leaderboards")
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|item| {
            let id = item.id;
            Leaderboard::try_from(item)
                .inspect_err(|e| warn!("Invalid db data (leaderboard {}): {}. Skipping.", id, e))
                .ok()
        })
        .collect();
    Ok(leaderboards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::SeedSourceConfig;
    use std::borrow::Cow;

    #[tokio::test]
    async fn current_migrations_upgrade_pre_replay_database() {
        let pool = in_memory().await.unwrap();
        let current = sqlx::migrate!();
        let historical = sqlx::migrate::Migrator {
            migrations: Cow::Owned(
                current
                    .iter()
                    .filter(|migration| migration.version < 20260901160000)
                    .cloned()
                    .collect(),
            ),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };
        historical.run(&pool).await.unwrap();
        sqlx::query("INSERT INTO matches (seed, participant_cnt) VALUES (7, 2)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO bots (name, source_code, language, created_at) \
             VALUES ('legacy', 'source', 'rust', CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        current.run(&pool).await.unwrap();

        let replay_columns: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('matches') WHERE name = 'replay_path'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let replay_watched_columns: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('matches') \
             WHERE name = 'replay_watched_at'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let old_matches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM matches WHERE seed = 7")
            .fetch_one(&pool)
            .await
            .unwrap();
        let old_replay_watched_at: Option<i64> =
            sqlx::query_scalar("SELECT replay_watched_at FROM matches WHERE seed = 7")
                .fetch_one(&pool)
                .await
                .unwrap();
        sqlx::query("INSERT INTO matches (seed, participant_cnt) VALUES (8, 2)")
            .execute(&pool)
            .await
            .unwrap();
        let new_replay_watched_at: Option<i64> =
            sqlx::query_scalar("SELECT replay_watched_at FROM matches WHERE seed = 8")
                .fetch_one(&pool)
                .await
                .unwrap();
        let migrated_role: String =
            sqlx::query_scalar("SELECT role FROM bots WHERE name = 'legacy'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(replay_columns, 1);
        assert_eq!(replay_watched_columns, 1);
        assert!(old_replay_watched_at.is_some());
        assert!(new_replay_watched_at.is_none());
        assert_eq!(old_matches, 1);
        assert_eq!(migrated_role, "benchmark");
    }

    #[tokio::test]
    async fn seed_sequence_migration_replaces_the_stored_suite_with_a_key() {
        let pool = in_memory().await.unwrap();
        let current = sqlx::migrate!();
        let historical = sqlx::migrate::Migrator {
            migrations: Cow::Owned(
                current
                    .iter()
                    .filter(|migration| migration.version < 20260907130000)
                    .cloned()
                    .collect(),
            ),
            ignore_missing: false,
            locking: true,
            no_tx: false,
        };
        historical.run(&pool).await.unwrap();
        let config = serde_json::json!({
            "evaluation": {
                "enabled_on_start": true,
                "generated_seeds": [11, 22],
                "stages": []
            }
        });
        sqlx::query("INSERT INTO arena_configuration (id, config_json) VALUES (1, $1)")
            .bind(config.to_string())
            .execute(&pool)
            .await
            .unwrap();

        current.run(&pool).await.unwrap();

        let stored: String =
            sqlx::query_scalar("SELECT config_json FROM arena_configuration WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
        assert!(stored["evaluation"]["seed_sequence_key"].as_u64().is_some());
        assert!(stored["evaluation"].get("generated_seeds").is_none());
    }

    #[tokio::test]
    async fn evaluation_plan_edits_create_immutable_revisions() {
        let pool = in_memory().await.unwrap();
        migrate(&pool).await.unwrap();
        let first = EvaluationConfig::default();
        let first_revision = ensure_evaluation_plan(&pool, &first).await.unwrap();
        let same_revision = ensure_evaluation_plan(&pool, &first).await.unwrap();
        assert_eq!(first_revision.id, same_revision.id);

        let mut edited = first.clone();
        edited.stages[0].name = "Edited stage".to_string();
        let edited_revision = ensure_evaluation_plan(&pool, &edited).await.unwrap();
        assert_ne!(first_revision.id, edited_revision.id);

        let original = fetch_evaluation_plan(&pool, first_revision.id)
            .await
            .unwrap();
        assert_eq!(original.stages[0].config.name, "Generated sequence");
    }

    #[tokio::test]
    async fn legacy_seed_suite_plan_revisions_remain_readable() {
        let pool = in_memory().await.unwrap();
        migrate(&pool).await.unwrap();
        let evaluation = EvaluationConfig::default();
        let plan_json = serde_json::to_string(&(vec![11_i64, 22], &evaluation.stages)).unwrap();
        let plan_id =
            sqlx::query("INSERT INTO evaluation_plan_revisions (config_json) VALUES ($1)")
                .bind(plan_json)
                .execute(&pool)
                .await
                .unwrap()
                .last_insert_rowid();
        let stage_json = serde_json::to_string(&evaluation.stages[0])
            .unwrap()
            .replace("\"generated\"", "\"generated_static\"");
        sqlx::query(
            "INSERT INTO evaluation_stage_revisions \
             (plan_revision_id, stage_index, config_json) VALUES ($1, 0, $2)",
        )
        .bind(plan_id)
        .bind(stage_json)
        .execute(&pool)
        .await
        .unwrap();

        let plan = fetch_evaluation_plan(&pool, plan_id).await.unwrap();

        assert_eq!(
            plan.seed_sequence,
            EvaluationSeedSequence::LegacySuite(vec![11, 22])
        );
        assert_eq!(
            plan.stages[0].config.seed_source,
            SeedSourceConfig::Generated
        );
    }

    #[tokio::test]
    async fn stored_matchmaking_configuration_migrates_once() {
        let pool = in_memory().await.unwrap();
        migrate(&pool).await.unwrap();
        let mut value = serde_json::to_value(ArenaConfig::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("evaluation");
        object.insert(
            "matchmaking".to_string(),
            serde_json::json!({
                "algorithm": "v2",
                "min_matches_against_best": 5,
                "min_matches_per_pair": 7,
                "max_matches": 50,
                "enabled_on_start": false
            }),
        );
        sqlx::query("INSERT INTO arena_configuration (id, config_json) VALUES (1, $1)")
            .bind(value.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let migrated = fetch_arena_config(&pool).await.unwrap().unwrap();
        assert_eq!(migrated.evaluation.enabled_on_start, Some(false));
        assert_eq!(
            migrated.evaluation.stages[0].coverage,
            crate::evaluation::CoveragePolicyConfig::PerBenchmark { target: 7 }
        );
        let stored: String =
            sqlx::query_scalar("SELECT config_json FROM arena_configuration WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(!stored.contains("\"matchmaking\""));
        assert!(stored.contains("\"evaluation\""));
    }
}
