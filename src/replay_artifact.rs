use crate::{
    db,
    domain::{BotId, Match, MatchId},
};
use anyhow::{bail, Context};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::path::{Component, Path, PathBuf};
use tracing::warn;
use uuid::Uuid;

const REPLAY_DIRECTORY: &str = "replays";
const DB_FILE_NAME: &str = "cgarena.db";

/// Owns replay artifacts from match persistence through lookup and deletion.
///
/// The lifecycle has three states:
/// - [`ProvisionalReplay`] owns a unique path reserved for a running match command.
/// - [`PendingReplay`] owns a non-empty file that is not yet referenced by a match row.
/// - a persisted match row owns its replay path after [`ReplayArtifacts::persist_match`] commits.
///
/// Provisional and pending owners compensate by deleting their file when they are rejected,
/// dropped, or fail to transition. Persisted artifacts are validated on lookup and are deleted
/// before the database operation that removes their owning match.
#[derive(Clone)]
pub struct ReplayArtifacts {
    pool: SqlitePool,
    arena_path: PathBuf,
}

impl ReplayArtifacts {
    pub fn new(pool: SqlitePool, arena_path: PathBuf) -> Self {
        Self { pool, arena_path }
    }

    /// Atomically transfers an optional pending artifact to a newly persisted match.
    /// A failed database write leaves the artifact pending, so its owner removes it.
    pub async fn persist_match(
        &self,
        replay: PendingReplay,
        new_match: &mut Match,
    ) -> anyhow::Result<()> {
        let artifact = replay.artifact;
        new_match.replay_path = artifact
            .as_ref()
            .map(|artifact| artifact.relative_path.clone());

        db::persist_match(&self.pool, new_match).await?;
        if let Some(artifact) = artifact {
            artifact.commit();
        }
        Ok(())
    }

    /// Resolves a persisted artifact only after its row, path, and file are validated.
    pub async fn lookup(&self, match_id: MatchId) -> Result<ReadableReplay, ReplayLookupError> {
        let row = sqlx::query_as::<_, (Option<String>, u8)>(
            "SELECT replay_path, participant_cnt FROM matches WHERE id = ?",
        )
        .bind::<i64>(match_id.into())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| ReplayLookupError::Internal(error.to_string()))?
        .ok_or(ReplayLookupError::MatchNotFound)?;

        let relative_path = row.0.ok_or(ReplayLookupError::Unavailable)?;
        let path = resolve(&self.arena_path, Path::new(&relative_path))
            .map_err(|error| ReplayLookupError::InvalidArtifact(error.to_string()))?;
        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| ReplayLookupError::InvalidArtifact(error.to_string()))?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(ReplayLookupError::InvalidArtifact(
                "artifact is not a non-empty file".to_string(),
            ));
        }

        Ok(ReadableReplay {
            path,
            participant_count: row.1,
        })
    }

    /// Deletes every match artifact owned through a bot before deleting the bot and its matches.
    pub async fn delete_bot(&self, bot_id: BotId) -> anyhow::Result<()> {
        let paths = sqlx::query_scalar::<_, String>(
            "SELECT m.replay_path
             FROM matches m
             INNER JOIN participations p ON p.match_id = m.id
             WHERE p.bot_id = ? AND m.replay_path IS NOT NULL",
        )
        .bind::<i64>(bot_id.into())
        .fetch_all(&self.pool)
        .await?;

        self.remove_all(paths).await?;
        db::delete_bot(&self.pool, bot_id).await
    }

    async fn delete_old_matches<F: Fn(usize) -> bool>(
        &self,
        percentage: u8,
        vacuum: bool,
        confirm: F,
    ) -> anyhow::Result<usize> {
        let percentage = percentage.clamp(0, 100);
        let mut match_ids: Vec<(i64,)> = sqlx::query_as("SELECT id FROM matches")
            .fetch_all(&self.pool)
            .await?;
        match_ids.sort();

        let amount_to_delete = match_ids.len() * usize::from(percentage) / 100;
        if amount_to_delete == 0 {
            println!("0 matches to delete, skipping");
        } else {
            if !confirm(amount_to_delete) {
                bail!("Cancelled by the user");
            }

            let last_deleted_id = match_ids[amount_to_delete - 1].0;
            let paths = sqlx::query_scalar::<_, String>(
                "SELECT replay_path FROM matches WHERE id <= ? AND replay_path IS NOT NULL",
            )
            .bind(last_deleted_id)
            .fetch_all(&self.pool)
            .await?;

            println!("Deleting {amount_to_delete} old matches");
            self.remove_all(paths).await?;
            sqlx::query("DELETE FROM matches WHERE id <= ?")
                .bind(last_deleted_id)
                .execute(&self.pool)
                .await?;
        }

        if vacuum {
            println!("Vacuuming the db");
            sqlx::query("VACUUM").execute(&self.pool).await?;
        }

        Ok(amount_to_delete)
    }

    async fn remove_all(&self, paths: Vec<String>) -> anyhow::Result<()> {
        for path in paths {
            remove(&self.arena_path, Path::new(&path)).await?;
        }
        Ok(())
    }
}

/// Owns the unique replay path offered to one running match command.
#[derive(Debug)]
pub struct ProvisionalReplay {
    artifact: Option<OwnedReplay>,
}

impl ProvisionalReplay {
    pub async fn create(arena_path: &Path) -> anyhow::Result<Self> {
        let relative_path =
            PathBuf::from(REPLAY_DIRECTORY).join(format!("{}.json", Uuid::new_v4()));
        let absolute_path = resolve(arena_path, &relative_path)?;
        let parent = absolute_path
            .parent()
            .context("replay artifact must have a parent directory")?;
        tokio::fs::create_dir_all(parent)
            .await
            .context("cannot create replay directory")?;

        Ok(Self {
            artifact: Some(OwnedReplay::new(arena_path, relative_path)?),
        })
    }

    pub fn command_path(&self) -> &Path {
        &self
            .artifact
            .as_ref()
            .expect("provisional replay ownership cannot be reused")
            .relative_path
    }

    /// Accepts only a non-empty regular file and transfers ownership to the pending state.
    pub async fn finish(mut self) -> anyhow::Result<PendingReplay> {
        let artifact = self
            .artifact
            .as_ref()
            .expect("provisional replay ownership cannot be reused");
        match tokio::fs::metadata(&artifact.absolute_path).await {
            Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(PendingReplay {
                artifact: self.artifact.take(),
            }),
            Ok(_) => {
                warn!("match command produced an empty replay artifact");
                self.remove().await?;
                Ok(PendingReplay::default())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                warn!("match command produced no replay artifact");
                self.remove().await?;
                Ok(PendingReplay::default())
            }
            Err(error) => Err(error).context("cannot inspect replay artifact"),
        }
    }

    async fn remove(&mut self) -> anyhow::Result<()> {
        if let Some(artifact) = self.artifact.take() {
            artifact.remove().await?;
        }
        Ok(())
    }
}

/// Owns a produced artifact until match persistence accepts it.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PendingReplay {
    artifact: Option<OwnedReplay>,
}

#[derive(Debug, PartialEq, Eq)]
struct OwnedReplay {
    relative_path: PathBuf,
    absolute_path: PathBuf,
    remove_on_drop: bool,
}

impl OwnedReplay {
    fn new(arena_path: &Path, relative_path: PathBuf) -> anyhow::Result<Self> {
        let absolute_path = resolve(arena_path, &relative_path)?;
        Ok(Self {
            relative_path,
            absolute_path,
            remove_on_drop: true,
        })
    }

    async fn remove(mut self) -> anyhow::Result<()> {
        remove_absolute(&self.absolute_path).await?;
        self.remove_on_drop = false;
        Ok(())
    }

    fn commit(mut self) {
        self.remove_on_drop = false;
    }
}

impl Drop for OwnedReplay {
    fn drop(&mut self) {
        if !self.remove_on_drop {
            return;
        }

        match std::fs::remove_file(&self.absolute_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => warn!(
                "cannot compensate replay artifact {}: {error}",
                self.absolute_path.display()
            ),
        }
    }
}

pub struct ReadableReplay {
    path: PathBuf,
    participant_count: u8,
}

impl ReadableReplay {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn participant_count(&self) -> u8 {
        self.participant_count
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayLookupError {
    #[error("match not found")]
    MatchNotFound,
    #[error("this match has no replay artifact")]
    Unavailable,
    #[error("replay artifact is missing or invalid: {0}")]
    InvalidArtifact(String),
    #[error("replay artifact lookup failed: {0}")]
    Internal(String),
}

pub async fn wipe_old_matches<F: Fn(usize) -> bool>(
    arena_path: &Path,
    percentage: u8,
    vacuum: bool,
    confirm: F,
) -> anyhow::Result<usize> {
    let options = SqliteConnectOptions::new()
        .filename(arena_path.join(DB_FILE_NAME))
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Delete)
        .create_if_missing(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    sqlx::migrate!().run(&pool).await?;

    ReplayArtifacts::new(pool, arena_path.to_owned())
        .delete_old_matches(percentage, vacuum, confirm)
        .await
}

fn resolve(arena_path: &Path, replay_path: &Path) -> anyhow::Result<PathBuf> {
    if replay_path.is_absolute() {
        bail!("replay path must be relative to the arena");
    }

    let mut components = replay_path.components();
    if components.next() != Some(Component::Normal(REPLAY_DIRECTORY.as_ref()))
        || components.any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("replay path must be inside the replay directory");
    }

    Ok(arena_path.join(replay_path))
}

async fn remove(arena_path: &Path, replay_path: &Path) -> anyhow::Result<()> {
    let path = resolve(arena_path, replay_path)?;
    remove_absolute(&path).await
}

async fn remove_absolute(path: &Path) -> anyhow::Result<()> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("cannot delete {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Match, Participant};
    use tempfile::TempDir;

    async fn fixture() -> (TempDir, ReplayArtifacts) {
        let arena = tempfile::tempdir().unwrap();
        let pool = db::in_memory().await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let artifacts = ReplayArtifacts::new(pool, arena.path().to_owned());
        (arena, artifacts)
    }

    async fn insert_bot(artifacts: &ReplayArtifacts, name: &str) -> BotId {
        sqlx::query(
            "INSERT INTO bots (name, source_code, language, created_at) VALUES (?, '', 'rust', 0)",
        )
        .bind(name)
        .execute(&artifacts.pool)
        .await
        .unwrap()
        .last_insert_rowid()
        .into()
    }

    fn new_match(bot_ids: &[BotId], replay_path: Option<PathBuf>) -> Match {
        Match::new(
            7,
            bot_ids
                .iter()
                .enumerate()
                .map(|(rank, bot_id)| Participant {
                    bot_id: *bot_id,
                    rank: rank as u8,
                    error: false,
                })
                .collect(),
            vec![],
            replay_path,
        )
    }

    async fn produced_replay(arena: &TempDir, contents: &[u8]) -> (PendingReplay, PathBuf) {
        let provisional = ProvisionalReplay::create(arena.path()).await.unwrap();
        let path = provisional.command_path().to_owned();
        tokio::fs::write(arena.path().join(&path), contents)
            .await
            .unwrap();
        (provisional.finish().await.unwrap(), path)
    }

    #[tokio::test]
    async fn successful_persistence_transfers_ownership_and_lookup_validates_the_artifact() {
        let (arena, artifacts) = fixture().await;
        let bot_1 = insert_bot(&artifacts, "one").await;
        let bot_2 = insert_bot(&artifacts, "two").await;
        let (pending, relative_path) = produced_replay(&arena, b"replay").await;
        let mut new_match = new_match(&[bot_1, bot_2], None);

        artifacts
            .persist_match(pending, &mut new_match)
            .await
            .unwrap();
        let readable = artifacts.lookup(new_match.id).await.unwrap();

        assert_eq!(readable.path(), arena.path().join(&relative_path));
        assert_eq!(readable.participant_count(), 2);
        assert!(readable.path().exists());
    }

    #[tokio::test]
    async fn rejected_output_or_deleted_participant_discards_the_pending_artifact() {
        let (arena, _artifacts) = fixture().await;
        let (pending, relative_path) = produced_replay(&arena, b"replay").await;

        drop(pending);

        assert!(!arena.path().join(relative_path).exists());
    }

    #[tokio::test]
    async fn persistence_failure_compensates_the_pending_artifact() {
        let (arena, artifacts) = fixture().await;
        let (pending, relative_path) = produced_replay(&arena, b"replay").await;
        let mut new_match = new_match(&[BotId::from(404)], None);

        assert!(artifacts
            .persist_match(pending, &mut new_match)
            .await
            .is_err());
        assert!(!arena.path().join(relative_path).exists());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM matches")
            .fetch_one(&artifacts.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn bot_deletion_removes_artifacts_owned_by_deleted_matches() {
        let (arena, artifacts) = fixture().await;
        let bot_1 = insert_bot(&artifacts, "one").await;
        let bot_2 = insert_bot(&artifacts, "two").await;
        let (pending, relative_path) = produced_replay(&arena, b"replay").await;
        let mut new_match = new_match(&[bot_1, bot_2], None);
        artifacts
            .persist_match(pending, &mut new_match)
            .await
            .unwrap();

        artifacts.delete_bot(bot_1).await.unwrap();

        assert!(!arena.path().join(relative_path).exists());
        assert!(matches!(
            artifacts.lookup(new_match.id).await,
            Err(ReplayLookupError::MatchNotFound)
        ));
    }

    #[tokio::test]
    async fn retention_deletes_only_artifacts_owned_by_removed_matches() {
        let (arena, artifacts) = fixture().await;
        let bot_1 = insert_bot(&artifacts, "one").await;
        let bot_2 = insert_bot(&artifacts, "two").await;
        let (old_pending, old_path) = produced_replay(&arena, b"old").await;
        let mut old_match = new_match(&[bot_1, bot_2], None);
        artifacts
            .persist_match(old_pending, &mut old_match)
            .await
            .unwrap();
        let (new_pending, new_path) = produced_replay(&arena, b"new").await;
        let mut new_match = new_match(&[bot_1, bot_2], None);
        artifacts
            .persist_match(new_pending, &mut new_match)
            .await
            .unwrap();

        assert_eq!(
            artifacts
                .delete_old_matches(50, false, |_| true)
                .await
                .unwrap(),
            1
        );

        assert!(!arena.path().join(old_path).exists());
        assert!(arena.path().join(new_path).exists());
        assert!(matches!(
            artifacts.lookup(old_match.id).await,
            Err(ReplayLookupError::MatchNotFound)
        ));
        artifacts.lookup(new_match.id).await.unwrap();
    }

    #[tokio::test]
    async fn missing_empty_and_legacy_artifacts_have_distinct_lookup_results() {
        let (arena, artifacts) = fixture().await;
        let missing_id: MatchId = sqlx::query(
            "INSERT INTO matches (seed, participant_cnt, replay_path) VALUES (1, 2, 'replays/missing.json')",
        )
        .execute(&artifacts.pool)
        .await
        .unwrap()
        .last_insert_rowid()
        .into();
        tokio::fs::create_dir_all(arena.path().join(REPLAY_DIRECTORY))
            .await
            .unwrap();
        tokio::fs::write(arena.path().join("replays/empty.json"), b"")
            .await
            .unwrap();
        let empty_id: MatchId = sqlx::query(
            "INSERT INTO matches (seed, participant_cnt, replay_path) VALUES (2, 2, 'replays/empty.json')",
        )
        .execute(&artifacts.pool)
        .await
        .unwrap()
        .last_insert_rowid()
        .into();
        let legacy_id: MatchId =
            sqlx::query("INSERT INTO matches (seed, participant_cnt) VALUES (3, 2)")
                .execute(&artifacts.pool)
                .await
                .unwrap()
                .last_insert_rowid()
                .into();

        assert!(matches!(
            artifacts.lookup(missing_id).await,
            Err(ReplayLookupError::InvalidArtifact(_))
        ));
        assert!(matches!(
            artifacts.lookup(empty_id).await,
            Err(ReplayLookupError::InvalidArtifact(_))
        ));
        assert!(matches!(
            artifacts.lookup(legacy_id).await,
            Err(ReplayLookupError::Unavailable)
        ));
    }

    #[tokio::test]
    async fn empty_provisional_artifact_is_removed_before_becoming_unavailable() {
        let (arena, _artifacts) = fixture().await;
        let provisional = ProvisionalReplay::create(arena.path()).await.unwrap();
        let relative_path = provisional.command_path().to_owned();
        tokio::fs::write(arena.path().join(&relative_path), b"")
            .await
            .unwrap();

        let pending = provisional.finish().await.unwrap();

        assert_eq!(pending, PendingReplay::default());
        assert!(!arena.path().join(relative_path).exists());
    }
}
