use crate::{
    cli::DatabaseCommand,
    server::{
        config::Config,
        db::{DatabaseBackend, DatabaseClient, EraseOptions},
    },
};
use std::{
    io::{Write, stdin, stdout},
    process::exit,
};
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
            confirm_erase(&config.database.name(), &config.database.host());

            let opts = EraseOptions::new(content_only, keep_devices);
            match client.erase(opts).await {
                Ok(()) => info!("Success!"),
                Err(why) => error!("Failed to erase database: {why}"),
            }
        }
    }
}

fn confirm_erase(database_name: &str, host: &str) {
    const KEY: &str = "yes, do it!";

    println!("==================");
    println!("WARNING:");
    println!("==================");
    println!();
    println!(
        "This action will permanently erase data from your database ({host}/{database_name})!"
    );
    println!();
    print!("Type '{KEY}' to confirm: ");

    let _ = stdout().flush();

    let mut buf = String::new();
    stdin().read_line(&mut buf).unwrap_or_default();

    if buf.trim_end() != KEY {
        info!("Operation cancelled, nothing was done.");
        exit(1);
    }
}
