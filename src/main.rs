mod api;
mod arena;
mod arena_commands;
mod arena_handle;
mod arena_server;
#[cfg(test)]
mod arena_tests;
mod async_leaderboard;
mod chart;
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
    /// Wipe old matches and vacuum db after that .
    WipeOldMatches {
        /// Path to the arena directory.
        /// If omitted the current working directory is used.
        path: Option<String>,

        /// The percentage of old matches to be wiped, 0..100
        #[arg(short, long)]
        percentage: u8,

        /// Automatically answer "yes" to prompts
        #[arg(short)]
        yes: bool,

        /// Whether to vacuum the db afterwards (reclaim occupied disk space).
        #[arg(short, long)]
        vacuum: bool,
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
        Commands::WipeOldMatches {
            path,
            percentage,
            yes,
            vacuum,
        } => {
            let path = unwrap_or_current_dir(path)?;
            db::wipe_old_matches(&path, percentage, vacuum, |cnt| {
                if yes {
                    true
                } else {
                    confirm_wiping_matches(cnt)
                }
            })
            .await?;
            println!("Done.")
        }
    }
    Ok(())
}

fn confirm_wiping_matches(cnt: usize) -> bool {
    println!("{} matches would be deleted. Continue? (y/n)", cnt);
    loop {
        let mut s = String::new();
        std::io::stdin().read_line(&mut s).unwrap();
        match s.trim().chars().next() {
            Some(c) if c.eq_ignore_ascii_case(&'y') => break true,
            None => continue,
            _ => break false,
        }
    }
}

fn unwrap_or_current_dir(path: Option<String>) -> anyhow::Result<PathBuf> {
    path.map(PathBuf::from).map_or_else(
        || std::env::current_dir().context("Cannot get current directory"),
        Ok,
    )
}
