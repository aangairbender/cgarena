mod api;
mod arena;
mod arena_handle;
mod arena_server;
#[cfg(test)]
mod arena_tests;
mod async_leaderboard;
mod config;
mod db;
mod domain;
mod matchmaking;
mod ranking;
mod worker;

use anyhow::Context;
use clap::{command, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new arena
    Init {
        /// Path to the arena directory. Path would be created if it does not exist.
        /// If omitted the current working directory is used.
        path: Option<String>,
    },
    /// Run existing arena
    Run {
        /// Path to the arena directory.
        /// If omitted the current working directory is used.
        path: Option<String>,
    },
    /// Vacuum DB - clean up space taken by deleted data (e.g. matches).
    VacuumDB {
        /// Path to the arena directory.
        /// If omitted the current working directory is used.
        path: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    handle_cli_command(cli.command).await?;
    Ok(())
}

async fn handle_cli_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Init { path } => {
            let path = unwrap_or_current_dir(path)?;
            arena_server::init(&path)?;
        }
        Commands::Run { path } => {
            let path = unwrap_or_current_dir(path)?;
            arena_server::start(&path).await?;
        }
        Commands::VacuumDB { path } => {
            let path = unwrap_or_current_dir(path)?;
            print!("Vacuum process started... ");
            db::vacuum_db(&path).await?;
            println!("Done.")
        }
    }
    Ok(())
}

fn unwrap_or_current_dir(path: Option<String>) -> anyhow::Result<PathBuf> {
    path.map(PathBuf::from).map_or_else(
        || std::env::current_dir().context("Cannot get current directory"),
        Ok,
    )
}
