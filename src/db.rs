use crate::domain::{Bot, BotId, Build, BuildResult, BuildStatus, Match, MatchId, Participant};
use anyhow::bail;
use chrono::{DateTime, Utc};
use indoc::indoc;
use itertools::Itertools;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{migrate::MigrateDatabase, ConnectOptions, Connection, Sqlite, SqliteConnection};
use std::collections::HashMap;
use std::path::Path;

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

impl TryFrom<(MatchesRow, Vec<ParticipationsRow>)> for Match {
    type Error = anyhow::Error;

    fn try_from((m, mut ps): (MatchesRow, Vec<ParticipationsRow>)) -> Result<Self, Self::Error> {
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
        })
    }
}

pub struct Database {
    conn: SqliteConnection,
}

const DB_FILE_NAME: &str = "cgarena.db";

impl Database {
    pub async fn connect(arena_path: &Path) -> Self {
        let db_path = arena_path.join(DB_FILE_NAME);
        let db_url = format!("sqlite://{}", db_path.display());

        if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
            Sqlite::create_database(&db_url)
                .await
                .expect("cannot create database");
        }
        let mut conn = SqliteConnectOptions::new()
            .filename(&db_path)
            .connect()
            .await
            .expect("cannot connect to database");

        sqlx::migrate!()
            .run(&mut conn)
            .await
            .expect("can't run migrations");

        Self { conn }
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
            .execute(&mut self.conn)
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
            .execute(&mut self.conn)
            .await
            .expect("Cannot update bot in db");

        assert_eq!(res.rows_affected(), 1);
    }

    pub async fn delete_bot(&mut self, id: BotId) {
        sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind::<i64>(id.into())
            .execute(&mut self.conn)
            .await
            .expect("Cannot delete bot from db");
    }

    pub async fn fetch_bots(&mut self) -> Vec<Bot> {
        sqlx::query_as::<_, BotsRow>("SELECT * from bots")
            .fetch_all(&mut self.conn)
            .await
            .expect("Cannot fetch all bots")
            .into_iter()
            .map(Bot::try_from)
            .try_collect()
            .expect("Error during mapping db to domain")
    }

    pub async fn fetch_builds(&mut self) -> Vec<Build> {
        sqlx::query_as::<_, BuildsRow>("SELECT * from builds")
            .fetch_all(&mut self.conn)
            .await
            .expect("Cannot fetch all builds")
            .into_iter()
            .map(Build::try_from)
            .try_collect()
            .expect("Error during mapping db to domain")
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
            .execute(&mut self.conn)
            .await
            .expect("Cannot upsert build to db");
    }

    pub async fn persist_match(&mut self, m: &mut Match) {
        assert_eq!(m.id, MatchId::UNINITIALIZED);
        m.id = self.create_match(m).await;
    }

    pub async fn create_match(&mut self, m: &Match) -> MatchId {
        let mut tx = self.conn.begin().await.expect("cannot start a transaction");

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

        tx.commit().await.expect("cannot commit transaction");
        match_id
    }

    pub async fn fetch_matches(&mut self) -> Vec<Match> {
        let matches: Vec<MatchesRow> = sqlx::query_as("SELECT * from matches")
            .fetch_all(&mut self.conn)
            .await
            .expect("Cannot query matches from db");

        let participations: Vec<ParticipationsRow> = sqlx::query_as("SELECT * from participations")
            .fetch_all(&mut self.conn)
            .await
            .expect("Cannot query matches from db");

        let mut combined = HashMap::with_capacity(matches.len());
        for m in matches {
            combined.insert(m.id, (m, vec![]));
        }
        for p in participations {
            combined
                .get_mut(&p.match_id)
                .expect("Participation does not match any match in db")
                .1
                .push(p);
        }

        combined
            .into_values()
            .map(Match::try_from)
            .try_collect()
            .expect("Error during mapping db to domain")
    }
}
