use clap::Parser;
use cli::Command;
use server::config::Config;
use std::env;
use tracing::{debug, info, warn};

mod cli;
mod dbmgr;
mod error;
mod logging;
mod otautil;
mod server;
mod svcmgr;
mod tester;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let args = cli::Cli::parse();
    let config_path = args.config.clone().unwrap_or_else(Config::default_path);
    let (config, first_run) = server::config::setup(&config_path)?;
    let force_debug =
        args.debug || env::var("PWMP_DEBUG").is_ok_and(|value| value.to_lowercase() == "true");

    if let Err(why) = logging::setup(force_debug, &config) {
        eprintln!("Failed to set up logging: {why}");
        return Err(why);
    }

    info!("PixelWeather Server v{}", env!("CARGO_PKG_VERSION"));
    debug!("Arguments: {args:?}");

    if first_run {
        warn!("No configuration found, creating one");
        info!("Configuration initialized at {}", config_path.display());
        return Ok(());
    }

    match args.command {
        Some(Command::Service { command }) => svcmgr::main(command),
        Some(Command::Database { command }) => dbmgr::main(command, &config).await,
        Some(Command::Test { host, mac, port }) => tester::test(host, port, mac),
        Some(Command::Ota { command }) => otautil::run(command, &config).await?,
        None => server::main(config).await,
    }

    Ok(())
}
