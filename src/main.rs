use clap::Parser;
use cli::Command;
use server::config::Config;
use sqlx::migrate::Migrator;
use std::process::exit;
use tracing::{debug, error, info};

mod cli;
mod dbmgr;
mod error;
mod logging;
mod server;
mod svcmgr;
mod tester;

pub static MIGRATOR: Migrator = sqlx::migrate!();

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let args = cli::Cli::parse();

    logging::setup(args.debug)?;

    info!("PixelWeather Server v{}", env!("CARGO_PKG_VERSION"));
    debug!("Arguments: {args:?}");

    let config_path = args.config.unwrap_or_else(Config::default_path);
    info!("Loading config from {}", config_path.display());

    let first_run = !config_path.exists();
    let config: Config = match confy::load_path(&config_path) {
        Ok(config) => config,
        Err(why) => {
            error!("Failed to load configuration: {why}");
            exit(1);
        }
    };

    if first_run {
        info!("Configuration initialized at {}", config_path.display());
        return Ok(());
    }

    match args.command {
        Some(Command::Service { command }) => svcmgr::main(command),
        Some(Command::Database { command }) => dbmgr::main(command, &config).await,
        Some(Command::Test { host, mac, port }) => tester::test(host, port, mac),
        None => server::main(config).await,
    }

    Ok(())
}
