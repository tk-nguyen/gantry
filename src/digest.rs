use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Digest {
    pub digest: String,
}
