use crate::config::Config;
use crate::db::Database;
use crate::{api, worker_manager};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::info;

pub async fn start(arena_path: &Path) {
    let config = Config::load(arena_path).expect("Cannot load arena config");
    let db = Database::connect(arena_path).await;
    let server_port = config.server.port;

    let tracker = TaskTracker::new();
    let token = CancellationToken::new();

    let worker_manager = worker_manager::WorkerManager::new(arena_path, config.workers, db.clone());
    // building existing bots, TODO: move it somewhere
    let bots = db.clone().fetch_bots().await;
    for bot in bots {
        worker_manager.ensure_built(bot.id).await;
    }

    tracker.spawn(api::start(
        server_port,
        db.clone(),
        worker_manager,
        token.clone(),
    ));

    // let match_manager =
    //     match_manager::MatchManager::new(db, config.game, config.matchmaking, match_sender);
    // tracker.spawn(match_manager.run(token.clone()));

    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracker.close();
    token.cancel();
    tracker.wait().await;
}

pub fn init(path: &Path) {
    match std::fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => panic!("Cannot create new arena: {}", e),
    }
    Config::create_default(path);
    info!("New arena has been initialized in {}", path.display());
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_arena_can_be_created_in_new_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        init(&path);
        assert!(path.join("cgarena_config.toml").exists());
    }

    #[test]
    fn new_arena_can_be_created_in_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        std::fs::create_dir(&path).unwrap();
        init(&path);
        assert!(path.join("cgarena_config.toml").exists());
    }
}
