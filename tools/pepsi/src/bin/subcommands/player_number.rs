use std::{path::PathBuf, str::FromStr};

use pepsi::{
    commands::player_number::configure_player_number,
    logging::apply_stdout_logging,
    util::{block_on_tasks, number_to_headname, spawn_task_per_element},
    NaoName, PlayerNumber,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

struct PlayerAssignment {
    head_name: NaoName,
    player_number: PlayerNumber,
}

impl FromStr for PlayerAssignment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (nao, player_number) = s
            .split_once(":")
            .ok_or_else(|| anyhow::anyhow!("cannot split assignment on ':'"))?;
        Ok(PlayerAssignment {
            head_name: number_to_headname(nao.parse()?),
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
    let tasks = spawn_task_per_element(&runtime, arguments.assignments, |assignment| {
        configure_player_number(
            assignment.head_name,
            assignment.player_number,
            project_root.clone(),
        )
    });
    block_on_tasks(&runtime, tasks)?;
    Ok(())
}
