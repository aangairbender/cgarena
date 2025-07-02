mod api;
mod arena;
mod arena_server;
mod config;
mod db;
mod domain;
mod embedded_worker;
mod matchmaking;
mod ranking;

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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path } => {
            let path = unwrap_or_current_dir(path);
            arena_server::init(&path);
        }
        Commands::Run { path } => {
            let path = unwrap_or_current_dir(path);
            arena_server::start(&path).await;
        }
    }
}

fn unwrap_or_current_dir(path: Option<String>) -> PathBuf {
    path.map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("Can not get current directory"))
}
