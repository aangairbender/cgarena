use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Pool, Row, Sqlite};
use uuid::Uuid;

use super::Language;

#[derive(Serialize, Deserialize)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub source_filename: String,
    pub language: Language,
    pub status: BotStatus,
    pub build_output: String,
}

impl Bot {
    pub fn new(name: String, source_filename: String, language: Language) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            source_filename,
            language,
            status: BotStatus::Pending,
            build_output: String::new(),
        }
    }

    pub async fn save(&self, pool: Pool<Sqlite>) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR REPLACE INTO bots (id, name, source_filename, language, status, build_output) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(serde_json::to_string(&self.id).unwrap())
            .bind(&self.name)
            .bind(&self.source_filename)
            .bind(serde_json::to_string(&self.language).unwrap())
            .bind(serde_json::to_string(&self.status).unwrap())
            .bind(&self.build_output)
            .execute(&pool)
            .await?;
        Ok(())
    }

    pub async fn find_by_id(id: &Uuid, pool: Pool<Sqlite>) -> Result<Option<Bot>, sqlx::Error> {
        let res = sqlx::query("SELECT * FROM bots where id = ?")
            .bind(serde_json::to_string(&id).unwrap())
            .map(|row: SqliteRow| Bot {
                id: serde_json::from_str(row.get("id")).expect("id should be valid uuid"),
                name: row.get("name"),
                source_filename: row.get("source_filename"),
                language: serde_json::from_str(row.get("language"))
                    .expect("language must be valid"),
                status: serde_json::from_str(row.get("status")).expect("status must be valid"),
                build_output: row.get("build_output"),
            })
            .fetch_optional(&pool)
            .await?;
        Ok(res)
    }

    pub async fn delete(&self, pool: Pool<Sqlite>) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM bots WHERE id = ?")
            .bind(serde_json::to_string(&self.id).unwrap())
            .execute(&pool)
            .await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub enum BotStatus {
    Pending,
    Building,
    Ready,
}
