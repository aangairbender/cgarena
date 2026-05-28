use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use anyhow::bail;
use itertools::Itertools;
use sqlx::{Pool, Sqlite};

use crate::domain::MatchId;

pub struct ReplayViewer {
    pool: Pool<Sqlite>,
    worker_path: PathBuf,
    cmd: String,
    active_replays: HashMap<MatchId, (String, Child)>,
}

impl ReplayViewer {
    pub fn new(pool: Pool<Sqlite>, worker_path: PathBuf, cmd: String) -> Self {
        Self {
            pool,
            cmd,
            worker_path,
            active_replays: Default::default(),
        }
    }

    pub async fn watch(&mut self, match_id: MatchId) -> anyhow::Result<String> {
        if self.active_replays.contains_key(&match_id) {
            return Ok(self.active_replays[&match_id].0.clone());
        }

        let seed: String = sqlx::query_scalar::<_, i64>("SELECT seed FROM matches WHERE id = ?")
            .bind::<i64>(match_id.into())
            .fetch_one(&self.pool)
            .await?
            .to_string();

        let command_parts = self
            .cmd
            .split_ascii_whitespace()
            .map(|s| match s {
                "{SEED}" => &seed,
                _ => s,
            })
            .collect_vec();

        let mut child = Command::new(&command_parts[0])
            .args(&command_parts[1..])
            .current_dir(&self.worker_path)
            .stdout(Stdio::piped())
            .spawn()?;

        let Some(stdout) = child.stdout.take() else {
            child.kill()?;
            bail!("Failed to read stdout of child process");
        };

        let mut reader = BufReader::new(stdout);
        let mut url = String::new();
        reader.read_line(&mut url)?;

        self.active_replays.insert(match_id, (url.clone(), child));

        Ok(url)
    }

    pub fn close(&mut self, match_id: MatchId) -> anyhow::Result<()> {
        let Some((_, mut child)) = self.active_replays.remove(&match_id) else {
            return Ok(());
        };

        child.kill()?;
        Ok(())
    }
}

impl Drop for ReplayViewer {
    fn drop(&mut self) {
        for (_, (_, mut child)) in self.active_replays.drain() {
            // ignoring failures on drop
            let _ = child.kill();
        }
    }
}
