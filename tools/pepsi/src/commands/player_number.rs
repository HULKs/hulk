use std::path::PathBuf;

use anyhow::Context;
use log::info;
use serde_json::Value;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{NaoName, PlayerNumber};

pub async fn get_configured_player_number(
    head_name: &str,
    project_root: PathBuf,
) -> anyhow::Result<PlayerNumber> {
    let brain_default_config = project_root
        .join("etc/configuration/location/default/head/")
        .join(head_name)
        .join("Brain.json");
    let brain_configuration: Value = {
        let mut file = File::open(brain_default_config).await.with_context(|| {
            format!(
                "Failed to open default head config for head name '{}'",
                head_name
            )
        })?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        serde_json::from_slice(&contents)?
    };
    brain_configuration["general.playerNumber"]
        .as_u64()
        .map(|n| n as PlayerNumber)
        .ok_or_else(|| anyhow::anyhow!("No key for playernumber"))
}

pub async fn configure_player_number(
    head_name: NaoName,
    player_number: PlayerNumber,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    let brain_default_config = project_root
        .join("etc/configuration/location/default/head/")
        .join(&head_name)
        .join("Brain.json");

    let mut brain_configuration: Value = {
        let mut file = File::open(&brain_default_config).await.with_context(|| {
            format!(
                "Failed to open default head config for head name '{}'",
                head_name
            )
        })?;
        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        serde_json::from_slice(&contents)?
    };
    let previous_player_number = brain_configuration["general.playerNumber"]
        .as_u64()
        .unwrap();
    brain_configuration["general.playerNumber"] = Value::from(player_number);
    {
        let mut file = File::create(brain_default_config).await.with_context(|| {
            format!(
                "Failed to create default head config for head name '{}'",
                head_name
            )
        })?;
        let serialized_buffer = serde_json::to_vec_pretty(&brain_configuration)?;
        file.write(&serialized_buffer).await?;
    }
    info!(
        "Changed player number of {} from {} to {}",
        head_name, previous_player_number, player_number
    );
    Ok(())
}
