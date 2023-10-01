use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub communication_addresses: Option<String>,
    pub hardware_parameters: PathBuf,
    pub parameters_directory: PathBuf,
}
