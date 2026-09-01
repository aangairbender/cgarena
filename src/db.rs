use crate::domain::{
    Bot, BotId, Build, BuildResult, BuildStatus, Leaderboard, LeaderboardId, Match, MatchAttribute,
    MatchAttributeValue, MatchFilter, MatchId, Participant,
};
use anyhow::bail;
use chrono::{DateTime, Utc};
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, QueryBuilder, Sqlite, SqlitePool};
use std::collections::{HashMap, HashSet};
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
}

#[derive(sqlx::FromRow)]
struct MatchesRow {
    pub id: i64,
    pub seed: i64,
    pub participant_cnt: u8,
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
        sqlx::query("INSERT INTO matches (seed, participant_cnt) VALUES ($1, $2)")
            .bind::<i64>(m.seed)
            .bind::<u8>(m.participants.len() as _)
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

        println!("Deleting {} old matches", amount_to_delete);

        sqlx::query("DELETE FROM matches WHERE id <= $1")
            .bind::<i64>(last_deleted_id)
            .execute(&mut conn)
            .await?;
    }

    if vacuum {
        println!("Vacuuming the db");
        sqlx::query("VACUUM").execute(&mut conn).await?;
    }

    Ok(amount_to_delete as usize)
}

pub struct MatchPage {
    pub matches: Vec<Match>,
    pub has_more: bool,
}

/// Fetches every matching match, newest first.
pub async fn fetch_matches_all(
    pool: &SqlitePool,
    filter: &MatchFilter,
) -> anyhow::Result<Vec<Match>> {
    collect_matching_matches(pool, filter, &HashSet::new(), 0, None).await
}

/// Fetches a page of matching matches, newest first.
///
/// Candidate matches are read in bounded batches because filters are evaluated
/// in Rust. Bot requirements and candidate ordering are applied by SQLite.
pub async fn fetch_matches(
    pool: &SqlitePool,
    filter: &MatchFilter,
    including_bots: Vec<BotId>,
    offset: usize,
    limit: usize,
) -> anyhow::Result<MatchPage> {
    let including_bots = including_bots
        .into_iter()
        .map(i64::from)
        .collect::<HashSet<_>>();
    let requested_matches = limit.saturating_add(1);
    let mut matches = if filter.needed_attributes().is_empty() {
        let empty_match = Match::new(0, vec![], vec![]);
        if !filter.matches(&empty_match) {
            return Ok(MatchPage {
                matches: vec![],
                has_more: false,
            });
        }
        fetch_unfiltered_matches(pool, &including_bots, offset, requested_matches).await?
    } else {
        collect_matching_matches(
            pool,
            filter,
            &including_bots,
            offset,
            Some(requested_matches),
        )
        .await?
    };

    let has_more = matches.len() > limit;
    matches.truncate(limit);
    hydrate_non_turn_attributes(pool, &mut matches).await?;

    Ok(MatchPage { matches, has_more })
}

async fn fetch_unfiltered_matches(
    pool: &SqlitePool,
    including_bots: &HashSet<i64>,
    offset: usize,
    limit: usize,
) -> anyhow::Result<Vec<Match>> {
    let candidates = fetch_match_candidates(pool, including_bots, None, offset, limit).await?;
    let candidate_ids = candidates
        .iter()
        .map(|candidate| candidate.id)
        .collect_vec();
    let mut participations = fetch_participations(pool, &candidate_ids).await?;

    Ok(candidates
        .into_iter()
        .filter_map(|candidate| {
            let candidate_id = candidate.id;
            Match::try_from((
                candidate,
                participations.remove(&candidate_id).unwrap_or_default(),
                vec![],
            ))
            .inspect_err(|error| {
                warn!(
                    "Invalid db data (match {}): {}. Skipping.",
                    candidate_id, error
                )
            })
            .ok()
        })
        .collect())
}

async fn collect_matching_matches(
    pool: &SqlitePool,
    filter: &MatchFilter,
    including_bots: &HashSet<i64>,
    offset: usize,
    max_matches: Option<usize>,
) -> anyhow::Result<Vec<Match>> {
    const CANDIDATE_BATCH_SIZE: usize = 256;

    let needed_attributes = filter
        .needed_attributes()
        .into_iter()
        .map(|attr| (attr.name, attr.bot_id.map(i64::from), attr.turn))
        .unique()
        .collect_vec();
    let mut before_id = None;
    let mut matching_before_page = 0;
    let mut matches = Vec::new();

    loop {
        let candidates =
            fetch_match_candidates(pool, including_bots, before_id, 0, CANDIDATE_BATCH_SIZE)
                .await?;
        if candidates.is_empty() {
            break;
        }
        before_id = candidates.last().map(|candidate| candidate.id);

        let candidate_ids = candidates
            .iter()
            .map(|candidate| candidate.id)
            .collect_vec();
        let candidate_count = candidate_ids.len();
        let mut participations = fetch_participations(pool, &candidate_ids).await?;
        let mut attributes =
            fetch_filter_attributes(pool, &candidate_ids, &needed_attributes).await?;

        for candidate in candidates {
            let candidate_id = candidate.id;
            let item = (
                candidate,
                participations.remove(&candidate_id).unwrap_or_default(),
                attributes.remove(&candidate_id).unwrap_or_default(),
            );
            let candidate = match Match::try_from(item) {
                Ok(candidate) => candidate,
                Err(error) => {
                    warn!(
                        "Invalid db data (match {}): {}. Skipping.",
                        candidate_id, error
                    );
                    continue;
                }
            };
            if !filter.matches(&candidate) {
                continue;
            }
            if matching_before_page < offset {
                matching_before_page += 1;
                continue;
            }

            matches.push(candidate);
            if max_matches.is_some_and(|maximum| matches.len() >= maximum) {
                return Ok(matches);
            }
        }

        if candidate_count < CANDIDATE_BATCH_SIZE {
            break;
        }
    }

    Ok(matches)
}

async fn fetch_match_candidates(
    pool: &SqlitePool,
    including_bots: &HashSet<i64>,
    before_id: Option<i64>,
    offset: usize,
    limit: usize,
) -> anyhow::Result<Vec<MatchesRow>> {
    let mut query =
        QueryBuilder::<Sqlite>::new("SELECT m.id, m.seed, m.participant_cnt FROM matches m");
    let mut has_where = false;

    for bot_id in including_bots.iter().sorted() {
        query.push(if has_where {
            " AND EXISTS ("
        } else {
            " WHERE EXISTS ("
        });
        query.push(
            "SELECT 1 FROM participations p \
             WHERE p.match_id = m.id AND p.bot_id = ",
        );
        query.push_bind(bot_id);
        query.push(")");
        has_where = true;
    }

    if let Some(before_id) = before_id {
        query.push(if has_where {
            " AND m.id < "
        } else {
            " WHERE m.id < "
        });
        query.push_bind(before_id);
    }

    query.push(" ORDER BY m.id DESC LIMIT ");
    query.push_bind(i64::try_from(limit)?);
    if offset != 0 {
        query.push(" OFFSET ");
        query.push_bind(i64::try_from(offset)?);
    }
    Ok(query.build_query_as().fetch_all(pool).await?)
}

async fn fetch_participations(
    pool: &SqlitePool,
    match_ids: &[i64],
) -> anyhow::Result<HashMap<i64, Vec<ParticipationsRow>>> {
    if match_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT match_id, bot_id, `index`, rank, error FROM participations WHERE match_id IN (",
    );
    let mut separated = query.separated(", ");
    for match_id in match_ids {
        separated.push_bind(match_id);
    }
    separated.push_unseparated(")");

    Ok(query
        .build_query_as::<ParticipationsRow>()
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| (row.match_id, row))
        .into_group_map())
}

async fn fetch_filter_attributes(
    pool: &SqlitePool,
    match_ids: &[i64],
    needed_attributes: &[(String, Option<i64>, Option<u16>)],
) -> anyhow::Result<HashMap<i64, Vec<MatchAttributesJoinedRow>>> {
    if match_ids.is_empty() || needed_attributes.is_empty() {
        return Ok(HashMap::new());
    }

    let mut query = joined_attributes_query();
    query.push(" WHERE ma.match_id IN (");
    let mut separated = query.separated(", ");
    for match_id in match_ids {
        separated.push_bind(match_id);
    }
    separated.push_unseparated(") AND (");

    for (index, (name, bot_id, turn)) in needed_attributes.iter().enumerate() {
        if index != 0 {
            query.push(" OR ");
        }
        query.push("(n.name = ");
        query.push_bind(name);
        match bot_id {
            Some(bot_id) => {
                query.push(" AND ma.bot_id = ");
                query.push_bind(bot_id);
            }
            None => {
                query.push(" AND ma.bot_id IS NULL");
            }
        }
        match turn {
            Some(turn) => {
                query.push(" AND ma.turn = ");
                query.push_bind(turn);
            }
            None => {
                query.push(" AND ma.turn IS NULL");
            }
        }
        query.push(")");
    }
    query.push(")");

    Ok(query
        .build_query_as::<MatchAttributesJoinedRow>()
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| (row.match_id, row))
        .into_group_map())
}

async fn hydrate_non_turn_attributes(
    pool: &SqlitePool,
    matches: &mut [Match],
) -> anyhow::Result<()> {
    if matches.is_empty() {
        return Ok(());
    }

    let mut query = joined_attributes_query();
    query.push(" WHERE ma.turn IS NULL AND ma.match_id IN (");
    let mut separated = query.separated(", ");
    for item in matches.iter() {
        separated.push_bind(i64::from(item.id));
    }
    separated.push_unseparated(")");

    let mut attributes = query
        .build_query_as::<MatchAttributesJoinedRow>()
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| (row.match_id, row))
        .into_group_map();

    for item in matches {
        item.attributes = attributes
            .remove(&i64::from(item.id))
            .unwrap_or_default()
            .into_iter()
            .filter_map(|attribute| {
                let match_id = attribute.match_id;
                MatchAttribute::try_from(attribute)
                    .inspect_err(|error| {
                        warn!(
                            "Invalid db data (match attribute of match {}): {}. Skipping.",
                            match_id, error
                        )
                    })
                    .ok()
            })
            .collect();
    }
    Ok(())
}

fn joined_attributes_query<'args>() -> QueryBuilder<'args, Sqlite> {
    QueryBuilder::new(
        "SELECT \
         n.name AS name, \
         ma.match_id AS match_id, \
         ma.bot_id AS bot_id, \
         ma.turn AS turn, \
         ma.value_int AS value_int, \
         ma.value_float AS value_float, \
         v.value AS value_string \
         FROM match_attributes ma \
         INNER JOIN match_attribute_names n ON n.id = ma.name_id \
         LEFT JOIN match_attribute_string_values v ON v.id = ma.value_string_id",
    )
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

    async fn database() -> SqlitePool {
        let pool = in_memory().await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    async fn insert_test_bot(pool: &SqlitePool, name: &str) -> BotId {
        let result = sqlx::query(
            "INSERT INTO bots (name, source_code, language, created_at) VALUES (?, '', 'rust', 0)",
        )
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
        result.last_insert_rowid().into()
    }

    async fn insert_test_match(
        pool: &SqlitePool,
        seed: i64,
        bot_ids: &[BotId],
        attributes: Vec<MatchAttribute>,
    ) -> MatchId {
        let participants = bot_ids
            .iter()
            .enumerate()
            .map(|(index, bot_id)| Participant {
                bot_id: *bot_id,
                rank: index as u8,
                error: false,
            })
            .collect();
        create_match(pool, &Match::new(seed, participants, attributes))
            .await
            .unwrap()
    }

    fn ids(page: &MatchPage) -> Vec<MatchId> {
        page.matches.iter().map(|item| item.id).collect()
    }

    #[tokio::test]
    async fn paginates_empty_exact_and_past_end_in_newest_order() {
        let pool = database().await;
        let bot = insert_test_bot(&pool, "bot").await;
        let filter = MatchFilter::accept_all();

        let empty = fetch_matches(&pool, &filter, vec![], 0, 2).await.unwrap();
        assert!(empty.matches.is_empty());
        assert!(!empty.has_more);

        let oldest = insert_test_match(&pool, 1, &[bot], vec![]).await;
        let middle = insert_test_match(&pool, 2, &[bot], vec![]).await;
        let newest = insert_test_match(&pool, 3, &[bot], vec![]).await;

        let first = fetch_matches(&pool, &filter, vec![], 0, 2).await.unwrap();
        assert_eq!(ids(&first), vec![newest, middle]);
        assert!(first.has_more);

        let exact_last = fetch_matches(&pool, &filter, vec![], 2, 1).await.unwrap();
        assert_eq!(ids(&exact_last), vec![oldest]);
        assert!(!exact_last.has_more);

        let exact_end = fetch_matches(&pool, &filter, vec![], 3, 1).await.unwrap();
        assert!(exact_end.matches.is_empty());
        assert!(!exact_end.has_more);

        let past_end = fetch_matches(&pool, &filter, vec![], 100, 10)
            .await
            .unwrap();
        assert!(past_end.matches.is_empty());
        assert!(!past_end.has_more);
    }

    #[tokio::test]
    async fn constant_false_filter_returns_no_candidates() {
        let pool = database().await;
        let bot = insert_test_bot(&pool, "bot").await;
        insert_test_match(&pool, 1, &[bot], vec![]).await;
        let filter: MatchFilter = "1 == 2".parse().unwrap();

        let page = fetch_matches(&pool, &filter, vec![], 0, 10).await.unwrap();

        assert!(page.matches.is_empty());
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn requires_every_included_bot_in_sql() {
        let pool = database().await;
        let bot_1 = insert_test_bot(&pool, "bot 1").await;
        let bot_2 = insert_test_bot(&pool, "bot 2").await;
        let bot_3 = insert_test_bot(&pool, "bot 3").await;
        let filter = MatchFilter::accept_all();

        let both_old = insert_test_match(&pool, 1, &[bot_1, bot_2], vec![]).await;
        insert_test_match(&pool, 2, &[bot_1, bot_3], vec![]).await;
        let both_new = insert_test_match(&pool, 3, &[bot_1, bot_2, bot_3], vec![]).await;
        insert_test_match(&pool, 4, &[bot_2, bot_3], vec![]).await;

        let page = fetch_matches(&pool, &filter, vec![bot_1, bot_2, bot_1], 0, 10)
            .await
            .unwrap();
        assert_eq!(ids(&page), vec![both_new, both_old]);
        assert!(!page.has_more);
    }

    #[tokio::test]
    async fn fills_filtered_pages_across_candidate_batches() {
        let pool = database().await;
        let bot = insert_test_bot(&pool, "bot").await;
        let score = |value| MatchAttribute {
            name: "score".to_string(),
            bot_id: Some(bot),
            turn: None,
            value: MatchAttributeValue::Integer(value),
        };
        let label = |value: &str| MatchAttribute {
            name: "label".to_string(),
            bot_id: None,
            turn: None,
            value: MatchAttributeValue::String(value.to_string()),
        };

        let older_match =
            insert_test_match(&pool, 1, &[bot], vec![score(10), label("older")]).await;
        let newer_match =
            insert_test_match(&pool, 2, &[bot], vec![score(20), label("newer")]).await;
        for seed in 3..259 {
            insert_test_match(&pool, seed, &[bot], vec![]).await;
        }

        let filter: MatchFilter = format!("bot({bot}).score >= 10").parse().unwrap();
        let first = fetch_matches(&pool, &filter, vec![], 0, 1).await.unwrap();
        assert_eq!(ids(&first), vec![newer_match]);
        assert!(first.has_more);
        assert!(first.matches[0].attributes.iter().any(|attribute| {
            attribute.name == "label"
                && matches!(
                    &attribute.value,
                    MatchAttributeValue::String(value) if value == "newer"
                )
        }));

        let second = fetch_matches(&pool, &filter, vec![], 1, 1).await.unwrap();
        assert_eq!(ids(&second), vec![older_match]);
        assert!(!second.has_more);

        let end = fetch_matches(&pool, &filter, vec![], 2, 1).await.unwrap();
        assert!(end.matches.is_empty());
        assert!(!end.has_more);
    }

    #[tokio::test]
    async fn fetches_all_matches_across_candidate_batches_without_duplicates() {
        let pool = database().await;
        let bot = insert_test_bot(&pool, "bot").await;
        let mut inserted = Vec::new();
        for seed in 0..300 {
            inserted.push(insert_test_match(&pool, seed, &[bot], vec![]).await);
        }

        let matches = fetch_matches_all(&pool, &MatchFilter::accept_all())
            .await
            .unwrap();
        let ids = matches.iter().map(|item| item.id).collect_vec();
        let expected = inserted.into_iter().rev().collect_vec();

        assert_eq!(ids, expected);
    }
}
