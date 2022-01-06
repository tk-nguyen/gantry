use color_eyre::eyre::Result;
use tracing_subscriber;

use std::env;

use actix_web::{middleware, App, HttpServer};

mod endpoints;
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
            .service(version)
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await?;

    Ok(())
}
