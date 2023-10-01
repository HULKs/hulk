use std::{collections::HashSet, path::PathBuf};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub communication_addresses: Option<String>,
    pub cycler_instances_to_be_recorded: HashSet<String>,
    pub hardware_parameters: PathBuf,
    pub parameters_directory: PathBuf,
}
