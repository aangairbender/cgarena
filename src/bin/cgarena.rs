use std::{error::Error, path::PathBuf};

use cg_arena::server::{create_new_arena, start_arena_server};
use clap::{command, Parser, Subcommand};
use simplelog::TermLogger;

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
}

#[derive(Subcommand)]
enum RunCommands {
    /// Run server
    Server,
    /// Run worker
    Worker {
        #[arg(short, long, default_value_t = 1)]
        threads: u8,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_colored();
    TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    let cli = Cli::parse();
    match cli.command {
        Commands::New { path } => {
            let path = PathBuf::from(path);
            create_new_arena(&path)?;
            log::info!("New arena has been created");
            Ok(())
        }
        Commands::Run { command } => handle_run(command).await,
    }
}

async fn handle_run(command: RunCommands) -> Result<(), Box<dyn Error>> {
    let path = std::env::current_dir()?;
    match command {
        RunCommands::Server => start_arena_server(&path).await,
        RunCommands::Worker { threads: _ } => todo!(),
    }
}

#[cfg(windows)]
fn init_colored() {
    colored::control::set_virtual_terminal(true).unwrap();
}

#[cfg(not(windows))]
fn init_colored() {}
