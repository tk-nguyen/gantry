use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Config {
    media_type: String,
    size: usize,
    digest: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Layer {
    media_type: String,
    size: usize,
    digest: String,
    urls: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ImageManifest {
    schema_version: u8,
    media_type: String,
    config: Config,
    layers: Vec<Layer>,
}
