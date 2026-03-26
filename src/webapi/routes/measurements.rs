use crate::server::db::{CompleteMeasurement, DatabaseBackend, DatabaseClient, NodeId};
use actix_web::{Result, get, web};

#[get("/measurements/{node}")]
pub async fn get_node_measurements(
    db: web::Data<DatabaseClient>,
    node: web::Path<(NodeId,)>,
) -> Result<web::Json<Box<[CompleteMeasurement]>>> {
    Ok(web::Json(db.node_measurements(node.into_inner().0).await?))
}
