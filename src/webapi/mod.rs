use crate::{config::Config, server::db::DatabaseClient};
use actix_web::{
    App, HttpServer,
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorForbidden,
    http::header::AUTHORIZATION,
    middleware::Next,
    web,
};
use std::process::exit;
use tracing::{error, info};

mod routes;

pub async fn start(config: &Config) -> std::io::Result<()> {
    let config = config.clone();

    info!("Connecting to database at \"{}\"", config.database.host());
    let db = match DatabaseClient::new(&config).await {
        Ok(db) => db,
        Err(why) => {
            error!("Failed to connect to database: {why}");
            exit(1);
        }
    };

    let shared_config = web::Data::new(config.clone());
    let shared_db = web::Data::new(db);

    tokio::task::spawn_blocking(move || {
        actix_web::rt::System::new().block_on(async {
            HttpServer::new(move || {
                App::new()
                    .app_data(shared_config.clone())
                    .app_data(shared_db.clone())
                    .wrap(actix_web::middleware::from_fn(auth_middleware))
                    .service(routes::index)
                    .service(routes::get_devices)
                    .service(routes::get_node_measurements)
            })
            .bind((config.webapi.ip, config.webapi.port))?
            .run()
            .await
        })
    })
    .await
    .unwrap()
}

async fn auth_middleware(
    req: ServiceRequest,
    next: Next<actix_web::body::BoxBody>,
) -> actix_web::Result<ServiceResponse<actix_web::body::BoxBody>> {
    let config = req.app_data::<web::Data<Config>>().unwrap();

    let Some((_, auth_header)) = req
        .headers()
        .iter()
        .find(|(name, _)| *name == AUTHORIZATION)
    else {
        return Err(ErrorForbidden("Missing auth token"));
    };

    let Some(token) = auth_header
        .to_str()
        .unwrap()
        .split_once(' ')
        .map(|res| res.1)
    else {
        return Err(ErrorForbidden("Invalid auth token format"));
    };

    if token != config.webapi.auth_key.as_ref() {
        return Err(ErrorForbidden("Forbidden"));
    }

    next.call(req).await
}
