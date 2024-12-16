use std::path::PathBuf;

use cgarena::arena_server;
use clap::{command, Parser, Subcommand, ValueEnum};
use serde_json::json;

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
    Bot {
        #[command(subcommand)]
        command: BotCommands,
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
    Worker,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path, target } => {
            let path = unwrap_or_current_dir(path);
            match target {
                Target::Server => arena_server::init(&path),
                Target::Worker => unimplemented!(),
            };
        }
        Commands::Run { path, target } => {
            let path = unwrap_or_current_dir(path);
            match target {
                Target::Server => arena_server::start(&path).await,
                Target::Worker => unimplemented!(),
            }
        }
        Commands::Bot { command } => handle_bot(command).await,
    }
}

async fn handle_bot(command: BotCommands) {
    let arena_url = std::env::current_dir()
        .ok()
        .and_then(|cur_dir| cgarena::config::Config::load(&cur_dir).ok())
        .map(|c| c.server.port)
        .map(|port| format!("http://127.0.0.1:{}", port));
    match command {
        BotCommands::Add { name, src, lang } => {
            let source_code = tokio::fs::read_to_string(src)
                .await
                .expect("failed to read source file");
            let url = dotenvy::var("CGARENA_URL")
                .ok()
                .or(arena_url)
                .expect("CGARENA_URL environment variable not set");
            let body = json!({
                "name": name,
                "source_code": source_code,
                "language": lang,
            });
            let client = reqwest::Client::new();
            let res = client
                .post(url + "/api/bots")
                .json(&body)
                .send()
                .await
                .expect("failed to send request to the arena");

            match res.status() {
                s if s.is_success() => {
                    println!("Bot added successfully");
                }
                _ => eprintln!("Unexpected error occurred: {:#?}", res),
            }
        }
    }
}

fn unwrap_or_current_dir(path: Option<String>) -> PathBuf {
    path.map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("Can not get current directory"))
}
