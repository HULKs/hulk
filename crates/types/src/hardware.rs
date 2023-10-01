use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Ids {
    pub body_id: String,
    pub head_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Paths {
    pub motions: PathBuf,
    pub neural_networks: PathBuf,
    pub sounds: PathBuf,
}
