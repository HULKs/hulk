use std::{path::Path, process::Command};

use color_eyre::{eyre::Context, Result};
use environment::EnvironmentArguments;
use repository::{
    cargo::{Cargo, Environment},
    configuration::read_sdk_version,
};

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

async fn cargo(
    environment_arguments: EnvironmentArguments,
    repository_root: impl AsRef<Path>,
) -> Result<Command> {
    let repository_root = repository_root.as_ref();

    let sdk_version = read_sdk_version(repository_root)
        .await
        .wrap_err("failed to read SDK version")?;

    let environment = match environment_arguments.env {
        Some(environment) => environment,
        None => Environment::Native,
    };
    let cargo = if environment_arguments.remote {
        Cargo::remote(environment)
    } else {
        Cargo::local(environment)
    };

    cargo
        .command(repository_root)
        .wrap_err("failed to create cargo command")
}
