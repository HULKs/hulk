use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ids {
    pub robot_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Paths {
    pub motions: PathBuf,
    pub neural_networks: PathBuf,
    pub sounds: PathBuf,
    pub cache: PathBuf,
}
