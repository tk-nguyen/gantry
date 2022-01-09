use actix_web::{
    get, head, patch, post, put,
    web::{BytesMut, Path, Payload, Query},
    HttpResponse, Responder,
};
use futures_util::StreamExt;
use rusty_ulid::generate_ulid_string;
use sha256::digest_bytes;
use tracing::info;

use std::{fs, fs::OpenOptions, io::Write};

use crate::digest::Digest;

#[get("/")]
pub async fn version() -> impl Responder {
    HttpResponse::Ok()
        .header("Docker-Distribution-API-Version", "registry/2.0")
        .finish()
}

#[head("/{name}/blobs/{digest}")]
pub async fn check_blob(path: Path<(String, String)>) -> impl Responder {
    let path = path.into_inner();
    let digest = path.1.split(':').last().unwrap();
    let mappings = fs::read_to_string("mappings.txt").unwrap();
    let mappings = mappings.split(":").collect::<Vec<&str>>();
    match mappings[1] == digest {
        true => {
            let metadata = fs::metadata(format!("./images/{}", mappings[0])).unwrap();
            HttpResponse::Ok()
                .header("Content-Length", metadata.len())
                .header("Docker-Content-Digest", digest)
                .finish()
        }
        false => HttpResponse::NotFound().finish(),
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
pub async fn finish_upload(
    path: Path<(String, String)>,
    mut data: Payload,
    query: Query<Digest>,
) -> impl Responder {
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
            let digest = digest_bytes(&content);
            let mappings = format!("{}:{}", path.0, digest);
            fs::write("mappings.txt", mappings).unwrap();
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
