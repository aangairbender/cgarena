use crate::config::{Config, WorkerConfig};
use crate::db::Database;
use crate::embedded_worker::EmbeddedWorker;
use crate::{api, arena};
use std::path::Path;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub async fn start(arena_path: &Path) {
    let config = Config::load(arena_path).expect("Cannot load arena config");
    let db = Database::connect(arena_path).await;
    let server_port = config.server.port;
    let token = CancellationToken::new();

    let [WorkerConfig::Embedded(cfg)] = config.workers.as_slice() else {
        panic!("In the current version only single embedded worker supported");
    };
    let worker = EmbeddedWorker::new(arena_path, cfg.clone(), token.clone());

    let (arena_tx, arena_rx) = tokio::sync::mpsc::channel(16);

    tokio::spawn(arena::run(config, db, worker, arena_rx, token.clone()));
    tokio::spawn(api::start(server_port, arena_tx, token.clone()));

    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");

    token.cancel();
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
