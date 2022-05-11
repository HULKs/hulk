use std::path::PathBuf;

use pepsi::{
    commands::logs::{delete_logs, download_logs},
    logging::apply_stdout_logging,
    util::{block_on_tasks, spawn_task_per_element},
    NaoAddress,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub enum Arguments {
    /// delete logs on the NAO
    Delete {
        /// the naos to execute that command on
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
    /// download logs from the NAO
    Download {
        /// location where to download the logs to
        log_dir: PathBuf,
        /// the NAOs to execute that command on
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub fn logs(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    match arguments {
        Arguments::Delete { naos } => {
            let tasks = spawn_task_per_element(&runtime, naos, |nao| {
                delete_logs(nao.ip, project_root.clone())
            });
            block_on_tasks(&runtime, tasks)?;
        }
        Arguments::Download { log_dir, naos } => {
            let tasks = spawn_task_per_element(&runtime, naos, |nao| {
                download_logs(nao.ip, log_dir.clone(), project_root.clone())
            });
            block_on_tasks(&runtime, tasks)?;
        }
    }
    Ok(())
}
