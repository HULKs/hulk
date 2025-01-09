use std::{path::Path, process::Command};

use clap::Args;
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use repository::{
    cargo::{Cargo, Environment},
    configuration::read_sdk_version,
};

use crate::CargoArguments;

pub mod build;
pub mod check;
pub mod clippy;
pub mod common;
pub mod environment;
pub mod run;
mod heading {
    pub const PACKAGE_SELECTION: &str = "Package Selection";
    pub const TARGET_SELECTION: &str = "Target Selection";
    pub const FEATURE_SELECTION: &str = "Feature Selection";
    pub const COMPILATION_OPTIONS: &str = "Compilation Options";
    pub const MANIFEST_OPTIONS: &str = "Manifest Options";
}

pub trait CargoCommand {
    fn apply(&self, cmd: &mut Command);
}

pub async fn cargo<Arguments: Args + CargoCommand>(
    arguments: CargoArguments<Arguments>,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let environment = arguments
        .environment
        .env
        .resolve(&repository_root)
        .await
        .wrap_err("failed to resolve enviroment")?;

    let cargo = if arguments.environment.remote {
        Cargo::remote(environment)
    } else {
        Cargo::local(environment)
    };
    let mut cargo_command = cargo
        .command(&repository_root)
        .wrap_err("failed to create cargo command")?;

    // TODO: Build extension trait for readability
    arguments.cargo.apply(&mut cargo_command);

    let status = tokio::process::Command::from(cargo_command)
        .status()
        .await
        .wrap_err("failed to run cargo build")?;

    if !status.success() {
        bail!("pepsi build failed with {status}");
    }

    Ok(())
}
