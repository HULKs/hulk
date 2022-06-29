use std::path::PathBuf;

use anyhow::Context;
use serde_json::{from_slice, to_vec_pretty, Value};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::PlayerNumber;

pub async fn configure_player_number(
    head_id: String,
    player_number: PlayerNumber,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    let configuration_file_path =
        project_root.join(format!("etc/configuration/head.{}.json", head_id));
    let mut configuration_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&configuration_file_path)
        .await
        .with_context(|| format!("Failed to open {}", configuration_file_path.display()))?;

    let mut contents = vec![];
    configuration_file
        .read_to_end(&mut contents)
        .await
        .with_context(|| format!("Failed to read from {}", configuration_file_path.display()))?;
    let mut configuration: Value = if contents.is_empty() {
        Value::Object(Default::default())
    } else {
        from_slice(&contents)
            .with_context(|| format!("Failed to parse {}", configuration_file_path.display()))?
    };

    configuration["player_number"] = match player_number {
        1 => "One".into(),
        2 => "Two".into(),
        3 => "Three".into(),
        4 => "Four".into(),
        5 => "Five".into(),
        _ => panic!("Unexpected player number"),
    };

    let mut contents = to_vec_pretty(&configuration).with_context(|| {
        format!(
            "Failed to dump configuration for {}",
            configuration_file_path.display()
        )
    })?;
    contents.push(b'\n');
    let mut configuration_file = File::create(&configuration_file_path)
        .await
        .with_context(|| format!("Failed to create {}", configuration_file_path.display()))?;
    configuration_file
        .write_all(&contents)
        .await
        .with_context(|| format!("Failed to parse {}", configuration_file_path.display()))?;

    Ok(())
}
