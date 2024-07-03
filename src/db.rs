use crate::model::Bot;
use indoc::indoc;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePoolOptions, Sqlite, SqlitePool};

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
}

impl Database {
    pub async fn new(db_url: &str) -> Self {
        if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
            Sqlite::create_database(db_url)
                .await
                .expect("cannot create database");
        }
        let pool = SqlitePoolOptions::new()
            .connect(db_url)
            .await
            .expect("cannot cannot to database");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("can't run migrations");
        Self { pool }
    }

    pub async fn add_bot(&self, bot: Bot) -> DBResult<()> {
        const SQL: &str = indoc! {"
            INSERT INTO bots (name, source_code, language, rating_mu, rating_sigma, created_at) \
            VALUES ($1, $2, $3, $4, $5, $6, $7) \
        "};

        sqlx::query(SQL)
            .bind(bot.name)
            .bind(bot.source_code)
            .bind(bot.language)
            .bind(bot.rating.mu)
            .bind(bot.rating.sigma)
            .bind(bot.created_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn remove_bot(&self, name: String) -> DBResult<()> {
        let res = sqlx::query("SELECT id FROM bots WHERE name = $1")
            .bind(&name)
            .fetch_optional(&self.pool)
            .await?;

        if res.is_none() {
            return Err(DBError::NotFound);
        };

        sqlx::query("DELETE FROM bots WHERE name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn rename_bot(&self, old_name: String, new_name: String) -> DBResult<()> {
        let res = sqlx::query("SELECT id FROM bots WHERE name = $1")
            .bind(&old_name)
            .fetch_optional(&self.pool)
            .await?;

        if res.is_none() {
            return Err(DBError::NotFound);
        };

        sqlx::query("UPDATE bots SET name = $1 WHERE name = $2")
            .bind(new_name)
            .bind(old_name)
            .execute(&self.pool)
            .await?;

        Ok(())
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
