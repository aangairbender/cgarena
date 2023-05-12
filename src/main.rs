use std::{fs::{File, self}, io::Write, path::Path};

use clap::{Parser, Subcommand};
use colored::Colorize;
use config::Config;
use models::{Bot, Match, Language};
use tabled::{Table, Tabled};

use crate::config::CONFIG_FILE;

pub mod models;
pub mod db;
pub mod config;
pub mod services;

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
        language: Option<String>,
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

#[cfg(windows)]
fn init_colored() {
    colored::control::set_virtual_terminal(true).unwrap();
}

#[cfg(not(windows))]
fn init_colored() {}

#[derive(Tabled)]
struct BotView<'a> {
    name: &'a str,
    language_name: &'a str,
    matches: u32,
    rating: f64,
}

impl<'a> From<&'a Bot> for BotView<'a> {
    fn from(bot: &'a Bot) -> Self {
        Self {
            name: &bot.name,
            language_name: &bot.language_name,
            matches: bot.completed_matches,
            rating: bot.rating(),
        }
    }
}

fn main() {
    init_colored();
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name } => cmd_new(&name),
        Commands::Run => todo!(),
        Commands::Bot { command } => match command {
            BotCommands::Add { name, description, file, language } => {
                cmd_bot_add(name, description, file, language);
            },
            BotCommands::Remove { name } => {
                cmd_bot_remove(name);
            },
            BotCommands::List => {
                cmd_bot_list();
            },
        },
        Commands::Match { command } => match command {
            MatchCommands::Add { p1, p2, p3, p4, p5, p6, p7, p8, seed, force_single } => todo!(),
        }
    }
}

fn cmd_bot_list() {
    let config = Config::open();
    let mut db = db::DB::open();
    let mut bots = db.fetch_bots();
    bots.sort_by(|a, b| a.rating().partial_cmp(&b.rating()).unwrap().reverse());
    let table = Table::new(bots.iter().map(|b| BotView::from(b)));
    println!("{}", table);
}

fn cmd_bot_remove(name: String) {
    let config = Config::open();
    let mut db = db::DB::open();
    db.delete_bot(&name);

    let bot_dir = std::env::current_dir().unwrap()
        .join("bots")
        .join(&name);
    fs::remove_dir_all(bot_dir)
        .expect("can't remove bot dir");
    println!("     {} bot '{}'", "Removed".bright_red(), name);
}

fn cmd_bot_add(name: String, description: Option<String>, file: String, language: Option<String>) {
    let config = Config::open();
    let mut db = db::DB::open();
    let languages = vec![
        Language {
            name: "cpp".to_string(),
            file_extension: "cpp".to_string(),
            health_check_cmd: vec!["g++".to_string(), "--version".to_string()],
            build_cmd: Some(vec!["g++".to_string(), "-O0".to_string(), "-g".to_string(), "-o bot".to_string(), "{SOURCE_FILE}".to_string()]),
            run_cmd: vec!["./bot".to_string()],
        }
    ];
    for lang in &languages {
        if !services::exec::health_check(lang) {
            panic!("Language {} didn't pass health check", &lang.name);
        }
    }
    let language_by_extension = |e| languages.iter().find(|lang| lang.file_extension == e);
    let language_by_name = |e| languages.iter().find(|lang| lang.name == e);

    let file_path = Path::new(&file);
    if !file_path.exists() {
        panic!("Provided source code file does not exist");
    }
    let file_language = file_path.extension()
        .and_then(|e| e.to_str())
        .and_then(language_by_extension);
    let language = language.and_then(language_by_name).or(file_language)
        .expect("Language should be supported");
    let bot_dir = std::env::current_dir().unwrap()
        .join("bots")
        .join(&name);
    fs::create_dir(&bot_dir)
        .expect("Should create a directory for a new bot");
    let source_code_file = bot_dir
        .join(format!("source.{}", &language.file_extension));
    fs::copy(file, &source_code_file)
        .expect("Should copy source code to the dedicated folder");

    services::exec::build_source_code(&name, source_code_file.to_str().unwrap(), language)
        .unwrap_or_else(|e| {
            fs::remove_dir_all(bot_dir)
                .expect("can't remove bot dir");
            println!("code should be without compile errors, but there are some:");
            println!("{}", e);
            panic!();
        });


    let bot = Bot::new(name, description.unwrap_or_default(), language.name.clone());
    let bot_name = bot.name.clone();
    db.insert_bot(bot);
    println!("     {} bot '{}' written in '{:?}'", "Added".bright_green(), bot_name, language.name);
}

fn cmd_new(name: &str) {
    let path = std::env::current_dir()
        .unwrap()
        .join(name);

    fs::create_dir(&path).unwrap();

    let mut config = Config::default();
    config.game.title = name.to_string();
    let toml_content = toml::to_string(&config)
        .expect("Default config should be serializable");

    let filename = path.join(CONFIG_FILE);
    let mut file = File::create(filename).unwrap();

    file.write_all(toml_content.as_bytes()).unwrap();

    let bots_folder = path.join("bots");
    fs::create_dir(bots_folder).unwrap();

    println!("     {} arena for the game \'{}\'", "Created".bright_green(), name);
}
