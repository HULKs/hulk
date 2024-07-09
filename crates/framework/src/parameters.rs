use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub communication_addresses: String,
    pub recording_intervals: HashMap<String, usize>,
    pub hardware_parameters: PathBuf,
    pub parameters_directory: PathBuf,
}
