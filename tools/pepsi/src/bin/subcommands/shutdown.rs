use std::path::PathBuf;

use pepsi::{
    commands,
    logging::apply_stdout_logging,
    util::{block_on_tasks, spawn_task_per_element},
    NaoAddress,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub struct Arguments {
    /// whether the nao should reboot
    #[structopt(long, short)]
    reboot: bool,
    /// the naos to execute that command on
    #[structopt(required = true)]
    naos: Vec<NaoAddress>,
}

pub fn shutdown(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    let reboot = arguments.reboot;
    let tasks = spawn_task_per_element(&runtime, arguments.naos, |nao| {
        commands::shutdown::shutdown(nao.ip, reboot, project_root.clone())
    });
    block_on_tasks(&runtime, tasks)?;
    Ok(())
}
