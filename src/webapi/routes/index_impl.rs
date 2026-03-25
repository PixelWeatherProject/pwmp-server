use actix_web::{get, web};
use serde::Serialize;
use tokio::runtime::Handle;
use tracing::error;

#[derive(Serialize)]
struct Response {
    version: &'static str,
    async_rt_global_queue_depth: Option<usize>,
    async_rt_num_alive_tasks: Option<usize>,
    async_rt_num_workers: Option<usize>,
}

#[get("/")]
pub async fn index() -> web::Json<Response> {
    let mut response = Response {
        version: env!("CARGO_PKG_VERSION"),
        async_rt_global_queue_depth: None,
        async_rt_num_alive_tasks: None,
        async_rt_num_workers: None,
    };

    match Handle::try_current().as_ref().map(Handle::metrics) {
        Ok(metrics) => {
            response.async_rt_global_queue_depth = Some(metrics.global_queue_depth());
            response.async_rt_num_alive_tasks = Some(metrics.num_alive_tasks());
            response.async_rt_num_workers = Some(metrics.num_workers());
        }
        Err(e) => {
            error!("Runtime metrics are not available: {e}");
        }
    }

    web::Json(response)
}
