use std::path::PathBuf;

use pepsi::{
    commands::hulk::{hulk_service, Command},
    logging::apply_stdout_logging,
    util::{block_on_tasks, spawn_task_per_element},
    NaoAddress,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub struct Arguments {
    /// the systemctl command to execute
    #[structopt(possible_values = &["stop", "start", "restart", "enable", "disable"])]
    command: Command,
    /// the NAOs to execute that command on
    #[structopt(required = true)]
    naos: Vec<NaoAddress>,
}

pub fn hulk(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    let command = arguments.command;
    let tasks = spawn_task_per_element(&runtime, arguments.naos, |nao| {
        hulk_service(nao.ip, command, project_root.clone())
    });
    block_on_tasks(&runtime, tasks)?;
    Ok(())
}
