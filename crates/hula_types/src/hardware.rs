use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(Clone, Debug, Serialize, PathSerialize, Deserialize, PathDeserialize, PathIntrospect)]
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
