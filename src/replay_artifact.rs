use anyhow::{bail, Context};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

const REPLAY_DIRECTORY: &str = "replays";

pub fn allocate() -> PathBuf {
    PathBuf::from(REPLAY_DIRECTORY).join(format!("{}.json", Uuid::new_v4()))
}

pub fn resolve(arena_path: &Path, replay_path: &Path) -> anyhow::Result<PathBuf> {
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

pub async fn remove(arena_path: &Path, replay_path: &Path) -> anyhow::Result<()> {
    let path = resolve(arena_path, replay_path)?;
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("cannot delete {}", path.display())),
    }
}
