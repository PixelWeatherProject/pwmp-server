use crate::{
    cli::DatabaseCommand,
    server::{
        config::Config,
        database::{Backend, DatabaseClient},
    },
};
use color_print::cprintln;
use std::{io::stdin, process::exit};
use tracing::{debug, error, info};

#[allow(clippy::cognitive_complexity)]
pub async fn main(cmd: DatabaseCommand, config: &Config) {
    match cmd {
        DatabaseCommand::Test => match DatabaseClient::new(config).await {
            Ok(_) => info!("Connection successful"),
            Err(why) => error!("Failed to connect: {why}"),
        },
        DatabaseCommand::Init => {
            debug!("Initializing database pool");
            let client = match DatabaseClient::new(config).await {
                Ok(conn) => conn,
                Err(why) => {
                    error!("Failed to connect: {why}");
                    exit(1);
                }
            };

            info!("Executing migrations");
            match client.run_migrations().await {
                Ok(()) => info!("Migrations executed successfully"),
                Err(why) => error!("Failed to execute migrations: {why}"),
            }
        }
        DatabaseCommand::Erase {
            content_only,
            keep_devices,
        } => {
            let client = match DatabaseClient::new(config).await {
                Ok(conn) => conn,
                Err(why) => {
                    error!("Failed to connect: {why}");
                    exit(1);
                }
            };

            info!("Connected to the database");
            confirm_erase(&config.short_db_identifier());

            match client.erase(content_only, keep_devices).await {
                Ok(()) => info!("Success!"),
                Err(why) => error!("Failed to erase database: {why}"),
            }
        }
    }
}

fn confirm_erase(database: &str) {
    const KEY: &str = "yes, do it!";

    cprintln!(
        "\n<red><bold><underline>WARNING:</> <yellow>THIS ACTION WILL COMPLETELE ERASE <underline>ALL DATA</underline> AND <italic>(IF SPECIFIED)</italic> <underline>TABLES</underline> FROM THE DATABASE</> <bright-blue><bold>\"{database}\"</> <yellow><bold>!!!</>"
    );
    cprintln!("\n<blue>TYPE <italic>\"{KEY}\"</italic> TO CONFIRM THIS OPERATION!</>");

    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap_or_default();

    if buf.trim_end() != KEY {
        info!("Operation cancelled, nothing was done.");
        exit(1);
    }
}
