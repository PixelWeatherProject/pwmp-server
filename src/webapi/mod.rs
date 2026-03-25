use crate::{config::Config, server::db::DatabaseClient};
use actix_web::{App, HttpServer, web};
use std::{process::exit, sync::Arc};
use tracing::{error, info};

mod routes;

pub async fn start(config: &Config) -> std::io::Result<()> {
    let config: Arc<Config> = Arc::new(config.clone());

    info!("Connecting to database at \"{}\"", config.database.host());
    let db = match DatabaseClient::new(&config).await {
        Ok(db) => db,
        Err(why) => {
            error!("Failed to connect to database: {why}");
            exit(1);
        }
    };

    let shared_db = web::Data::new(db);

    tokio::task::spawn_blocking(move || {
        actix_web::rt::System::new().block_on(async {
            HttpServer::new(move || {
                App::new()
                    .app_data(shared_db.clone())
                    .service(routes::index)
            })
            .bind((config.webapi.ip, config.webapi.port))?
            .run()
            .await
        })
    })
    .await
    .unwrap()
}
