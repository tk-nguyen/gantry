use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub media_type: String,
    pub size: Option<usize>,
    pub digest: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Layer {
    pub media_type: String,
    pub size: Option<usize>,
    pub digest: String,
    pub urls: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImageManifest {
    pub schema_version: u8,
    pub media_type: String,
    pub config: Config,
    pub layers: Vec<Layer>,
}
