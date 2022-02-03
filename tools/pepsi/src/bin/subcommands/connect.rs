use std::{path::PathBuf, process::Command};

use log::{error, info};
use pepsi::{logging::apply_stdout_logging, NaoAddress};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Arguments {
    /// The nao to connect to
    nao: NaoAddress,
}

pub fn connect(
    arguments: Arguments,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    info!("Connecting to {}", arguments.nao);
    let mut process = Command::new("ssh")
        .arg("-oUserKnownHostsFile=/dev/null")
        .arg("-oStrictHostKeyChecking=no")
        .arg("-oLogLevel=quiet")
        .arg("-lnao")
        .arg(format!(
            "-i{}",
            project_root.join("scripts/ssh_key").to_str().unwrap()
        ))
        .arg(arguments.nao.to_string())
        .spawn()
        .expect("failed to spawn ssh process");
    let exit_status = process.wait()?;
    if !exit_status.success() {
        error!("ssh exited with {}", exit_status);
    }
    Ok(())
}
