use crate::domain::{
    Bot, BotId, Build, BuildResult, BuildStatus, Match, MatchAttribute, MatchId, Participant,
};
use anyhow::bail;
use chrono::{DateTime, Utc};
use indoc::indoc;
use sqlx::SqlitePool;
use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions};
use std::collections::HashMap;
use std::path::Path;
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
pub struct MatchAttributesRow {
    #[allow(unused)]
    pub id: i64,
    pub name: String,
    pub match_id: i64,
    pub bot_id: Option<i64>,
    pub turn: Option<u16>,
    pub value: String,
}

impl From<MatchAttributesRow> for MatchAttribute {
    fn from(row: MatchAttributesRow) -> Self {
        MatchAttribute {
            name: row.name,
            bot_id: row.bot_id.map(|id| id.into()),
            turn: row.turn,
            value: row.value,
        }
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

impl TryFrom<(MatchesRow, Vec<ParticipationsRow>, Vec<MatchAttributesRow>)> for Match {
    type Error = anyhow::Error;

    fn try_from(
        (m, mut ps, ar): (MatchesRow, Vec<ParticipationsRow>, Vec<MatchAttributesRow>),
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
            attributes: ar.into_iter().map(|p| p.into()).collect(),
        })
    }
}

pub struct Database {
    pool: SqlitePool,
}

const DB_FILE_NAME: &str = "cgarena.db";

impl Database {
    pub async fn connect(arena_path: &Path) -> Self {
        let db_path = arena_path.join(DB_FILE_NAME);

        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(opts)
            .await
            .expect("cannot connect to db");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("can't run migrations");

        Self { pool }
    }

    pub async fn vacuum_db(arena_path: &Path) {
        let db_path = arena_path.join(DB_FILE_NAME);

        let mut conn = SqliteConnectOptions::new()
            .filename(db_path)
            .connect()
            .await
            .expect("cannot connect to database");

        sqlx::query("VACUUM")
            .execute(&mut conn)
            .await
            .expect("can't vacuum the db");
    }

    /// for tests
    #[cfg(test)]
    pub async fn in_memory() -> (Self, SqlitePool) {
        use sqlx::sqlite::SqlitePoolOptions;

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("can't run migrations");

        (Self { pool: pool.clone() }, pool)
    }

    pub async fn persist_bot(&mut self, bot: &mut Bot) {
        if bot.id == BotId::UNINITIALIZED {
            bot.id = self.insert_bot(bot).await;
        } else {
            self.update_bot(bot).await;
        }
    }

    async fn insert_bot(&mut self, bot: &Bot) -> BotId {
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
            .execute(&self.pool)
            .await
            .expect("Cannot insert bot to db");

        BotId::from(res.last_insert_rowid())
    }

    /// only updates mutable fields
    async fn update_bot(&mut self, bot: &Bot) {
        assert_ne!(bot.id, BotId::UNINITIALIZED);
        const SQL: &str = indoc! {"
            UPDATE bots SET name = $1 \
            WHERE id = $2"
        };

        let res = sqlx::query(SQL)
            .bind::<&str>(&bot.name)
            .bind::<i64>(bot.id.into())
            .execute(&self.pool)
            .await
            .expect("Cannot update bot in db");

        assert_eq!(res.rows_affected(), 1);
    }

    pub async fn delete_bot(&mut self, id: BotId) {
        sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind::<i64>(id.into())
            .execute(&self.pool)
            .await
            .expect("Cannot delete bot from db");
    }

    pub async fn fetch_bots(&mut self) -> Vec<Bot> {
        sqlx::query_as::<_, BotsRow>("SELECT * from bots")
            .fetch_all(&self.pool)
            .await
            .expect("Cannot fetch all bots")
            .into_iter()
            .filter_map(|item| {
                let id = item.id;
                Bot::try_from(item)
                    .inspect_err(|e| warn!("Invalid db data (bot {}): {}. Skipping.", id, e))
                    .ok()
            })
            .collect()
    }

    pub async fn fetch_builds(&mut self) -> Vec<Build> {
        sqlx::query_as::<_, BuildsRow>("SELECT * from builds")
            .fetch_all(&self.pool)
            .await
            .expect("Cannot fetch all builds")
            .into_iter()
            .filter_map(|item| {
                let id = (item.worker_name.clone(), item.bot_id);
                Build::try_from(item)
                    .inspect_err(|e| warn!("Invalid db data (build {:?}): {}. Skipping.", id, e))
                    .ok()
            })
            .collect()
    }

    pub async fn persist_build(&mut self, build: &Build) {
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
            .execute(&self.pool)
            .await
            .expect("Cannot upsert build to db");
    }

    pub async fn persist_match(&mut self, m: &mut Match) {
        assert_eq!(m.id, MatchId::UNINITIALIZED);
        m.id = self.create_match(m).await;
    }

    pub async fn create_match(&mut self, m: &Match) -> MatchId {
        let mut tx = self.pool.begin().await.expect("cannot start a transaction");

        let match_id: MatchId =
            sqlx::query("INSERT INTO matches (seed, participant_cnt) VALUES ($1, $2)")
                .bind::<i64>(m.seed)
                .bind::<u8>(m.participants.len() as _)
                .execute(&mut *tx)
                .await
                .expect("Cannot create match in db")
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
                .await
                .expect("Cannot create participation in db");
        }

        for attr in &m.attributes {
            const SQL: &str = indoc! {
                "INSERT INTO match_attributes (name, match_id, bot_id, turn, value) \
                VALUES ($1, $2, $3, $4, $5)"
            };

            sqlx::query(SQL)
                .bind::<&str>(&attr.name)
                .bind::<i64>(match_id.into())
                .bind::<Option<i64>>(attr.bot_id.map(|id| id.into()))
                .bind::<Option<u16>>(attr.turn)
                .bind::<&str>(&attr.value)
                .execute(&mut *tx)
                .await
                .expect("Cannot create match attribute in db");
        }

        tx.commit().await.expect("cannot commit transaction");
        match_id
    }

    pub async fn fetch_matches(&mut self) -> Vec<Match> {
        let matches: Vec<MatchesRow> = sqlx::query_as("SELECT * from matches")
            .fetch_all(&self.pool)
            .await
            .expect("Cannot query matches from db");

        let participations: Vec<ParticipationsRow> = sqlx::query_as("SELECT * from participations")
            .fetch_all(&self.pool)
            .await
            .expect("Cannot query match participations from db");

        let attributes: Vec<MatchAttributesRow> = sqlx::query_as("SELECT * from match_attributes")
            .fetch_all(&self.pool)
            .await
            .expect("Cannot query match attributes from db");

        let mut combined = HashMap::with_capacity(matches.len());
        for m in matches {
            combined.insert(m.id, (m, vec![], vec![]));
        }
        for p in participations {
            combined
                .get_mut(&p.match_id)
                .expect("Participation does not match any match in db")
                .1
                .push(p);
        }
        for attr in attributes {
            combined
                .get_mut(&attr.match_id)
                .expect("Match attribute does not match any match in db")
                .2
                .push(attr);
        }

        combined
            .into_values()
            .filter_map(|item| {
                let id = item.0.id;
                Match::try_from(item)
                    .inspect_err(|e| warn!("Invalid db data (match {}): {}. Skipping.", id, e))
                    .ok()
            })
            .collect()
    }
}
