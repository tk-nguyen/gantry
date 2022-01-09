use color_eyre::eyre::Result;
use tracing_subscriber;

use std::env;

use actix_web::{middleware, web, App, HttpServer};

mod digest;
mod endpoints;
mod manifest;
use endpoints::*;

#[actix_web::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    if let Err(_) = env::var("RUST_LOG") {
        env::set_var("RUST_LOG", "info");
    }
    tracing_subscriber::fmt::init();
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::new(
                middleware::normalize::TrailingSlash::Trim,
            ))
            .service(
                web::scope("/v2")
                    .service(version)
                    .service(check_blob)
                    .service(start_upload)
                    .service(write_image)
                    .service(finish_upload),
            )
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await?;

    Ok(())
}
