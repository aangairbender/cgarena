use std::{error::Error, path::PathBuf};

use cg_arena::server::{ArenaService, start_arena_server};
use clap::{Parser, Subcommand, command};
use simplelog::{SimpleLogger, TermLogger, Config, CombinedLogger};


#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new arena at <path>
    New {
        path: String,
    },
    /// Run server or worker
    Run {
        #[command(subcommand)]
        command: RunCommands,
    },
    // Manage the bots
    // Bot {
    //     #[command(subcommand)]
    //     command: BotCommands,
    // },
    // Match {
    //     #[command(subcommand)]
    //     command: MatchCommands,
    // },
}

#[derive(Subcommand)]
enum RunCommands {
    /// run server
    Server,
    /// run worker
    Worker {
        #[arg(short, long, default_value_t = 1)]
        threads: u8,
    }
}

#[derive(Subcommand)]
enum BotCommands {
    /// Add a new bot
    Add {
        name: String,
        #[arg(short, long)]
        file: String,
        #[arg(short, long, value_enum)]
        language: Option<String>,
    },
    /// Remove existing bot
    Remove {
        name: String,
    },
    /// List all the bots
    List,
}

// #[derive(Subcommand)]
// enum MatchCommands {
//     Add {
//         #[arg(long)]
//         p1: Option<String>,
//         #[arg(long)]
//         p2: Option<String>,
//         #[arg(long)]
//         p3: Option<String>,
//         #[arg(long)]
//         p4: Option<String>,
//         #[arg(long)]
//         p5: Option<String>,
//         #[arg(long)]
//         p6: Option<String>,
//         #[arg(long)]
//         p7: Option<String>,
//         #[arg(long)]
//         p8: Option<String>,
//         #[arg(short, long)]
//         seed: i32,
//         #[arg(long)]
//         force_single: Option<bool>,
//     },
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    TermLogger::init(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto
    ).unwrap();
    // init_colored();
    let cli = Cli::parse();

    match cli.command {
        Commands::New { path } => {
            let path = PathBuf::from(path);
            ArenaService::create_new_arena(&path)?;
            log::info!("New arena has been created");
            Ok(())
        },
        Commands::Run { command } => handle_run(command).await,
        // Commands::Bot { command } => handle_bot(command).await,
        // Commands::Match { command } => match command {
        //     MatchCommands::Add { p1, p2, p3, p4, p5, p6, p7, p8, seed, force_single } => todo!(),
        // }
    }
}

async fn handle_run(command: RunCommands) -> Result<(), Box<dyn Error>> {
    let path = std::env::current_dir()?;
    match command {
        RunCommands::Server => start_arena_server(&path).await,
        RunCommands::Worker { threads } => todo!(),
    }
}

async fn handle_bot(command: BotCommands) -> Result<(), Box<dyn Error>> {
    let path = std::env::current_dir()?;
    todo!("read config and init arena client");
}

#[cfg(windows)]
fn init_colored() {
    colored::control::set_virtual_terminal(true).unwrap();
}

#[cfg(not(windows))]
fn init_colored() {}