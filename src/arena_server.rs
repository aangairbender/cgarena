use crate::{
    api,
    config::{BootstrapConfig, Config},
    db,
    runtime::ArenaRuntime,
};
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
    let bootstrap =
        BootstrapConfig::load(arena_path).context("Cannot load bootstrap configuration")?;

    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(
            arena_path.join(
                bootstrap
                    .log
                    .file
                    .unwrap_or_else(|| "cgarena.log".to_string()),
            ),
        )
        .context("Cannot write to cgarena.log")?;

    let log_level = bootstrap
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
    db::migrate(&pool).await?;
    let saved_configuration = load_or_migrate_arena_configuration(arena_path, &pool)
        .await
        .context("Cannot load arena config")?;

    let runtime = ArenaRuntime::new(pool.clone(), arena_path.to_owned());
    if let Some(configuration) = saved_configuration {
        runtime.start_saved(configuration).await;
        if let Some(error) = runtime.last_error().await {
            warn!("Arena runtime is unavailable: {error}");
        }
    }

    let exposed = bootstrap.server.expose;
    let addr = if exposed {
        SocketAddr::from(([0, 0, 0, 0], bootstrap.server.port))
    } else {
        SocketAddr::from(([127, 0, 0, 1], bootstrap.server.port))
    };
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Port is already in use")?;
    let bind_addr = listener
        .local_addr()
        .context("Cannot get local address of tcp binding")?;
    let token = CancellationToken::new();
    let mut api_task = tokio::spawn(api::start(
        listener,
        pool.clone(),
        runtime.clone(),
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
        println!("Network: use 'server.expose' config param to expose");
    }
    println!();

    let api_finished = tokio::select! {
        _ = shutdown_signal() => {
            println!("Stopping CG Arena... press Ctrl+C again to kill it");
            false
        },
        _ = &mut api_task => true,
    };

    token.cancel();
    if !api_finished {
        api_task.await.context("API task terminated unexpectedly")?;
    }
    runtime
        .shutdown()
        .await
        .context("Cannot stop arena runtime")?;
    pool.close().await;
    info!("CG Arena stopped");

    if api_finished {
        bail!("API task terminated unexpectedly");
    }
    Ok(())
}

async fn load_or_migrate_arena_configuration(
    arena_path: &Path,
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<Option<crate::config::ArenaConfig>> {
    if let Some(config) = db::fetch_arena_config(pool).await? {
        return Ok(Some(config));
    }

    let Some(legacy) = Config::load_legacy(arena_path)? else {
        return Ok(None);
    };
    legacy.validate().context("Invalid legacy configuration")?;

    let Config {
        game,
        matchmaking,
        ranking,
        server,
        log,
        leaderboards,
        workers,
    } = legacy;
    let arena_config = crate::config::ArenaConfig {
        game,
        matchmaking,
        ranking,
        leaderboards,
        workers,
    };
    let bootstrap = BootstrapConfig { server, log };
    let bootstrap_content =
        toml::to_string_pretty(&bootstrap).context("Cannot serialize bootstrap configuration")?;

    let config_path = arena_path.join("cgarena_config.toml");
    let archive_path = arena_path.join("cgarena_config.pre-ui.toml");
    let pending_path = arena_path.join(".cgarena_config.toml.migrating");
    if archive_path.exists() {
        bail!(
            "Cannot migrate configuration because {} already exists",
            archive_path.display()
        );
    }

    let mut pending = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending_path)
        .context("Cannot create pending bootstrap configuration")?;
    if let Err(error) = pending
        .write_all(bootstrap_content.as_bytes())
        .and_then(|_| pending.sync_all())
    {
        let _ = std::fs::remove_file(&pending_path);
        return Err(error).context("Cannot write pending bootstrap configuration");
    }
    drop(pending);

    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            let _ = std::fs::remove_file(&pending_path);
            return Err(error).context("Cannot begin configuration migration");
        }
    };
    if let Err(error) = db::persist_arena_config_transaction(&mut transaction, &arena_config).await
    {
        let _ = std::fs::remove_file(&pending_path);
        return Err(error).context("Cannot persist migrated arena configuration");
    }

    if let Err(error) = std::fs::rename(&config_path, &archive_path) {
        let _ = std::fs::remove_file(&pending_path);
        return Err(error).context("Cannot archive legacy configuration");
    }
    if let Err(error) = std::fs::rename(&pending_path, &config_path) {
        let restore = std::fs::rename(&archive_path, &config_path);
        let _ = std::fs::remove_file(&pending_path);
        if let Err(restore_error) = restore {
            return Err(error).context(format!(
                "Cannot activate bootstrap configuration; restoring the legacy file also failed: {restore_error}"
            ));
        }
        return Err(error).context("Cannot activate bootstrap configuration");
    }

    if let Err(error) = transaction.commit().await {
        let remove_result = std::fs::remove_file(&config_path);
        let restore_result = std::fs::rename(&archive_path, &config_path);
        if let Err(rollback_error) = remove_result.and(restore_result) {
            return Err(error).context(format!(
                "Cannot commit configuration migration; restoring the legacy file also failed: {rollback_error}"
            ));
        }
        return Err(error).context("Cannot commit configuration migration");
    }

    Ok(Some(arena_config))
}

static DEFAULT_FILES: &[(&str, &str)] = &[(
    "cgarena_config.toml",
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/bootstrap_config.toml"
    )),
)];

pub async fn init(path: &Path) -> anyhow::Result<()> {
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
    let pool = db::connect(path)
        .await
        .context("Cannot create arena database")?;
    db::migrate(&pool).await?;
    pool.close().await;
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
    use crate::config::WorkerConfig;

    #[tokio::test]
    async fn new_arena_can_be_created_in_new_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        init(&path).await.unwrap();

        let bootstrap: toml::Value =
            toml::from_str(&std::fs::read_to_string(path.join("cgarena_config.toml")).unwrap())
                .unwrap();
        assert_eq!(
            bootstrap.as_table().unwrap().keys().collect::<Vec<_>>(),
            vec!["log", "server"]
        );
        assert!(path.join("cgarena.db").exists());
        assert!(!path.join("CommandLineInterface.java").exists());
        assert!(!path.join("pom_build_section.xml").exists());

        let pool = db::connect(&path).await.unwrap();
        db::migrate(&pool).await.unwrap();
        assert!(db::fetch_arena_config(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn new_arena_can_be_created_in_existing_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test");
        std::fs::create_dir(&path).unwrap();
        init(&path).await.unwrap();
        assert!(path.join("cgarena_config.toml").exists());
        assert!(path.join("cgarena.db").exists());
    }

    async fn legacy_arena() -> (tempfile::TempDir, sqlx::SqlitePool, String) {
        let directory = tempfile::tempdir().unwrap();
        let legacy = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/default_config.toml"
        ))
        .replace(
            "[workers.referee]\ntype = \"managed_codingame\"\nrepository_url = \"https://github.com/CodinGame/SpringChallenge2023.git\"",
            "cmd_play_match = \"runner\"\ncmd_watch_replay = \"renderer\"",
        );
        std::fs::write(directory.path().join("cgarena_config.toml"), &legacy).unwrap();
        let pool = db::connect(directory.path()).await.unwrap();
        db::migrate(&pool).await.unwrap();
        (directory, pool, legacy)
    }

    #[tokio::test]
    async fn legacy_configuration_is_imported_archived_and_slimmed_once() {
        let (directory, pool, legacy) = legacy_arena().await;

        let imported = load_or_migrate_arena_configuration(directory.path(), &pool)
            .await
            .unwrap()
            .unwrap();

        let WorkerConfig::Embedded(worker) = &imported.workers[0];
        let crate::config::RefereeConfig::Command(referee) = &worker.referee else {
            panic!("legacy commands must import as the command referee");
        };
        assert!(referee.legacy);
        assert_eq!(referee.play_match, "runner");
        assert_eq!(
            std::fs::read_to_string(directory.path().join("cgarena_config.pre-ui.toml")).unwrap(),
            legacy
        );
        let bootstrap: toml::Value = toml::from_str(
            &std::fs::read_to_string(directory.path().join("cgarena_config.toml")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            bootstrap.as_table().unwrap().keys().collect::<Vec<_>>(),
            vec!["log", "server"]
        );

        let original_archive =
            std::fs::read_to_string(directory.path().join("cgarena_config.pre-ui.toml")).unwrap();
        load_or_migrate_arena_configuration(directory.path(), &pool)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(directory.path().join("cgarena_config.pre-ui.toml")).unwrap(),
            original_archive
        );
    }

    #[tokio::test]
    async fn archive_collision_leaves_legacy_configuration_and_database_untouched() {
        let (directory, pool, legacy) = legacy_arena().await;
        std::fs::write(
            directory.path().join("cgarena_config.pre-ui.toml"),
            "existing archive",
        )
        .unwrap();

        let error = load_or_migrate_arena_configuration(directory.path(), &pool)
            .await
            .err()
            .expect("archive collision must fail migration");

        assert!(error.to_string().contains("already exists"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("cgarena_config.toml")).unwrap(),
            legacy
        );
        assert!(db::fetch_arena_config(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn invalid_legacy_configuration_is_not_migrated() {
        let (directory, pool, legacy) = legacy_arena().await;
        let invalid = legacy.replace("min_players = 2", "min_players = 0");
        std::fs::write(directory.path().join("cgarena_config.toml"), &invalid).unwrap();

        let error = load_or_migrate_arena_configuration(directory.path(), &pool)
            .await
            .err()
            .expect("invalid configuration must fail migration");

        assert!(error.to_string().contains("Invalid legacy configuration"));
        assert_eq!(
            std::fs::read_to_string(directory.path().join("cgarena_config.toml")).unwrap(),
            invalid
        );
        assert!(!directory.path().join("cgarena_config.pre-ui.toml").exists());
        assert!(db::fetch_arena_config(&pool).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn database_failure_leaves_legacy_configuration_untouched() {
        let (directory, pool, legacy) = legacy_arena().await;
        pool.close().await;

        assert!(load_or_migrate_arena_configuration(directory.path(), &pool)
            .await
            .is_err());
        assert_eq!(
            std::fs::read_to_string(directory.path().join("cgarena_config.toml")).unwrap(),
            legacy
        );
        assert!(!directory.path().join("cgarena_config.pre-ui.toml").exists());
    }
}
