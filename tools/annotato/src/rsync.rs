use color_eyre::Result;
use std::{
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::Command,
};

const USER_NAME: &str = "hulk";
const HOST: Ipv4Addr = Ipv4Addr::new(134, 28, 57, 225);
const DATASETS_FOLDER: &str = "/home/hulk/labelling/output";

fn full_dataset_path(dataset_name: &str) -> String {
    format!(
        "{USER_NAME}@{HOST}:{}",
        PathBuf::from(DATASETS_FOLDER)
            .join(dataset_name)
            .display()
            .to_string()
    )
}

pub fn rsync_dataset_list() -> Result<Vec<String>> {
    let output = Command::new("ssh")
        .arg(format!("{USER_NAME}@{HOST}"))
        .arg("--")
        .arg("ls")
        .arg(DATASETS_FOLDER)
        .output()?;
    let output = String::from_utf8(output.stdout)?;
    Ok(output.lines().map(|line| line.to_owned()).collect())
}

pub fn rsync_to_local(local_folder: impl AsRef<Path>, dataset_name: &str) -> Result<()> {
    Command::new("rsync")
        .arg("-r")
        .arg(full_dataset_path(dataset_name))
        .arg(local_folder.as_ref())
        .output()?;

    Ok(())
}

pub fn rsync_to_host(local_folder: impl AsRef<Path>, dataset_name: &str) -> Result<()> {
    Command::new("rsync")
        .arg("-r")
        .arg(local_folder.as_ref().join(dataset_name))
        .arg(full_dataset_path(dataset_name))
        .output()?;

    Ok(())
}
