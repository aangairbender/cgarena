use std::path::PathBuf;

use config::Config;

pub mod config;
pub mod models;
pub mod services;

pub enum ArenaState {
    Unitialized,
    Initialized { arena_root: PathBuf, config: Config },
}

fn main() {}
