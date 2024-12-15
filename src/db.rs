use crate::domain::{BotId, DomainEvent};
use crate::worker::BuildOutput;
use chrono::{DateTime, Utc};
use indoc::indoc;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite, SqlitePool};
use std::path::Path;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

// Represents a bot submitted to arena
#[derive(sqlx::FromRow)]
pub struct Bot {
    pub id: i32,
    pub name: String,
    pub source_code: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct BotStats {
    pub bot_id: i32,
    pub games: u32,
    pub rating_mu: f64,
    pub rating_sigma: f64,
}

/// Represents finished match
/// This should not be created until match result is known
#[derive(sqlx::FromRow)]
pub struct Match {
    pub id: i32,
    pub seed: i32,
}

#[derive(sqlx::FromRow)]
pub struct Participation {
    pub match_id: i32,
    pub bot_id: i32,
    pub rank: usize,
    pub error: bool,
}

#[derive(sqlx::FromRow)]
pub struct Build {
    pub bot_id: i32,
    pub worker_name: String,
    pub status_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(thiserror::Error, Debug)]
pub enum DBError {
    #[error("Already exists")]
    AlreadyExists,
    #[error("Not found")]
    NotFound,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

pub type DBResult<T> = Result<T, DBError>;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
    sender: Sender<DomainEvent>,
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

        let (tx, _rx) = broadcast::channel(16);

        Self { pool, sender: tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }

    pub async fn create_bot(&self, name: &str, source_code: &str, language: &str) -> DBResult<()> {
        const SQL: &str = indoc! {"
            INSERT INTO bots (name, source_code, language, created_at) \
            VALUES ($1, $2, $3, $4) \
        "};

        let res = sqlx::query(SQL)
            .bind(name)
            .bind(source_code)
            .bind(language)
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;

        // ignoring failure if no active receivers
        let _ = self
            .sender
            .send(DomainEvent::BotCreated(BotId(res.last_insert_rowid() as _)));

        Ok(())
    }

    pub async fn remove_bot(&self, id: i32) -> DBResult<()> {
        let res = sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            Err(DBError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn rename_bot(&self, id: i32, new_name: String) -> DBResult<()> {
        let res = sqlx::query("UPDATE bots SET name = $1 WHERE id = $2")
            .bind(new_name)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if res.rows_affected() == 0 {
            Err(DBError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn fetch_bot(&self, id: BotId) -> Option<Bot> {
        sqlx::query_as("SELECT * FROM bots WHERE id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .expect("Query execution failed")
    }

    pub async fn fetch_bots(&self) -> DBResult<Vec<Bot>> {
        let res = sqlx::query_as("SELECT * from bots")
            .fetch_all(&self.pool)
            .await?;
        Ok(res)
    }

    pub async fn fetch_bot_stats(&self) -> DBResult<Vec<BotStats>> {
        let res = sqlx::query_as("SELECT * from bot_stats")
            .fetch_all(&self.pool)
            .await?;
        Ok(res)
    }

    pub async fn insert_build(&self, bot_id: BotId, worker_name: &str, build_output: BuildOutput) {
        const SQL: &str = indoc! {"
            INSERT OR REPLACE INTO builds (bot_id, worker_name, status_code, stdout, stderr, created_at) \
            VALUES ($1, $2, $3, $4, $5, $6) \
        "};

        sqlx::query(SQL)
            .bind(bot_id.0)
            .bind(worker_name)
            .bind(build_output.status_code)
            .bind(build_output.stdout)
            .bind(build_output.stderr)
            .bind(Utc::now())
            .execute(&self.pool)
            .await
            .expect("Cannot insert build to db");
    }

    pub async fn fetch_builds(&self, bot_id: BotId) -> Vec<Build> {
        sqlx::query_as("SELECT * from builds where bot_id = $1")
            .bind(bot_id.0)
            .fetch_all(&self.pool)
            .await
            .expect("Cannot fetch builds from db")
    }
}

impl From<sqlx::Error> for DBError {
    fn from(value: sqlx::Error) -> Self {
        if value.as_database_error().is_some() {
            let err = value.into_database_error().unwrap();
            if err.is_unique_violation() {
                DBError::AlreadyExists
            } else {
                DBError::Unexpected(anyhow::Error::from(err))
            }
        } else {
            DBError::Unexpected(anyhow::Error::from(value))
        }
    }
}
