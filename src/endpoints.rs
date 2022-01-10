use actix_web::{
    get, head, patch, post, put,
    web::{BytesMut, Json, Path, Payload, Query},
    HttpResponse, Responder,
};
use futures_util::StreamExt;
use rusty_ulid::generate_ulid_string;
use sha2::{Digest, Sha256};
use tracing::info;

use std::{fs, fs::OpenOptions, io::Write};

use crate::manifest::*;

#[get("/")]
pub async fn version() -> impl Responder {
    HttpResponse::Ok()
        .header("Docker-Distribution-API-Version", "registry/2.0")
        .finish()
}

#[head("/{name}/blobs/{digest}")]
pub async fn check_blob(path: Path<(String, String)>) -> impl Responder {
    let path = path.into_inner();
    let location = format!("./images/{}", path.0);
    let metadata = fs::metadata(location.clone()).unwrap();
    let file = fs::read(location).unwrap();
    let digest = format!("{:x}", Sha256::digest(file));
    match digest == path.1.split(":").last().unwrap() {
        true => HttpResponse::Ok()
            .header("Content-Length", metadata.len())
            .header("Docker-Content-Digest", digest)
            .finish(),
        false => HttpResponse::Created()
            .header("Content-Length", 0.to_string())
            .finish(),
    }
}

#[post("/{name}/blobs/uploads")]
pub async fn start_upload(path: Path<(String,)>) -> impl Responder {
    let path = path.into_inner();
    HttpResponse::Accepted()
        .header(
            "Location",
            format!(
                "/v2/{name}/blobs/uploads/{ulid}",
                name = path.0,
                ulid = generate_ulid_string()
            ),
        )
        .header("Range", "bytes=0-0")
        .header("Content-Length", 0.to_string())
        .finish()
}

#[patch("/{name}/blobs/uploads/{ulid}")]
pub async fn write_image(path: Path<(String, String)>, mut data: Payload) -> impl Responder {
    let path = path.into_inner();
    let mut image = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("./images/{name}", name = path.0))
        .unwrap();
    let mut bytes = BytesMut::new();
    while let Some(d) = data.next().await {
        bytes.extend_from_slice(&d.unwrap());
    }
    match image.write(&bytes) {
        Ok(_) => HttpResponse::Accepted()
            .header(
                "Location",
                format!(
                    "/v2/{name}/blobs/uploads/{ulid}",
                    name = path.0,
                    ulid = path.1
                ),
            )
            .header("Range", format!("0-{}", bytes.len()))
            .header("Content-Length", 0.to_string())
            .finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[put("/{name}/blobs/uploads/{ulid}")]
pub async fn finish_upload(path: Path<(String, String)>, mut data: Payload) -> impl Responder {
    let path = path.into_inner();
    let mut image = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("./images/{name}", name = path.0))
        .unwrap();
    let mut bytes = BytesMut::new();
    while let Some(d) = data.next().await {
        bytes.extend_from_slice(&d.unwrap());
    }
    match image.write(&bytes) {
        Ok(_) => {
            let content = fs::read(format!("./images/{}", path.0)).unwrap();
            let digest = format!("{:x}", Sha256::digest(content));
            HttpResponse::Created()
                .header(
                    "Location",
                    format!(
                        "/v2/{name}/blobs/{digest}",
                        name = path.0,
                        digest = digest.clone()
                    ),
                )
                .header("Content-Length", 0.to_string())
                // .header("Docker-Content-Digest", digest)
                .finish()
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[put("/{name}/manifests/{reference}")]
pub async fn write_manifest(
    manifest: Json<ImageManifest>,
    path: Path<(String, String)>,
) -> impl Responder {
    let path = path.into_inner();
    let location = format!("./images/{}", path.0);
    let metadata = fs::metadata(location.clone()).unwrap();
    let content = fs::read(location).unwrap();
    let digest = format!("{:x}", Sha256::digest(content));
    let response = ImageManifest {
        schema_version: 2,
        media_type: manifest.media_type.clone(),
        config: Config {
            media_type: manifest.config.media_type.clone(),
            size: Some(metadata.len() as usize),
            digest: format!("sha256:{}", digest),
        },
        layers: vec![Layer {
            media_type: manifest.layers[0].media_type.clone(),
            size: Some(metadata.len() as usize),
            digest: format!("sha256:{}", digest),
            urls: None,
        }],
    };
    info!("{:#?}", response);
    Json(response)
}
