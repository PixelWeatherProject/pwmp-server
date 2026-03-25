use crate::server::db::{DatabaseBackend, DatabaseClient, DeviceDescriptor};
use actix_web::{Result, get, web};

#[get("/devices")]
pub async fn get_devices(
    db: web::Data<DatabaseClient>,
) -> Result<web::Json<Box<[DeviceDescriptor]>>> {
    Ok(web::Json(db.devices().await?))
}
