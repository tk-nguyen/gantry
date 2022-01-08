use actix_web::{get, web::Bytes, HttpResponse, Responder};
use tracing::info;

#[get("/v2")]
pub async fn version() -> impl Responder {
    HttpResponse::Ok()
        .header("Docker-Distribution-API-Version", "registry/2.0")
        .finish()
}

pub async fn request_debug(req: Bytes) -> impl Responder {
    let request = String::from_utf8(req.to_vec()).unwrap();
    info!("Request body: {}", request);
    "Ok"
}
