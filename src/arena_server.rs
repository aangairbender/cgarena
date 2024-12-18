use crate::config::{Config, WorkerConfig};
use crate::db::Database;
use crate::embedded_worker::EmbeddedWorker;
use crate::ranking::Ranker;
use crate::{api, arena};
use std::fs::OpenOptions;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use tokio_util::sync::CancellationToken;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

pub async fn start(arena_path: &Path) {
    let config = Config::load(arena_path).expect("Cannot load arena config");

    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(arena_path.join(config.log.file.unwrap_or("cgarena.log".to_string())))
        .expect("Cannot write to cgarena.log");

    tracing_subscriber::fmt()
        .with_max_level(
            config
                .log
                .level
                .and_then(|lvl| Level::from_str(&lvl).ok())
                .unwrap_or(Level::INFO),
        )
        .with_writer(log_file)
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let db = Database::connect(arena_path).await;
    let ranker = Ranker::new(config.ranking);
    let token = CancellationToken::new();

    let [WorkerConfig::Embedded(cfg)] = config.workers.as_slice() else {
        panic!("In the current version only single embedded worker supported");
    };
    let worker = EmbeddedWorker::new(arena_path, cfg.clone(), token.clone());

    let (arena_tx, arena_rx) = tokio::sync::mpsc::channel(16);

    tokio::spawn(arena::run(
        config.game,
        config.matchmaking,
        ranker,
        db,
        worker,
        arena_rx,
        token.clone(),
    ));

    let exposed = config.server.expose;
    let addr = if exposed {
        SocketAddr::from(([0, 0, 0, 0], config.server.port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], config.server.port))
    };

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Cannot bind tcp listener to the target address");

    let bind_addr = listener
        .local_addr()
        .expect("Cannot get local address of tcp binding");

    tokio::spawn(api::start(listener, arena_tx, token.clone()));

    println!("CG Arena started, press Ctrl+C to stop it");
    println!("Local:   http://localhost:{}/", bind_addr.port());
    if exposed {
        if let Ok(ip) = local_ip_address::local_ip() {
            println!("Network: http://{}:{}/", ip, bind_addr.port());
        }
    } else {
        println!("Network: use 'server.expose' config param to expose",);
    }

    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");

    println!("Stopping CG Arena... press Ctrl+C again to kill it");

    token.cancel();
}

pub fn init(path: &Path) {
    match std::fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => panic!("Cannot create new arena: {}", e),
    }
    Config::create_default(path);
    println!("New arena has been initialized in {}", path.display());
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
