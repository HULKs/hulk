use std::{path::PathBuf, process::Command};

use anyhow::{bail, Context};
use pepsi::{logging::apply_stdout_logging, NaoAddress};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Arguments {
    /// The NAO to connect to
    nao: NaoAddress,
}

pub fn connect(
    arguments: Arguments,
    is_verbose: bool,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    apply_stdout_logging(is_verbose)?;
    let exit_status = Command::new("ssh")
        .arg("-oUserKnownHostsFile=/dev/null")
        .arg("-oStrictHostKeyChecking=no")
        .arg("-oLogLevel=quiet")
        .arg("-lnao")
        .arg(format!(
            "-i{}",
            project_root.join("scripts/ssh_key").to_str().unwrap()
        ))
        .arg(arguments.nao.to_string())
        .status()
        .context("Failed to spawn ssh process")?;
    if !exit_status.success() {
        bail!("ssh exited with {}", exit_status);
    }
    Ok(())
}
