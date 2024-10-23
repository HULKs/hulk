use std::{collections::HashMap, path::Path};

use color_eyre::{eyre::Context, Result};
use tokio::fs::read_to_string;
use types::hardware::Ids;

pub async fn get_hardware_ids(repository_root: impl AsRef<Path>) -> Result<HashMap<u8, Ids>> {
    let hardware_ids_path = repository_root
        .as_ref()
        .join("etc/parameters/hardware_ids.json");

    let content = read_to_string(&hardware_ids_path)
        .await
        .wrap_err_with(|| format!("failed to open {}", hardware_ids_path.display()))?;

    let id_map = serde_json::from_str(&content).wrap_err("failed to parse hardware IDs")?;
    Ok(id_map)
}
