use std::{collections::HashMap, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Context};
use pepsi::{
    commands::player_number::configure_player_number,
    logging::apply_stdout_logging,
    util::{block_on_tasks, spawn_task_per_element},
    NaoNumber, PlayerNumber,
};
use serde::Deserialize;
use serde_json::from_slice;
use structopt::StructOpt;
use tokio::{fs::File, io::AsyncReadExt, runtime::Runtime};

struct PlayerAssignment {
    nao_number: NaoNumber,
    player_number: PlayerNumber,
}

impl FromStr for PlayerAssignment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (nao, player_number) = s
            .split_once(':')
            .ok_or_else(|| anyhow!("cannot split assignment on ':'"))?;
        Ok(PlayerAssignment {
            nao_number: nao.parse()?,
            player_number: player_number.parse()?,
        })
    }
}

#[derive(StructOpt)]
pub struct Arguments {
    /// the assignments to change (e.g. '20:2')
    #[structopt(required = true)]
    assignments: Vec<PlayerAssignment>,
}

pub fn player_number(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    let hardware_ids = runtime
        .block_on(get_hardware_ids(project_root.clone()))
        .context("Failed to get hardware IDs")?;
    let tasks = spawn_task_per_element(&runtime, arguments.assignments, |assignment| {
        configure_player_number(
            hardware_ids[&assignment.nao_number].head_id.clone(),
            assignment.player_number,
            project_root.clone(),
        )
    });
    block_on_tasks(&runtime, tasks)?;
    Ok(())
}

#[derive(Deserialize)]
struct HardwareIds {
    #[allow(dead_code)]
    body_id: String,
    head_id: String,
}

async fn get_hardware_ids(
    project_root: PathBuf,
) -> anyhow::Result<HashMap<NaoNumber, HardwareIds>> {
    let hardware_ids_path = project_root.join("tools/pepsi/hardware_ids.json");
    let mut hardware_ids = File::open(&hardware_ids_path)
        .await
        .with_context(|| format!("Failed to open {}", hardware_ids_path.display()))?;
    let mut contents = vec![];
    hardware_ids.read_to_end(&mut contents).await?;
    let hardware_ids_with_string_keys: HashMap<String, HardwareIds> = from_slice(&contents)?;
    let hardware_ids_with_nao_number_keys = hardware_ids_with_string_keys
        .into_iter()
        .map(|(nao_number, hardware_ids)| {
            Ok((
                nao_number
                    .parse()
                    .with_context(|| format!("Failed to parse NAO number: {:?}", nao_number))?,
                hardware_ids,
            ))
        })
        .collect::<anyhow::Result<HashMap<_, _>>>()?;
    Ok(hardware_ids_with_nao_number_keys)
}
