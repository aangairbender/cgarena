use crate::domain::{
    Bot, BotId, Build, BuildResult, BuildStatus, Leaderboard, LeaderboardId, Match, MatchAttribute,
    MatchAttributeValue, MatchFilter, MatchId, Participant,
};
use anyhow::bail;
use chrono::{DateTime, Utc};
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, SqlitePool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::warn;

#[derive(sqlx::FromRow)]
struct BotsRow {
    pub id: i64,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct MatchesRow {
    pub id: i64,
    pub seed: i64,
    pub participant_cnt: u8,
    pub replay_path: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ParticipationsRow {
    pub match_id: i64,
    pub bot_id: i64,
    pub index: u8,
    pub rank: u8,
    pub error: bool,
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
pub struct MatchAttributesJoinedRow {
    pub name: String,
    pub match_id: i64,
    pub bot_id: Option<i64>,
    pub turn: Option<u16>,
    pub value_int: Option<i64>,
    pub value_float: Option<f64>,
    pub value_string: Option<String>,
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

impl TryFrom<MatchAttributesJoinedRow> for MatchAttribute {
    type Error = anyhow::Error;

    fn try_from(row: MatchAttributesJoinedRow) -> Result<Self, Self::Error> {
        Ok(MatchAttribute {
            name: row.name,
            bot_id: row.bot_id.map(|id| id.into()),
            turn: row.turn,
            value: match (row.value_int, row.value_float, row.value_string) {
                (Some(value), None, None) => MatchAttributeValue::Integer(value),
                (None, Some(value), None) => MatchAttributeValue::Float(value),
                (None, None, Some(value)) => MatchAttributeValue::String(value),
                _ => bail!("Ambiguous attribute value type. match id {}", row.match_id),
            },
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
            source_code: bot.source_code.try_into()?,
            language: bot.language.try_into()?,
            created_at: bot.created_at,
        })
    }
}

impl From<ParticipationsRow> for Participant {
    fn from(row: ParticipationsRow) -> Self {
        Participant {
            bot_id: row.bot_id.into(),
            rank: row.rank,
            error: row.error,
        }
    }
}

impl
    TryFrom<(
        MatchesRow,
        Vec<ParticipationsRow>,
        Vec<MatchAttributesJoinedRow>,
    )> for Match
{
    type Error = anyhow::Error;

    fn try_from(
        (m, mut ps, ar): (
            MatchesRow,
            Vec<ParticipationsRow>,
            Vec<MatchAttributesJoinedRow>,
        ),
    ) -> Result<Self, Self::Error> {
        if m.participant_cnt as usize != ps.len() {
            bail!("participant count mismatch");
        }
        ps.sort_by_key(|p| p.index);
        for (index, p) in ps.iter().enumerate() {
            if index != p.index as usize {
                bail!("Some participation index is missing");
            }
        }
        Ok(Match {
            id: m.id.into(),
            seed: m.seed,
            replay_path: m.replay_path.map(PathBuf::from),
            participants: ps.into_iter().map(|p| p.into()).collect(),
            attributes: ar
                .into_iter()
                .filter_map(|item| {
                    let id = item.match_id;
                    MatchAttribute::try_from(item)
                        .inspect_err(|e| {
                            warn!(
                                "Invalid db data (match attribute of match {}): {}. Skipping.",
                                id, e
                            )
                        })
                        .ok()
                })
                .collect(),
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
        INSERT INTO bots (name, source_code, language, created_at) \
        VALUES ($1, $2, $3, $4) \
    "};

    let res = sqlx::query(SQL)
        .bind::<&str>(&bot.name)
        .bind::<&str>(&bot.source_code)
        .bind::<&str>(&bot.language)
        .bind::<DateTime<Utc>>(bot.created_at)
        .execute(pool)
        .await?;

    Ok(BotId::from(res.last_insert_rowid()))
}

/// only updates mutable fields
async fn update_bot(pool: &SqlitePool, bot: &Bot) -> anyhow::Result<()> {
    assert_ne!(bot.id, BotId::UNINITIALIZED);
    const SQL: &str = indoc! {"
        UPDATE bots SET name = $1 \
        WHERE id = $2"
    };

    let res = sqlx::query(SQL)
        .bind::<&str>(&bot.name)
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

    let match_id: MatchId =
        sqlx::query("INSERT INTO matches (seed, participant_cnt, replay_path) VALUES ($1, $2, $3)")
            .bind::<i64>(m.seed)
            .bind::<u8>(m.participants.len() as _)
            .bind(m.replay_path.as_ref().and_then(|path| path.to_str()))
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
pub struct ReplayMetadata {
    pub replay_path: Option<PathBuf>,
    pub participant_count: u8,
}

pub async fn fetch_replay_metadata(
    pool: &SqlitePool,
    match_id: MatchId,
) -> anyhow::Result<Option<ReplayMetadata>> {
    let row = sqlx::query_as::<_, (Option<String>, u8)>(
        "SELECT replay_path, participant_cnt FROM matches WHERE id = ?",
    )
    .bind::<i64>(match_id.into())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(replay_path, participant_count)| ReplayMetadata {
        replay_path: replay_path.map(PathBuf::from),
        participant_count,
    }))
}

pub async fn fetch_replay_paths_for_bot(
    pool: &SqlitePool,
    bot_id: BotId,
) -> anyhow::Result<Vec<PathBuf>> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT m.replay_path
         FROM matches m
         INNER JOIN participations p ON p.match_id = m.id
         WHERE p.bot_id = ? AND m.replay_path IS NOT NULL",
    )
    .bind::<i64>(bot_id.into())
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(PathBuf::from)
    .collect())
}

pub async fn fetch_turn_attributes(
    pool: &SqlitePool,
    match_ids: &[MatchId],
    attribute_name: &str,
) -> anyhow::Result<Vec<MatchAttribute>> {
    if match_ids.is_empty() {
        return Ok(vec![]);
    }

    let match_ids_joined = match_ids.iter().join(",");

    let sql = formatdoc! {
        "SELECT
            n.name as name,
            ma.match_id as match_id,
            ma.bot_id as bot_id,
            ma.turn as turn,
            ma.value_int as value_int,
            ma.value_float as value_float,
            NULL as value_string
        FROM match_attributes ma
        INNER JOIN match_attribute_names n ON (n.id = ma.name_id)
        WHERE n.name = $1
        AND ma.bot_id IS NOT NULL
        AND ma.turn IS NOT NULL
        AND ma.match_id IN ({match_ids_joined})"
    };

    let res: Vec<MatchAttributesJoinedRow> = sqlx::query_as(&sql)
        .bind::<&str>(attribute_name)
        .fetch_all(pool)
        .await?;

    let res = res.into_iter().flat_map(TryInto::try_into).collect();

    Ok(res)
}

pub async fn wipe_old_matches<F: Fn(usize) -> bool>(
    arena_path: &Path,
    percentage: u8,
    vacuum: bool,
    confirm: F,
) -> anyhow::Result<usize> {
    let db_path = arena_path.join(DB_FILE_NAME);

    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Delete)
        .create_if_missing(false);

    let mut conn = opts.connect().await?;
    sqlx::migrate!().run(&mut conn).await?;

    let percentage = percentage.clamp(0, 100);

    let mut match_ids: Vec<(i64,)> = sqlx::query_as("SELECT id FROM matches")
        .fetch_all(&mut conn)
        .await?;

    match_ids.sort();

    let amount_to_delete = (match_ids.len() as u64) * (percentage as u64) / 100;
    if amount_to_delete == 0 {
        println!("0 matches to delete, skipping");
    } else {
        if !confirm(amount_to_delete as usize) {
            bail!("Cancelled by the user");
        }

        assert!(amount_to_delete <= match_ids.len() as u64);
        let last_deleted_id = match_ids[amount_to_delete as usize - 1].0;
        let replay_paths = sqlx::query_scalar::<_, String>(
            "SELECT replay_path FROM matches WHERE id <= ? AND replay_path IS NOT NULL",
        )
        .bind(last_deleted_id)
        .fetch_all(&mut conn)
        .await?;

        println!("Deleting {} old matches", amount_to_delete);

        sqlx::query("DELETE FROM matches WHERE id <= $1")
            .bind::<i64>(last_deleted_id)
            .execute(&mut conn)
            .await?;
        for replay_path in replay_paths {
            crate::replay_artifact::remove(arena_path, Path::new(&replay_path)).await?;
        }
    }

    if vacuum {
        println!("Vacuuming the db");
        sqlx::query("VACUUM").execute(&mut conn).await?;
    }

    Ok(amount_to_delete as usize)
}

/// newest would come first
pub async fn fetch_matches_all(
    pool: &SqlitePool,
    filter: &MatchFilter,
) -> anyhow::Result<Vec<Match>> {
    fetch_matches(pool, filter, vec![], None, None).await
}

/// newest would come first
pub async fn fetch_matches(
    pool: &SqlitePool,
    filter: &MatchFilter,
    including_bots: Vec<BotId>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> anyhow::Result<Vec<Match>> {
    let attrs = filter.needed_attributes();
    let filter_attributes: Vec<MatchAttributesJoinedRow> = if attrs.is_empty() {
        vec![]
    } else {
        let names_joined = attrs.iter().map(|a| format!("'{}'", a.name)).join(",");
        let turns = attrs.iter().flat_map(|a| a.turn).collect_vec();
        let turns_condition = if turns.is_empty() {
            "ma.turn IS NULL".to_string()
        } else {
            format!(
                "(ma.turn is NULL OR ma.turn IN ({}))",
                turns.iter().join(",")
            )
        };
        let bot_ids: Vec<i64> = attrs
            .iter()
            .flat_map(|a| a.bot_id)
            .map(|id| id.into())
            .collect_vec();
        let bots_condition = if bot_ids.is_empty() {
            "ma.bot_id IS NULL".to_string()
        } else {
            format!(
                "(ma.bot_id is NULL OR ma.bot_id IN ({}))",
                bot_ids.iter().join(",")
            )
        };

        let sql = formatdoc! {
            "SELECT
                n.name as name,
                ma.match_id as match_id,
                ma.bot_id as bot_id,
                ma.turn as turn,
                ma.value_int as value_int,
                ma.value_float as value_float,
                v.value as value_string
            FROM match_attributes ma
            INNER JOIN match_attribute_names n ON (n.id = ma.name_id)
            LEFT JOIN match_attribute_string_values v ON (v.id = ma.value_string_id)
            WHERE n.name IN ({names_joined}) AND {turns_condition} AND {bots_condition}"
        };
        sqlx::query_as(&sql).fetch_all(pool).await?
    };

    // let matches: Vec<MatchesRow> = {
    //     let q = if !attributes.is_empty() {
    //         let match_ids: Vec<i64> = attributes.iter().map(|a| a.match_id).unique().collect();
    //         format!("id IN {}", match_ids.iter().join(","))
    //     } else {
    //         "1=1".to_string()
    //     };

    //     sqlx::query_as(&format!("SELECT * from matches WHERE {}", q))
    //         .fetch_all(pool)
    //         .await?
    // };

    let matches: Vec<MatchesRow> = sqlx::query_as("SELECT * from matches")
        .fetch_all(pool)
        .await?;

    let participations: Vec<ParticipationsRow> = sqlx::query_as("SELECT * from participations")
        .fetch_all(pool)
        .await?;

    let mut combined = HashMap::with_capacity(matches.len());
    for m in matches {
        combined.insert(m.id, (m, vec![], vec![]));
    }
    for p in participations {
        let target = combined.get_mut(&p.match_id);
        let Some(target) = target else {
            continue;
        };
        target.1.push(p);
    }
    for attr in filter_attributes {
        let target = combined.get_mut(&attr.match_id);
        let Some(target) = target else {
            continue;
        };
        target.2.push(attr);
    }

    let mut matches: Vec<Match> = combined
        .into_values()
        .filter_map(|item| {
            let id = item.0.id;
            Match::try_from(item)
                .inspect_err(|e| warn!("Invalid db data (match {}): {}. Skipping.", id, e))
                .ok()
        })
        .collect();

    matches.retain(|m| filter.matches(m));

    if !including_bots.is_empty() {
        matches.retain(|m| {
            including_bots
                .iter()
                .all(|id| m.participants.iter().any(|p| p.bot_id == *id))
        });
    }

    if offset.is_none() && limit.is_none() {
        return Ok(matches);
    }

    // important for offset & limit
    // sorting by negative it for having the newest matches first
    matches.sort_by_key(|m| -i64::from(m.id));

    if let Some(offset) = offset {
        drop(matches.drain(0..offset));
    }
    if let Some(limit) = limit {
        matches.truncate(limit);
    }

    // does not include turn attrs
    let mut match_attrs = {
        let match_ids_joined = matches.iter().map(|m| m.id).join(",");
        let sql = formatdoc! {
            "SELECT
                n.name as name,
                ma.match_id as match_id,
                ma.bot_id as bot_id,
                ma.turn as turn,
                ma.value_int as value_int,
                ma.value_float as value_float,
                NULL as value_string
            FROM match_attributes ma
            INNER JOIN match_attribute_names n ON (n.id = ma.name_id)
            WHERE ma.match_id IN ({match_ids_joined})
            AND ma.turn IS NULL"
        };

        sqlx::query_as::<_, MatchAttributesJoinedRow>(&sql)
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|e| (MatchId::from(e.match_id), e))
            .into_group_map()
    };

    for m in &mut matches {
        let Some(attrs) = match_attrs.remove(&m.id) else {
            continue;
        };
        m.attributes = attrs.into_iter().map(|a| a.try_into()).try_collect()?;
    }

    Ok(matches)
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

    #[tokio::test]
    async fn replay_metadata_preserves_paths_and_leaves_legacy_matches_unavailable() {
        let pool = in_memory().await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let legacy_id: MatchId =
            sqlx::query("INSERT INTO matches (seed, participant_cnt) VALUES (1, 2)")
                .execute(&pool)
                .await
                .unwrap()
                .last_insert_rowid()
                .into();
        let replay_path = "replays/fixture.json";
        let replay_id: MatchId = sqlx::query(
            "INSERT INTO matches (seed, participant_cnt, replay_path) VALUES (2, 4, ?)",
        )
        .bind(replay_path)
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_rowid()
        .into();

        let legacy = fetch_replay_metadata(&pool, legacy_id)
            .await
            .unwrap()
            .unwrap();
        assert!(legacy.replay_path.is_none());
        let replay = fetch_replay_metadata(&pool, replay_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(replay.replay_path, Some(PathBuf::from(replay_path)));
        assert_eq!(replay.participant_count, 4);
    }

    #[tokio::test]
    async fn wiping_matches_removes_owned_replay_artifacts() {
        let arena = tempfile::tempdir().unwrap();
        let pool = connect(arena.path()).await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let replay_path = PathBuf::from("replays/fixture.json");
        tokio::fs::create_dir_all(arena.path().join("replays"))
            .await
            .unwrap();
        tokio::fs::write(arena.path().join(&replay_path), b"fixture")
            .await
            .unwrap();
        sqlx::query("INSERT INTO matches (seed, participant_cnt, replay_path) VALUES (1, 2, ?)")
            .bind(replay_path.to_str().unwrap())
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        assert_eq!(
            wipe_old_matches(arena.path(), 100, false, |_| true)
                .await
                .unwrap(),
            1
        );
        assert!(!arena.path().join(replay_path).exists());
    }
}
