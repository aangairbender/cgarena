mod api;
mod arena;
mod arena_server;
mod config;
mod db;
mod domain;
mod embedded_worker;
mod ranking;

use clap::{command, Parser, Subcommand, ValueEnum};
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
    /// Initialize a new server or worker
    Init {
        target: Target,
        path: Option<String>,
    },
    /// Run server or worker
    Run {
        target: Target,
        path: Option<String>,
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

#[derive(Copy, Clone, ValueEnum)]
enum Target {
    Server,
    // Worker,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path, target } => {
            let path = unwrap_or_current_dir(path);
            match target {
                Target::Server => arena_server::init(&path),
            };
        }
        Commands::Run { path, target } => {
            let path = unwrap_or_current_dir(path);
            match target {
                Target::Server => arena_server::start(&path).await,
            }
        }
    }
}

fn unwrap_or_current_dir(path: Option<String>) -> PathBuf {
    path.map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("Can not get current directory"))
}
