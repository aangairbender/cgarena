use crate::domain::{
    Bot, BotId, BotName, BotStats, Build, BuildStatus, Match, MatchId, Participant, Rating,
};
use anyhow::{anyhow, bail};
use chrono::{DateTime, Utc};
use indoc::indoc;
use itertools::Itertools;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite, SqlitePool};
use std::path::Path;

// Represents a bot submitted to arena
#[derive(sqlx::FromRow)]
struct BotsRow {
    pub id: i64,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<BotsRow> for Bot {
    type Error = anyhow::Error;

    fn try_from(r: BotsRow) -> Result<Self, Self::Error> {
        Ok(Bot {
            id: r.id.into(),
            name: r.name.try_into()?,
            source_code: r.source_code.try_into()?,
            language: r.language.try_into()?,
            created_at: r.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct BotsStatsRow {
    pub bot_id: i64,
    pub matches_played: i64,
    pub rating_mu: f64,
    pub rating_sigma: f64,
    pub matches_with_error: i64,
}

impl From<BotsStatsRow> for BotStats {
    fn from(r: BotsStatsRow) -> Self {
        BotStats {
            matches_played: r.matches_played,
            rating: Rating {
                mu: r.rating_mu,
                sigma: r.rating_sigma,
            },
            matches_with_error: r.matches_with_error,
        }
    }
}

impl From<BotsStatsRow> for (BotId, BotStats) {
    fn from(r: BotsStatsRow) -> Self {
        (r.bot_id.into(), r.into())
    }
}

#[derive(sqlx::FromRow)]
struct MatchesWithParticipationRow {
    pub id: i64,
    pub seed: i64,
    pub bot_id: i64,
    pub index: u8,
    pub rank: u8,
    pub error: bool,
}

impl TryFrom<Vec<MatchesWithParticipationRow>> for Match {
    type Error = anyhow::Error;

    fn try_from(mut ps: Vec<MatchesWithParticipationRow>) -> Result<Self, Self::Error> {
        if ps.is_empty() {
            bail!("No participations found for match");
        }
        ps.sort_by_key(|p| p.index);

        let m = &ps[0];

        Ok(Match {
            id: m.id.into(),
            seed: m.seed,
            participants: ps
                .iter()
                .map(|p| Participant {
                    bot_id: p.bot_id.into(),
                    rank: p.rank,
                    error: p.error,
                })
                .collect(),
        })
    }
}

#[derive(sqlx::FromRow)]
pub struct BuildsRow {
    pub bot_id: i64,
    pub worker_name: String,
    pub status: u8,
    pub error: Option<String>,
}

impl TryFrom<BuildsRow> for Build {
    type Error = anyhow::Error;

    fn try_from(b: BuildsRow) -> Result<Self, Self::Error> {
        Ok(Build {
            bot_id: b.bot_id.into(),
            worker_name: b.worker_name.try_into()?,
            status: match b.status {
                0 => BuildStatus::Pending,
                1 => BuildStatus::Running,
                2 => BuildStatus::Success,
                3 => BuildStatus::Failure(
                    b.error
                        .ok_or(anyhow!("Error cannot be null for failed build"))?,
                ),
                _ => bail!("unexpected build status"),
            },
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DBError {
    #[error("Already exists")]
    AlreadyExists,
    #[error("Not found")]
    NotFound,
}

pub type DBResult<T> = Result<T, DBError>;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
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
        let pool = SqlitePoolOptions::new()
            .connect(&db_url)
            .await
            .expect("cannot connect to database");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("can't run migrations");

        Self { pool }
    }

    pub async fn create_bot(&self, bot: Bot) -> DBResult<BotId> {
        assert_eq!(bot.id, BotId::UNINITIALIZED);
        const SQL: &str = indoc! {"
            INSERT INTO bots (name, source_code, language, created_at) \
            VALUES ($1, $2, $3, $4) \
        "};

        let res = sqlx::query(SQL)
            .bind::<String>(bot.name.into())
            .bind::<String>(bot.source_code.into())
            .bind::<String>(bot.language.into())
            .bind::<DateTime<Utc>>(bot.created_at)
            .execute(&self.pool)
            .await?;

        let bot_id = BotId::from(res.last_insert_rowid());
        Ok(bot_id)
    }

    pub async fn delete_bot(&self, id: BotId) -> DBResult<()> {
        let res = sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind::<i64>(id.into())
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            Err(DBError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn rename_bot(&self, id: BotId, new_name: BotName) -> DBResult<()> {
        let res = sqlx::query("UPDATE bots SET name = $1 WHERE id = $2")
            .bind::<String>(new_name.into())
            .bind::<i64>(id.into())
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            Err(DBError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn fetch_bot_stats(&self, id: BotId) -> Option<BotStats> {
        sqlx::query_as("SELECT * FROM bot_stats WHERE bot_id = $1")
            .bind::<i64>(id.into())
            .fetch_optional(&self.pool)
            .await
            .expect("Cannot fetch bot stats from db")
            .map(BotsStatsRow::into)
    }

    pub async fn fetch_bot_stats_all(&self) -> Vec<(BotId, BotStats)> {
        sqlx::query_as("SELECT * FROM bot_stats")
            .fetch_all(&self.pool)
            .await
            .expect("Cannot fetch bot stats from db")
            .into_iter()
            .map(BotsStatsRow::into)
            .collect()
    }

    pub async fn upsert_bot_stats(&self, id: BotId, stats: BotStats) {
        const SQL: &str = indoc! {
            "INSERT OR REPLACE INTO bot_stats (bot_id, matches_played, rating_mu, rating_sigma, matches_with_error) \
            VALUES ($1, $2, $3, $4, $5)"
        };
        sqlx::query(SQL)
            .bind::<i64>(id.into())
            .bind::<i64>(stats.matches_played)
            .bind::<f64>(stats.rating.mu)
            .bind::<f64>(stats.rating.sigma)
            .bind::<i64>(stats.matches_with_error)
            .execute(&self.pool)
            .await
            .expect("Cannot upsert bot stats");
    }

    pub async fn fetch_bot(&self, id: BotId) -> Option<Bot> {
        let row: Option<BotsRow> = sqlx::query_as("SELECT * FROM bots WHERE id = $1")
            .bind::<i64>(id.into())
            .fetch_optional(&self.pool)
            .await
            .expect("Query execution failed");

        row.map(|r| r.try_into().expect("invalid bot in db"))
    }

    pub async fn fetch_bots(&self) -> Vec<Bot> {
        let bots: Vec<BotsRow> = sqlx::query_as("SELECT * from bots")
            .fetch_all(&self.pool)
            .await
            .expect("Query execution failed");

        bots.into_iter()
            .map(|r| r.try_into().expect("invalid bot in db"))
            .collect()
    }

    pub async fn upsert_build(&self, build: &Build) {
        const SQL: &str = indoc! {"
            INSERT OR REPLACE INTO builds (bot_id, worker_name, status, error) \
            VALUES ($1, $2, $3, $4) \
        "};

        let (status, error) = match &build.status {
            BuildStatus::Pending => (0, None),
            BuildStatus::Running => (1, None),
            BuildStatus::Success => (2, None),
            BuildStatus::Failure(err) => (3, Some(err.as_ref())),
        };

        sqlx::query(SQL)
            .bind::<i64>(build.bot_id.into())
            .bind::<&str>(&build.worker_name)
            .bind::<u8>(status)
            .bind::<Option<&str>>(error)
            .execute(&self.pool)
            .await
            .expect("Cannot insert build to db");
    }

    pub async fn fetch_bot_builds(&self, bot_id: BotId) -> Vec<Build> {
        let builds: Vec<BuildsRow> = sqlx::query_as("SELECT * from builds where bot_id = $1")
            .bind::<i64>(bot_id.into())
            .fetch_all(&self.pool)
            .await
            .expect("Cannot fetch builds from db");

        builds
            .into_iter()
            .map(|b| b.try_into().expect("invalid build in db"))
            .collect()
    }

    pub async fn create_match(&self, r#match: Match) -> MatchId {
        let mut tx = self.pool.begin().await.expect("cannot start a transaction");
        let match_id: MatchId = sqlx::query("INSERT INTO matches (seed) VALUES ($1)")
            .bind::<i64>(r#match.seed)
            .execute(&mut *tx)
            .await
            .expect("Cannot create match in db")
            .last_insert_rowid()
            .into();

        for (index, p) in r#match.participants.into_iter().enumerate() {
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

    pub async fn fetch_matches_with_bot(&self, bot_id: BotId) -> Vec<Match> {
        const SQL: &str = indoc! {
            "SELECT m.*, p.* FROM matches m \
            INNER JOIN participations p ON m.id = p.match_id \
            WHERE m.bot_id = $1"
        };

        let rows: Vec<MatchesWithParticipationRow> = sqlx::query_as(SQL)
            .bind::<i64>(bot_id.into())
            .fetch_all(&self.pool)
            .await
            .expect("Cannot query matches from db");

        rows.into_iter()
            .into_group_map_by(|r| r.id)
            .into_values()
            .map(|ps| ps.try_into().expect("invalid match in db"))
            .collect()
    }
}

impl From<sqlx::Error> for DBError {
    fn from(value: sqlx::Error) -> Self {
        let err = value.into_database_error().expect("Unexpected db error");
        if err.is_unique_violation() {
            DBError::AlreadyExists
        } else {
            panic!("Unexpected db error: {}", err);
        }
    }
}
