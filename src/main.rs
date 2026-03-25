use crate::config::Config;
use clap::Parser;
use cli::Command;
use tracing::{debug, info};

mod cli;
mod config;
mod dbmgr;
mod error;
mod logging;
mod server;
mod svcmgr;
mod tester;
mod webapi;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let args = cli::Cli::parse();
    let config_path = args.config.clone().unwrap_or_else(Config::default_path);
    let (config, first_run) = config::setup(&config_path)?;

    if let Err(why) = logging::setup(args.debug, &config) {
        eprintln!("Failed to set up logging: {why}");
        return Err(why);
    }

    info!("PixelWeather Server v{}", env!("CARGO_PKG_VERSION"));
    debug!("Arguments: {args:?}");

    if first_run {
        info!("Configuration initialized at {}", config_path.display());
        return Ok(());
    }

    match args.command {
        Some(Command::Service { command }) => svcmgr::main(command),
        Some(Command::WebApi) => webapi::start(&config)?,
        Some(Command::Database { command }) => dbmgr::main(command, &config).await,
        Some(Command::Test { host, mac, port }) => tester::test(host, port, mac),
        None => server::main(config).await,
    }

    Ok(())
}
