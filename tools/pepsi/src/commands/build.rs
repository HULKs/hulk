use std::{path::PathBuf, process::ExitStatus, string::String};

use anyhow::Context;
use structopt::clap::arg_enum;
use tokio::process::Command;

arg_enum! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub enum BuildType {
        Release,
        Debug,
    }
}

impl Default for BuildType {
    fn default() -> Self {
        Self::Release
    }
}

arg_enum! {
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub enum Target {
        NAO,
        Webots,
    }
}

impl Default for Target {
    fn default() -> Self {
        Self::Webots
    }
}

pub async fn build(
    project_root: PathBuf,
    build_type: BuildType,
    target: Target,
    is_verbose: bool,
) -> anyhow::Result<ExitStatus> {
    let mut command = Command::new("bash");

    let mut command_string = String::new();

    if target == Target::NAO {
        command_string += format!(
            ". {:?} && ",
            project_root.join("sdk/current/environment-setup-corei7-64-aldebaran-linux")
        )
        .as_str();
        command.env("NAO_CARGO_HOME", project_root.join(".nao_cargo_home"));
    }

    command_string += "cargo build";

    if build_type == BuildType::Release {
        command_string += " --release";
    }
    command_string += match target {
        Target::NAO => " --features \"nao\" --bin nao",
        Target::Webots => " --features \"webots\" --bin webots",
    };
    if is_verbose {
        command_string += " --verbose";
    }
    command.args(&["-c", &command_string]);

    let mut child = command.spawn().context("Build command failed")?;
    Ok(child.wait().await?)
}
