use std::path::PathBuf;

use pepsi::{
    commands::wlan::{connect_wireless, disconnect_wireless, get_networks, get_wireless},
    logging::apply_stdout_logging,
    util::{block_on_tasks, spawn_task_per_element},
    NaoAddress,
};
use structopt::StructOpt;
use tokio::runtime::Runtime;

#[derive(StructOpt)]
pub enum Arguments {
    Show {
        /// the naos to execute that command on
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
    GetNetworks {
        /// the naos to execute that command on
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
    Connect {
        /// the ssid to connect the wireless device to
        ssid: String,
        /// the passphrase of the network
        passphrase: Option<String>,
        /// the naos to execute that command on
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
    Disconnect {
        /// the naos to execute that command on
        #[structopt(required = true)]
        naos: Vec<NaoAddress>,
    },
}

pub fn wlan(
    arguments: Arguments,
    runtime: Runtime,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    match arguments {
        Arguments::Show { naos } => {
            let tasks = spawn_task_per_element(&runtime, naos, |nao| {
                get_wireless(nao.ip, project_root.clone())
            });
            let outputs = block_on_tasks(&runtime, tasks)?;
            for output in outputs {
                println!("{}", output);
            }
        }
        Arguments::GetNetworks { naos } => {
            let tasks = spawn_task_per_element(&runtime, naos, |nao| {
                get_networks(nao.ip, project_root.clone())
            });
            let outputs = block_on_tasks(&runtime, tasks)?;
            for output in outputs {
                println!("{}", output);
            }
        }
        Arguments::Connect {
            ssid,
            passphrase,
            naos,
        } => {
            let tasks = spawn_task_per_element(&runtime, naos, |nao| {
                connect_wireless(
                    nao.ip,
                    ssid.to_string(),
                    passphrase.clone(),
                    project_root.clone(),
                )
            });
            block_on_tasks(&runtime, tasks)?;
        }
        Arguments::Disconnect { naos } => {
            let tasks = spawn_task_per_element(&runtime, naos, |nao| {
                disconnect_wireless(nao.ip, project_root.clone())
            });
            block_on_tasks(&runtime, tasks)?;
        }
    };
    Ok(())
}
