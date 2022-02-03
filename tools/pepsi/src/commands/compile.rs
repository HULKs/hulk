use std::{fmt::Display, path::PathBuf, process::ExitStatus, str::FromStr};

use anyhow::Context;
use tokio::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum BuildType {
    Release,
    RelWithDebInfo,
    Develop,
    DevWithDebInfo,
    Debug,
}

impl Default for BuildType {
    fn default() -> Self {
        Self::Develop
    }
}

impl Display for BuildType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, "{:?}", self)
    }
}

impl FromStr for BuildType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Release" => Ok(BuildType::Release),
            "RelWithDebInfo" => Ok(BuildType::RelWithDebInfo),
            "Develop" => Ok(BuildType::Develop),
            "DevWithDebInfo" => Ok(BuildType::DevWithDebInfo),
            "Debug" => Ok(BuildType::Debug),
            _ => Err(anyhow::anyhow!("cannot parse BuildType from string")),
        }
    }
}

pub async fn compile(project_root: PathBuf, build_type: BuildType) -> anyhow::Result<ExitStatus> {
    let mut command = Command::new(project_root.join("scripts/compile"));
    command.env_remove("LD_LIBRARY_PATH");
    command.arg("--build-type");
    command.arg(build_type.to_string());
    command.arg("--target");
    command.arg("NAO");

    // info!("Compiling for target NAO build-type {}", build_type);
    let mut child = command.spawn().context("compilation command failed")?;
    Ok(child.wait().await?)
}
