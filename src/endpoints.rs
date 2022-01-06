use actix_web::{get, HttpResponse, Responder};

#[get("/v2")]
pub async fn version() -> impl Responder {
    HttpResponse::Ok()
        .header("Docker-Distribution-API-Version", "registry/2.0")
        .finish()
}
