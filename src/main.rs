use std::path::{Path, PathBuf};

use clap::{command, Parser, Subcommand};
use tracing::info;

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
    Server { path: String },
    /// Run worker
    Worker {
        #[arg(short, long, default_value_t = 1)]
        threads: u8,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();
    init_colored();

    let cli = Cli::parse();
    match cli.command {
        Commands::New { path } => {
            let path = PathBuf::from(path);
            create_new_arena(&path)?;
            info!("New arena has been created");
            Ok(())
        }
        Commands::Run { command } => handle_run(command).await,
    }
}

async fn handle_run(command: RunCommands) -> Result<(), anyhow::Error> {
    match command {
        RunCommands::Server { path } => server::start_arena_server(Path::new(&path)).await,
        RunCommands::Worker { threads: _ } => todo!(),
    }
}

#[cfg(windows)]
fn init_colored() {
    colored::control::set_virtual_terminal(true).unwrap();
}

#[cfg(not(windows))]
fn init_colored() {}

static DEFAULT_CONFIG_CONTENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/cgarena_config.toml"
));

const CONFIG_FILE_NAME: &str = "cgarena_config.toml";

fn create_new_arena(path: &Path) -> Result<(), std::io::Error> {
    match std::fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        e => return e,
    }

    let config_file_path = path.join(CONFIG_FILE_NAME);
    std::fs::write(config_file_path, DEFAULT_CONFIG_CONTENT)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn new_arena_can_be_created() {
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        let res = create_new_arena(&path);
        assert!(res.is_ok(), "Arena creation failed {:?}", res.err());
        assert!(path.join("cgarena_config.toml").exists());
    }
}