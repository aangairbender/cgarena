use crate::arena_handle::ArenaHandle;
use crate::config::{Config, WorkerConfig};
use crate::{api, arena, db, worker};
use anyhow::{bail, Context};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, Level};
use tracing_subscriber::fmt::format::FmtSpan;

pub async fn start(arena_path: &Path) -> anyhow::Result<()> {
    let config = Config::load(arena_path).context("Cannot load arena config")?;

    config.validate().context("Invalid config")?;

    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(arena_path.join(config.log.file.unwrap_or("cgarena.log".to_string())))
        .context("Cannot write to cgarena.log")?;

    let log_level = config
        .log
        .level
        .and_then(|lvl| Level::from_str(&lvl).ok())
        .unwrap_or(Level::INFO);

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_writer(log_file)
        .with_ansi(false)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let pool = db::connect(arena_path)
        .await
        .context("Cannot connect to db")?;
    let token = CancellationToken::new();

    let [WorkerConfig::Embedded(cfg)] = config.workers.as_slice() else {
        bail!("In the current version only single embedded worker supported");
    };
    let worker_handle = worker::run_embedded_worker(arena_path, cfg.clone())
        .context("Cannot start embedded worker")?;

    let (arena_tx, arena_rx) = tokio::sync::mpsc::channel(16);

    let arena_task_handle = arena::run(
        config.game,
        config.matchmaking,
        config.ranking,
        pool,
        worker_handle,
        arena_rx,
        token.clone(),
    )
    .await
    .context("Cannot start arena")?;

    let exposed = config.server.expose;
    let addr = if exposed {
        SocketAddr::from(([0, 0, 0, 0], config.server.port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], config.server.port))
    };

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Port is already in use")?;

    let bind_addr = listener
        .local_addr()
        .context("Cannot get local address of tcp binding")?;

    let arena_handle = ArenaHandle::new(arena_tx);
    let api_task_handle = tokio::spawn(api::start(listener, arena_handle, token.clone()));

    info!("CG Arena started");
    println!("CG Arena started, press Ctrl+C to stop it");
    println!("Local:   http://localhost:{}/", bind_addr.port());
    if exposed {
        if let Ok(ip) = local_ip_address::local_ip() {
            println!("Network: http://{}:{}/", ip, bind_addr.port());
        }
    } else {
        println!("Network: use 'server.expose' config param to expose",);
    }
    println!(); // empty line for nicer stdout

    tokio::select! {
        _ = shutdown_signal() => {
            println!("Stopping CG Arena... press Ctrl+C again to kill it");
            token.cancel();
        },
        _ = arena_task_handle => {
            warn!("Arena task terminated unexpectedly.");
        }
        _ = api_task_handle => {
            warn!("API task terminated unexpectedly.");
        }
    }

    info!("CG Arena stopped");

    Ok(())
}

static DEFAULT_FILES: &[(&str, &str, bool)] = &[
    (
        "cgarena_config.toml",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/default_config.toml"
        )),
        true,
    ),
    (
        "build.sh",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/build.sh")),
        false,
    ),
    (
        "run.sh",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/run.sh")),
        false,
    ),
    (
        "play_game.py",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/play_game.py")),
        false,
    ),
];

pub fn init(path: &Path, clean: bool) -> anyhow::Result<()> {
    match std::fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => bail!("Cannot create new arena: {}", e),
    }
    for &(file, content, include_in_clean) in DEFAULT_FILES {
        if clean && !include_in_clean {
            continue;
        }

        let filepath = path.join(file);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filepath)
            .context(format!("Cannot create {file} file"))?
            .write_all(content.as_bytes())
            .context(format!("Cannot write to {file}"))?;
    }
    println!("New arena has been initialized in {}", path.display());
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new_arena_can_be_created_in_new_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        init(&path, false).unwrap();
        assert!(path.join("cgarena_config.toml").exists());
        assert!(path.join("build.sh").exists());
        assert!(path.join("run.sh").exists());
        assert!(path.join("play_game.py").exists());
    }

    #[test]
    fn new_arena_can_be_created_in_new_folder_clean() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        init(&path, true).unwrap();
        assert!(path.join("cgarena_config.toml").exists());
        assert!(!path.join("build.sh").exists());
        assert!(!path.join("run.sh").exists());
        assert!(!path.join("play_game.py").exists());
    }

    #[test]
    fn new_arena_can_be_created_in_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        std::fs::create_dir(&path).unwrap();
        init(&path, true).unwrap();
        assert!(path.join("cgarena_config.toml").exists());
    }
}
