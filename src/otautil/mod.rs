use crate::{
    cli::OtaCommand,
    error::Error,
    server::{
        config::Config,
        db::{DatabaseBackend, DatabaseClient},
    },
};
use pwmp_client::pwmp_msg::version::Version;
use std::process::exit;
use tokio::fs;
use tracing::{error, info};

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
        OtaCommand::Pull { id, output } => {
            let firmwares = client.get_firmwares().await?;
            let selection = firmwares.iter().find(|candidate| candidate.id == id);

            match selection {
                Some(entry) => {
                    fs::write(output, &entry.blob).await?;
                    info!("Successfully pulled");
                }
                None => {
                    error!("No firmware with ID {id}");
                }
            }
        }
        OtaCommand::Push {
            blob,
            version,
            restrict,
        } => {
            let blob = fs::read(blob).await?;
            let Some(version) = Version::parse(&version) else {
                error!("Invalid semantic version string '{version}'");
                exit(1);
            };

            client.upload_firmware(blob, version, restrict).await?;
            info!("Successfully pushed");
        }
    }

    Ok(())
}
