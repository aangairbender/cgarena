use clap::{Parser, Subcommand};

pub mod models;
pub mod db;
pub mod config;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // init the config file
    Init,
    // run the arena
    Run,
    Bot {
        #[command(subcommand)]
        command: BotCommands,
    },
    Match {
        #[command(subcommand)]
        command: MatchCommands,
    },
}

#[derive(Subcommand)]
enum BotCommands {
    Add {
        name: String,
        #[arg(short, long)]
        description: Option<String>,
        #[arg(short, long)]
        cmd: String,
    },
    Remove {
        name: String,
    }
}

#[derive(Subcommand)]
enum MatchCommands {
    Add {
        #[arg(long)]
        p1: Option<String>,
        #[arg(long)]
        p2: Option<String>,
        #[arg(long)]
        p3: Option<String>,
        #[arg(long)]
        p4: Option<String>,
        #[arg(long)]
        p5: Option<String>,
        #[arg(long)]
        p6: Option<String>,
        #[arg(long)]
        p7: Option<String>,
        #[arg(long)]
        p8: Option<String>,
        #[arg(short, long)]
        seed: i32,
        #[arg(long)]
        force_single: Option<bool>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init => {
            init();
        }
    }
}

fn init() {

}