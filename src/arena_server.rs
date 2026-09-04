use crate::arena_handle::ArenaHandle;
use crate::cg_referee::CgReferee;
use crate::config::{Config, WorkerConfig};
use crate::replay_viewer::ReplayViewer;
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

    if let Some(ref git_url) = config.game.referee_git_url {
        if !git_url.is_empty() {
            let cg_referee = CgReferee::new(git_url.clone(), arena_path.join("referee"));
            cg_referee.ensure_initialized()?;
        }
    }

    let pool = db::connect(arena_path)
        .await
        .context("Cannot connect to db")?;
    let token = CancellationToken::new();
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

    let [WorkerConfig::Embedded(cfg)] = config.workers.as_slice() else {
        bail!("In the current version only single embedded worker supported");
    };
    let worker::StartedWorker {
        worker,
        supervisor: worker_supervisor,
    } = worker::start_embedded_worker(arena_path, cfg.clone())
        .context("Cannot start embedded worker")?;

    let replay_viewer = ReplayViewer::new(
        pool.clone(),
        arena_path.to_owned(),
        cfg.referee.clone(),
        token.clone(),
    );

    let (arena_tx, arena_rx) = tokio::sync::mpsc::channel(16);

    let mut arena_task_handle = match arena::run(
        config.game,
        config.matchmaking,
        config.leaderboards,
        config.ranking,
        pool,
        arena_path.to_owned(),
        worker,
        arena_rx,
        token.clone(),
    )
    .await
    {
        Ok(task) => task,
        Err(error) => {
            token.cancel();
            let worker_result = worker_supervisor.shutdown().await;
            replay_viewer.shutdown().await;
            if let Err(failure) = worker_result {
                return Err(failure).context("Worker cleanup failed during arena startup");
            }
            return Err(error).context("Cannot start arena");
        }
    };

    let arena_handle = ArenaHandle::new(arena_tx);
    let mut api_task_handle = tokio::spawn(api::start(
        listener,
        arena_handle,
        replay_viewer.clone(),
        token.clone(),
    ));

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
    println!();

    let mut arena_result = None;
    let mut api_result = None;
    let mut observed_worker_failure = None;
    tokio::select! {
        _ = shutdown_signal() => {
            println!("Stopping CG Arena... press Ctrl+C again to kill it");
        },
        failure = worker_supervisor.failed() => {
            warn!("Embedded worker failed: {failure}");
            observed_worker_failure = Some(failure);
        }
        result = &mut arena_task_handle => {
            warn!("Arena task terminated unexpectedly.");
            arena_result = Some(result);
        }
        result = &mut api_task_handle => {
            warn!("API task terminated unexpectedly.");
            api_result = Some(result);
        }
    }

    token.cancel();
    let worker_result = worker_supervisor.shutdown().await;
    replay_viewer.shutdown().await;
    let arena_result = match arena_result {
        Some(result) => result,
        None => arena_task_handle.await,
    };
    let api_result = match api_result {
        Some(result) => result,
        None => api_task_handle.await,
    };

    info!("CG Arena stopped");

    if let Err(failure) = worker_result {
        return Err(failure.into());
    }
    if let Some(failure) = observed_worker_failure {
        return Err(failure.into());
    }
    match arena_result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error).context("Arena task failed"),
        Err(error) => return Err(error).context("Arena task terminated unexpectedly"),
    }
    if let Err(error) = api_result {
        return Err(error).context("API task terminated unexpectedly");
    }

    Ok(())
}

static DEFAULT_FILES: &[(&str, &str)] = &[
    (
        "cgarena_config.toml",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/default_config.toml"
        )),
    ),
    (
        "CommandLineInterface.java",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/CommandLineInterface.java"
        )),
    ),
    (
        "pom_build_section.xml",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/pom_build_section.xml"
        )),
    ),
];

pub fn init(path: &Path) -> anyhow::Result<()> {
    match std::fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => bail!("Cannot create new arena: {}", e),
    }
    for &(file, content) in DEFAULT_FILES {
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
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
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
        init(&path).unwrap();
        assert!(path.join("cgarena_config.toml").exists());
        assert!(!path.join("play_game.py").exists());
        assert!(!path.join("watch_replay.py").exists());
        assert!(path.join("CommandLineInterface.java").exists());
        assert!(path.join("pom_build_section.xml").exists());
    }

    #[test]
    fn new_arena_can_be_created_in_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        std::fs::create_dir(&path).unwrap();
        init(&path).unwrap();
        assert!(path.join("cgarena_config.toml").exists());
    }
}
