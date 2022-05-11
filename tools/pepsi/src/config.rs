use std::path::Path;

use anyhow::anyhow;
use serde_json::Value;
use tokio::{fs::File, io::AsyncReadExt};

pub async fn head_name_from_id(head_id: &str, project_root: &Path) -> anyhow::Result<String> {
    let id_map_json = project_root
        .join("etc/configuration/location/default/")
        .join("id_map.json");
    let id_map: Value = {
        let mut file = File::open(&id_map_json).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        serde_json::from_slice(&contents)?
    };
    match id_map["idmap.nao"]
        .as_array()
        .ok_or_else(|| anyhow!("Expected array in id_map.json"))?
        .iter()
        .find(|entry| entry["headid"].as_str().unwrap() == head_id)
    {
        Some(entry) => Ok(entry["name"].as_str().unwrap().to_string()),
        None => Err(anyhow!("cannot find head_id in id_map.json")),
    }
}

pub async fn body_name_from_id(body_id: &str, project_root: &Path) -> anyhow::Result<String> {
    let id_map_json = project_root
        .join("etc/configuration/location/default/")
        .join("id_map.json");
    let id_map: Value = {
        let mut file = File::open(&id_map_json).await?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        serde_json::from_slice(&contents)?
    };
    match id_map["idmap.nao"]
        .as_array()
        .ok_or_else(|| anyhow!("Expected array in id_map.json"))?
        .iter()
        .find(|entry| entry["bodyid"].as_str().unwrap() == body_id)
    {
        Some(entry) => Ok(entry["name"].as_str().unwrap().to_string()),
        None => Err(anyhow!("cannot find body_id in id_map.json")),
    }
}
