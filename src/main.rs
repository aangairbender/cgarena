use std::{fs::{File, OpenOptions, self}, io::{Write, ErrorKind}, path::Path};

use clap::{Parser, Subcommand};
use colored::{Colorize, control};
use config::Config;
use models::{Bot, Match, Language};
use polodb_core::bson::doc;
use tabled::Table;
use uuid::Uuid;

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
    New {
        name: String,
    },
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
        file: String,
        #[arg(short, long, value_enum)]
        language: Option<Language>,
    },
    Remove {
        name: String,
    },
    List,
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
    control::set_virtual_terminal(true).unwrap();
    let cli = Cli::parse();
    let mut db = db::DB::open();

    match cli.command {
        Commands::New { name } => cmd_new(&name),
        Commands::Run => todo!(),
        Commands::Bot { command } => match command {
            BotCommands::Add { name, description, file, language } => {
                let extension = Path::new(&file).extension();
                let language = language.or_else(|| extension
                                                        .and_then(|e| e.to_str())
                                                        .and_then(Language::from_file_extension)).unwrap();
                let bots_dir = std::env::current_dir().unwrap().join("bots");
                if !bots_dir.exists() {
                    panic!("You should run this command inside the arena folder. Use --help for more info.")
                }
                let source_code_file = format!("{}.{}", &name, &language.to_file_extension());
                let target_file = bots_dir.join(source_code_file.clone());
                fs::copy(file, target_file).unwrap();

                let bot = Bot::new(name, description.unwrap_or_default(), source_code_file, language);
                let bot_name = bot.name.clone();
                db.insert_bot(bot);
                println!("     {} bot '{}' written in '{:?}'", "Added".bright_green(), bot_name, language);
            },
            BotCommands::Remove { name } => {
                db.delete_bot(&name);

                let bots_dir = std::env::current_dir().unwrap().join("bots");
                // todo delete bot source code
                println!("     {} bot '{}'", "Removed".bright_red(), name);
            },
            BotCommands::List => {
                let mut bots = db.fetch_bots();
                bots.sort_by(|a, b| a.estimated_rating().partial_cmp(&b.estimated_rating()).unwrap().reverse());
                println!("{}", Table::new(bots));
            },
        },
        Commands::Match { command } => match command {
            MatchCommands::Add { p1, p2, p3, p4, p5, p6, p7, p8, seed, force_single } => todo!(),
        }
    }
}

fn cmd_new(name: &str) {
    let path = std::env::current_dir()
        .unwrap()
        .join(name);

    fs::create_dir(&path).unwrap();

    let config = Config::default();
    let toml_content = toml::to_string(&config)
        .expect("Default config should be serializable");

    let filename = path.join("config.toml");
    let mut file = File::create(filename).unwrap();

    file.write_all(toml_content.as_bytes()).unwrap();

    let bots_folder = path.join("bots");
    fs::create_dir(bots_folder).unwrap();

    println!("     {} arena for the game \'{}\'", "Created".bright_green(), name);
}
