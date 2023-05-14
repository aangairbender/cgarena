mod bot_subcommand_handler;

use std::path::Path;

use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::{config::Config, services::tool, ArenaState};

use self::bot_subcommand_handler::BotSubCommandHandler;

use super::{bot_service::BotService, db::DB};

pub struct CliService<'a> {
    arena_state: &'a ArenaState,
}

impl<'a> CliService<'a> {
    pub fn new(arena_state: &'a ArenaState) -> Self {
        Self { arena_state }
    }

    pub fn run(&mut self) {
        init_colored();
        let cli = Cli::parse();

        match cli.command {
            Commands::New { name } => self.cmd_new(&name),
            Commands::Run => todo!(),
            Commands::Bot { command } => {
                match &self.arena_state {
                    ArenaState::Unitialized => panic!("You should run this command from the arena folder. Try creating one first with 'cg-local-arena new' command"),
                    ArenaState::Initialized { arena_root, config } => self.cmd_bot(command, arena_root, config),
                }
            },
            // Commands::Match { command } => match command {
            //     MatchCommands::Add { p1, p2, p3, p4, p5, p6, p7, p8, seed, force_single } => todo!(),
            // }
        }
    }

    fn cmd_bot(&mut self, command: BotCommands, arena_root: &Path, config: &Config) {
        let db = DB::open(arena_root);
        let bot_service = BotService::new(config, &db);
        let handler = BotSubCommandHandler::new(&bot_service);
        match command {
            BotCommands::Add {
                name,
                file,
                language,
            } => {
                handler.cmd_bot_add(name, file, language);
            }
            BotCommands::Remove { name } => {
                handler.cmd_bot_remove(name);
            }
            BotCommands::List => {
                handler.cmd_list();
            }
        }
    }

    fn cmd_new(&mut self, name: &str) {
        tool::create_new_arena(name);
        println!(
            "     {} arena for the game \'{}\'",
            "Created".bright_green(),
            name
        );
    }
}

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
    // Match {
    //     #[command(subcommand)]
    //     command: MatchCommands,
    // },
}

#[derive(Subcommand)]
enum BotCommands {
    Add {
        name: String,
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

#[cfg(windows)]
fn init_colored() {
    colored::control::set_virtual_terminal(true).unwrap();
}

#[cfg(not(windows))]
fn init_colored() {}
