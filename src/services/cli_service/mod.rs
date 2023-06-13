mod bot_subcommand_handler;

use std::{path::{Path, PathBuf}, rc::Rc};

use clap::{Parser, Subcommand};
use colored::Colorize;

use crate::config::{Config, self};

use self::bot_subcommand_handler::BotSubCommandHandler;

use super::{bot_service::BotService, db::DB};

pub struct CliService {}

impl CliService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run(&mut self) {
        init_colored();
        let cli = Cli::parse();

        match cli.command {
            Commands::New { name } => self.cmd_new(&name),
            Commands::Run => todo!(),
            Commands::Bot { command } => {
                match Self::arena_state() {
                    ArenaState::Unitialized => panic!("You should run this command from the arena folder. Try creating one first with 'cgarena new' command"),
                    ArenaState::Initialized { arena_root, config } => todo!()//self.cmd_bot(command, &arena_root, &config),
                }
            },
            // Commands::Match { command } => match command {
            //     MatchCommands::Add { p1, p2, p3, p4, p5, p6, p7, p8, seed, force_single } => todo!(),
            // }
        }
    }

    fn arena_state() -> ArenaState {
        let cur_dir = std::env::current_dir()
            .unwrap();
        let config_file = cur_dir.join(config::CONFIG_FILE);
        if config_file.exists() {
            ArenaState::Initialized { arena_root: cur_dir, config: Config::open()}
        } else { ArenaState::Unitialized }
    }

    // fn cmd_bot(&mut self, command: BotCommands, arena_root: &Path, config: &Config) {
    //     let db = DB::open(arena_root);
    //     let bot_service = BotService::new(config, &db);
    //     let handler = BotSubCommandHandler::new(Rc::new(bot_service));
    //     match command {
    //         BotCommands::Add {
    //             name,
    //             file,
    //             language,
    //         } => {
    //             handler.cmd_bot_add(name, file, language);
    //         }
    //         BotCommands::Remove { name } => {
    //             handler.cmd_bot_remove(name);
    //         }
    //         BotCommands::List => {
    //             handler.cmd_list();
    //         }
    //     }
    // }

    fn cmd_new(&mut self, name: &str) {
        tool::create_new_arena(name);
        println!(
            "     {} arena for the game \'{}\'",
            "Created".bright_green(),
            name
        );
    }
}
