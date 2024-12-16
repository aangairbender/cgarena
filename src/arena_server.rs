use crate::config::Config;
use crate::db::Database;
use crate::worker::Worker;
use crate::{api, build_manager, match_result_processor, match_scheduler, matchmaking, ranking};
use itertools::Itertools;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::channel;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::info;

pub async fn start(arena_path: &Path) {
    let config = Config::load(arena_path).expect("Cannot load arena config");
    let db = Database::connect(arena_path).await;
    let server_port = config.server.port;

    let (match_result_tx, match_result_rx) = channel(16);

    let ranking = ranking::Ranking::new(Arc::new(config.ranking), db.clone());

    let token = CancellationToken::new();
    let workers = config
        .workers
        .into_iter()
        .map(|config| Worker::new(arena_path, config, match_result_tx.clone(), token.clone()))
        .collect_vec();
    assert!(
        workers.iter().map(|w| &w.name).all_unique(),
        "All worker names must be unique"
    );
    drop(match_result_tx);
    let workers: Arc<[Worker]> = workers.into();

    let tracker = TaskTracker::new();

    let worker_manager = build_manager::BuildManager::new(Arc::clone(&workers), db.clone());

    tracker.spawn(match_result_processor::run(
        match_result_rx,
        db.clone(),
        ranking,
    ));

    tracker.spawn(api::start(
        server_port,
        db.clone(),
        worker_manager,
        token.clone(),
    ));

    let (scheduled_matches_tx, scheduled_matches_rx) = channel(20);
    // let match_scheduler = match_scheduler::MatchScheduler::new(scheduled_matches_tx);
    tracker.spawn(match_scheduler::run(
        db.clone(),
        Arc::clone(&workers),
        scheduled_matches_rx,
    ));

    tracker.spawn(matchmaking::run(
        scheduled_matches_tx.clone(),
        db.clone(),
        Arc::new(config.game),
        Arc::new(config.matchmaking),
        token.clone(),
    ));

    drop(workers);
    drop(scheduled_matches_tx);

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
