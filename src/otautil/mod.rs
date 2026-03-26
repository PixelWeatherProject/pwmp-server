use crate::{
    cli::OtaCommand,
    error::Error,
    server::{
        config::Config,
        db::{DatabaseBackend, DatabaseClient},
    },
};
use std::process::exit;
use tracing::error;

pub async fn run(command: OtaCommand, config: &Config) -> Result<(), Error> {
    let client = DatabaseClient::new(config).await?;

    match command {
        OtaCommand::List => {
            let firmwares = client.get_firmwares().await?;
            let mut total_size = 0;

            for entry in firmwares {
                print!(
                    "#{}: {}, {} bytes, {}, ",
                    entry.id, entry.version, entry.size, entry.added
                );

                match entry.restrict {
                    None => println!("public"),
                    Some(nodes) if nodes.is_empty() => println!("internal"),
                    Some(nodes) => println!("restricted ({nodes:?})"),
                }

                total_size += entry.size;
            }

            println!("Total size: {total_size} bytes");
        }
        OtaCommand::Download { id } => {}
        OtaCommand::Push {
            blob,
            version,
            restrict,
        } => todo!(),
    }

    Ok(())
}
