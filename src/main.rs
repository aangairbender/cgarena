mod api;
mod matchmaking;
mod config;
mod db;
mod server;
mod worker;

use std::path::{Path, PathBuf};

use anyhow::bail;
use clap::{command, Parser, Subcommand};
use config::Config;
use serde_json::json;
use tokio::fs;
use tracing::info;

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new arena at `path`
    New { path: String },
    /// Run server or worker
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },
    Bot {
        #[command(subcommand)]
        command: BotCommands,
    },
}

#[derive(Subcommand)]
enum RunCommands {
    /// Run server
    Server { path: String },
    /// Run worker
    Worker {
        #[arg(short, long, default_value_t = 1)]
        threads: u8,
    },
}

#[derive(Subcommand)]
enum BotCommands {
    Add {
        #[arg(help = "Name of the bot, must be unique")]
        name: String,
        #[arg(short, long, help = "Path to the bot's source file")]
        src: String,
        #[arg(short, long, help = "Bot's language")]
        lang: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();
    init_colored();

    let cli = Cli::parse();
    match cli.command {
        Commands::New { path } => {
            let path = PathBuf::from(path);
            create_new_arena(&path)?;
            info!("New arena has been created");
            Ok(())
        }
        Commands::Run { command } => handle_run(command).await,
        Commands::Bot { command } => handle_bot(command).await,
    }
}

async fn handle_run(command: RunCommands) -> Result<(), anyhow::Error> {
    match command {
        RunCommands::Server { path } => server::start_server(Path::new(&path)).await,
        RunCommands::Worker { threads: _ } => todo!(),
    }
}

async fn handle_bot(command: BotCommands) -> Result<(), anyhow::Error> {
    match command {
        BotCommands::Add { name, src, lang } => {
            let source_code = fs::read_to_string(src).await?;
            let url = std::env::var("CGARENA_URL").unwrap_or("127.0.0.1:12345".to_string());
            let body = json!({
                "name": name,
                "source_code": source_code,
                "language": lang,
            });
            let client = reqwest::Client::new();
            let res = client
                .post(url + "/api/bots")
                .body(body.to_string())
                .send()
                .await?;
            if res.status().is_success() {
                Ok(())
            } else {
                bail!("Unexpected error: {:#?}", res)
            }
        }
    }
}

#[cfg(windows)]
fn init_colored() {
    colored::control::set_virtual_terminal(true).unwrap();
}

#[cfg(not(windows))]
fn init_colored() {}

fn create_new_arena(path: &Path) -> Result<(), anyhow::Error> {
    match std::fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        e => e?,
    }

    let res = Config::create_default(path);
    if let Err(e) = res {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            bail!("Arena already exists")
        }
        return Err(anyhow::Error::new(e));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn new_arena_can_be_created() {
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        let res = create_new_arena(&path);
        assert!(res.is_ok(), "Arena creation failed {:?}", res.err());
        assert!(path.join("cgarena_config.yaml").exists());
    }
}
