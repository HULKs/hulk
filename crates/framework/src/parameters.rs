use std::{collections::HashSet, path::PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub communication_addresses: Option<String>,
    pub recording_settings: HashSet<String>,
    pub hardware_parameters: PathBuf,
    pub parameters_directory: PathBuf,
}
