use actix_web::{
    get, head, patch, post, put,
    web::{BytesMut, Json, Path, Payload, Query},
    HttpResponse, Responder,
};
use futures_util::StreamExt;
use rusty_ulid::generate_ulid_string;
use serde_json;
use sha2::{Digest, Sha256};
use tracing::info;

use std::{fs, fs::File, fs::OpenOptions, io::Write};

use crate::image_digest::ImageDigest;
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
    let location = format!(
        "./images/{name}/{digest}",
        name = path.0,
        digest = path.1.split(":").last().unwrap()
    );
    match fs::metadata(location.clone()) {
        Ok(metadata) => {
            let file = fs::read(location).unwrap();
            let digest = format!("{:x}", Sha256::digest(file));
            HttpResponse::Ok()
                .header("Content-Length", metadata.len())
                .header("Docker-Content-Digest", digest)
                .finish()
        }
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[post("/{name}/blobs/uploads")]
pub async fn start_upload(path: Path<(String,)>) -> impl Responder {
    let path = path.into_inner();
    let directory = path.0.as_str();
    let mut uploaded_images = fs::read_dir("./images").unwrap();
    if let None = uploaded_images.find(|d| d.as_ref().unwrap().file_name() == directory) {
        fs::create_dir(format!("./images/{}", path.0)).unwrap();
    }
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
        .open(format!(
            "./images/{name}/{ulid}",
            name = path.0,
            ulid = path.1
        ))
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
    query: Query<ImageDigest>,
) -> impl Responder {
    let path = path.into_inner();
    let orig = format!("./images/{name}/{ulid}", name = path.0, ulid = path.1);
    let dest = format!(
        "./images/{name}/{digest}",
        name = path.0,
        digest = query.digest.split(":").last().unwrap()
    );
    fs::copy(orig.clone(), dest.clone()).unwrap();
    let mut image = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dest.clone())
        .unwrap();
    let mut bytes = BytesMut::new();
    while let Some(d) = data.next().await {
        bytes.extend_from_slice(&d.unwrap());
    }
    match image.write(&bytes) {
        Ok(_) => {
            let content = fs::read(dest).unwrap();
            let digest = format!("{:x}", Sha256::digest(content));
            let mut mappings = OpenOptions::new()
                .create(true)
                .append(true)
                .open("mappings.txt")
                .unwrap();
            mappings
                .write(
                    format!("{name}\t{digest}\n", name = path.0, digest = digest.clone())
                        .as_bytes(),
                )
                .unwrap();
            fs::remove_file(orig).unwrap();
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
                .header("Docker-Content-Digest", digest)
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
    match File::open(format!("./images/{}/manifest.json", path.0)) {
        Ok(f) => {
            let mut current_manifest: ImageManifest = serde_json::from_reader(f).unwrap();
            let received_manifest = manifest.into_inner();
            if let Some(_) = received_manifest.config.size {
                current_manifest.config = received_manifest.config;
            }
            if current_manifest.layers.len() < received_manifest.layers.len() {
                for i in (received_manifest.layers.len() - current_manifest.layers.len() + 1)
                    ..received_manifest.layers.len()
                {
                    let received_layer = &received_manifest.layers[i];
                    let added_layer = Layer {
                        media_type: received_layer.media_type.clone(),
                        size: received_layer.size,
                        digest: received_layer.digest.clone(),
                        urls: received_layer.urls.clone(),
                    };
                    current_manifest.layers.push(added_layer);
                }
            }
            let file = File::create(format!("./images/{}/manifest.json", path.0)).unwrap();
            serde_json::to_writer_pretty(file, &current_manifest).unwrap();
        }

        Err(_) => {
            let file = File::create(format!("./images/{}/manifest.json", path.0)).unwrap();
            serde_json::to_writer_pretty(file, &manifest.into_inner()).unwrap();
        }
    }
    let maps = fs::read_to_string("mappings.txt").unwrap();

    // TODO Use a better container (HashMap?)
    let mappings = maps
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .collect::<Vec<&str>>();
    HttpResponse::Created()
        .header(
            "Location",
            format!(
                "/v2/{name}/manifests/{reference}",
                name = path.0,
                reference = path.1,
            ),
        )
        .header("Content-Length", 0.to_string())
        .header("Docker-Content-Digest", format!("sha256:{}", mappings[1]))
        .finish()
}
