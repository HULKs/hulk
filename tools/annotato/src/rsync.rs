use color_eyre::{
    eyre::{bail, ContextCompat},
    Result,
};
use std::{
    path::Path,
    process::{Command, Stdio},
};

use crate::user_toml::CONFIG;

fn full_dataset_path(dataset_name: &str) -> Result<String> {
    let config = CONFIG.get().wrap_err("could not find config file")?;

    Ok(format!("{}/{}", config.remote.rsync_path, dataset_name))
}

pub fn rsync_dataset_list() -> Result<Vec<String>> {
    let output = Command::new("rsync")
        .arg(full_dataset_path("")?)
        .arg("-o")
        .arg("ConnectTimeout=2")
        .arg("--list-only")
        .output()?;
    if !output.status.success() {
        bail!(String::from_utf8(output.stderr)?)
    }
    let output = String::from_utf8(output.stdout)?;
    Ok(output.lines().map(|line| line.to_owned()).collect())
}

pub fn rsync_to_local(local_folder: impl AsRef<Path>, dataset_name: &str) -> Result<()> {
    let output = Command::new("rsync")
        .arg("--timeout")
        .arg("2")
        .arg("--recursive")
        .arg("--info=progress2")
        .arg("--no-inc-recursive")
        .arg("--human-readable")
        .arg(full_dataset_path(dataset_name)?)
        .arg(local_folder.as_ref())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    if !output.status.success() {
        bail!(String::from_utf8(output.stderr)?)
    }

    Ok(())
}

pub fn rsync_to_host(local_folder: impl AsRef<Path>, dataset_name: &str) -> Result<()> {
    let output = Command::new("rsync")
        .arg("--timeout")
        .arg("2")
        .arg("--recursive")
        .arg("--info=progress2")
        .arg("--no-inc-recursive")
        .arg("--human-readable")
        .arg(local_folder.as_ref().join(dataset_name))
        .arg(full_dataset_path("")?)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()?;

    if !output.status.success() {
        bail!(String::from_utf8(output.stderr)?)
    }

    Ok(())
}
