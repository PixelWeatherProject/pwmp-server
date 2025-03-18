use clap::Parser;
use cli::Command;
use log::{debug, error, info};
use server::config::Config;
use simple_logger::SimpleLogger;
use sqlx::migrate::Migrator;
use std::process::exit;
use time::macros::format_description;

mod cli;
mod dbmgr;
mod error;
mod server;
mod svcmgr;
mod tester;

pub static MIGRATOR: Migrator = sqlx::migrate!();

fn main() -> Result<(), error::Error> {
    let args = cli::Cli::parse();

    let logger = SimpleLogger::new().with_timestamp_format(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]"
    ));

    let log_level = if args.debug || cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };

    let logger = logger.with_level(log_level);
    logger.init()?;

    info!("PixelWeather Server v{}", env!("CARGO_PKG_VERSION"));
    debug!("Arguments: {args:?}");

    let config_path = args.config.unwrap_or_else(Config::default_path);
    info!("Loading config from {}", config_path.display());

    let config: Config = match confy::load_path(config_path) {
        Ok(config) => config,
        Err(why) => {
            error!("Failed to load configuration: {why}");
            exit(1);
        }
    };

    match args.command {
        Some(Command::Service { command }) => svcmgr::main(command),
        Some(Command::Database { command }) => dbmgr::main(command, &config),
        Some(Command::Test { host, mac, port }) => tester::test(host, port, mac),
        None => server::main(config),
    }

    Ok(())
}
