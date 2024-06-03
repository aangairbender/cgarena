use std::path::Path;

use tracing::warn;

use crate::model::Bot;

static BOTS_FILE: &str = "cgarena.bots";

pub fn load_bots(arena_path: &Path) -> Result<Vec<Bot>, anyhow::Error> {
    let path = arena_path.join(BOTS_FILE);
    let json = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => {
            warn!("Can't load bots: {}", e);
            return Ok(vec![]);
        }
    };
    let bots: Vec<Bot> = serde_json::from_str(&json)?;
    Ok(bots)
}

pub fn save_bots(arena_path: &Path, bots: &[Bot]) -> Result<(), anyhow::Error> {
    let path = arena_path.join(BOTS_FILE);
    let json = serde_json::to_string(bots)?;
    std::fs::write(path, json)?;
    Ok(())
}
