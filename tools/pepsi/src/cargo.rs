use std::{
    collections::HashMap,
    env::current_dir,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf, absolute},
};

use clap::Args;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use environment::{Environment, EnvironmentArguments};
use lazy_static::lazy_static;
use pathdiff::diff_paths;
use repository::Repository;
use tokio::{fs::read_to_string, process::Command};
use toml::Table;
use tracing::debug;

use crate::sdk::{self, ROBOT_TARGET_DIRECTORY};

pub mod build;
pub mod check;
pub mod clippy;
pub mod common;
pub mod environment;
pub mod install;
pub mod nextest;
pub mod run;
pub mod test;
mod heading {
    pub const PACKAGE_SELECTION: &str = "Package Selection";
    pub const TARGET_SELECTION: &str = "Target Selection";
    pub const FEATURE_SELECTION: &str = "Feature Selection";
    pub const COMPILATION_OPTIONS: &str = "Compilation Options";
    pub const MANIFEST_OPTIONS: &str = "Manifest Options";
}

lazy_static! {
    /// Map package aliases to their repository-relative directories.
    pub static ref MANIFEST_PATHS: HashMap<&'static str, &'static str> = {
        HashMap::from([
            ("aliveness", "services/aliveness"),
            ("annotato", "tools/annotato"),
            ("depp", "tools/depp"),
            ("pepsi", "tools/pepsi"),
            ("ros-z-cli", "crates/ros-z-cli"),
            ("twix", "tools/twix"),
            ("widget_gallery", "tools/widget_gallery"),
        ])
    };
}

/// Shared Pepsi arguments for running a Cargo subcommand.
#[derive(Args)]
#[group(skip)]
pub struct Arguments<CargoArguments: Args> {
    /// Path to a manifest or package directory, or a known package alias.
    pub manifest: Option<OsString>,
    /// Build environment and execution host options.
    #[command(flatten)]
    pub environment: EnvironmentArguments,
    /// Options forwarded to the selected Cargo subcommand.
    #[command(flatten, next_help_heading = "Cargo Options")]
    pub cargo: CargoArguments,
}

/// Arguments and profile selection for a supported Cargo subcommand.
pub trait CargoCommand {
    /// Arguments that identify the Cargo subcommand, in order.
    const SUB_COMMAND: &'static [&'static str];

    /// Append the subcommand's options to the command.
    fn apply(&self, cmd: &mut Command);
    /// Return the effective Cargo profile.
    fn profile(&self) -> &str;
}

impl<CargoArguments: Args> Arguments<CargoArguments> {
    async fn manifest_path(&self, repository: &Repository) -> Result<Option<PathBuf>> {
        match self.manifest.as_ref() {
            Some(manifest) => resolve_manifest_path(manifest, repository)
                .await
                .wrap_err("failed to resolve manifest path")
                .map(Some),
            None => Ok(None),
        }
    }

    async fn resolve_environment(&self, manifest_path: Option<&Path>) -> Result<Environment> {
        match self.environment.env {
            Some(environment) => Ok(environment),
            None => read_requested_environment(manifest_path)
                .await
                .wrap_err("failed to read requested environment"),
        }
    }
}

impl Arguments<build::Arguments> {
    /// Resolve the expected executable path relative to the repository root.
    pub async fn binary_path(&self, repository: &Repository, binary_name: &str) -> Result<PathBuf> {
        let manifest_path = self.manifest_path(repository).await?;
        let environment = self.resolve_environment(manifest_path.as_deref()).await?;
        let is_native = matches!(environment, Environment::PathCargo | Environment::NixNative);
        let target_dir = self.cargo.common.target_dir.clone().or_else(|| {
            if is_native && !self.environment.remote {
                std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from)
            } else {
                None
            }
        });
        let target_dir = match target_dir {
            None if is_native => PathBuf::from("target"),
            None => PathBuf::from(ROBOT_TARGET_DIRECTORY),
            Some(path) if path.is_absolute() && self.environment.remote => {
                bail!("use a relative --target-dir to retrieve binaries from a remote build")
            }
            Some(path) if path.is_absolute() => diff_paths(path, &repository.root)
                .wrap_err("failed to express target directory relative to repository root")?,
            Some(path) => repository.root_to_current_dir()?.join(path),
        };
        Ok(repository::upload::get_binary(
            target_dir,
            self.cargo.profile(),
            binary_name,
        ))
    }
}

/// Run a Cargo subcommand in the selected build environment.
pub async fn cargo<CargoArguments: Args + CargoCommand>(
    arguments: Arguments<CargoArguments>,
    repository: &Repository,
    compiler_artifacts: &[impl AsRef<Path>],
) -> Result<()> {
    let mut cargo_command =
        construct_cargo_command(arguments, repository, compiler_artifacts).await?;

    debug!("Running `{cargo_command:?}`");
    let status = cargo_command
        .status()
        .await
        .wrap_err("failed to run cargo")?;

    if !status.success() {
        bail!("cargo failed with {status}");
    }

    Ok(())
}

/// Prepare a Cargo command and any remote artifact retrieval without starting it.
pub async fn construct_cargo_command<CargoArguments: Args + CargoCommand>(
    arguments: Arguments<CargoArguments>,
    repository: &Repository,
    compiler_artifacts: &[impl AsRef<Path>],
) -> Result<Command> {
    let manifest_path = arguments.manifest_path(repository).await?;
    let environment = arguments
        .resolve_environment(manifest_path.as_deref())
        .await?;
    let mut cargo = sdk::build_command(
        repository,
        environment,
        arguments.environment.remote,
        "cargo",
        compiler_artifacts,
    )?;
    cargo.args(CargoArguments::SUB_COMMAND);
    if let Some(manifest_path) = manifest_path {
        let manifest_path = diff_paths(
            manifest_path,
            &current_dir().wrap_err("failed to get current directory")?,
        )
        .wrap_err("failed to express manifest relative to current directory")?;
        if CargoArguments::SUB_COMMAND == ["install"] {
            cargo.arg("--path");
            cargo.arg(
                manifest_path
                    .parent()
                    .wrap_err("failed to retrieve package path from manifest path")?,
            );
        } else {
            cargo.arg("--manifest-path");
            cargo.arg(manifest_path);
        }
    }
    arguments.cargo.apply(&mut cargo);
    Ok(cargo)
}

async fn read_requested_environment(manifest_path: Option<&Path>) -> Result<Environment> {
    let Some(manifest_path) = manifest_path else {
        return Ok(Environment::PathCargo);
    };

    let manifest = read_to_string(manifest_path).await.wrap_err_with(|| {
        format!(
            "failed to read manifest at {path}",
            path = manifest_path.display()
        )
    })?;
    let manifest: Table = toml::from_str(&manifest).wrap_err("failed to parse manifest")?;
    Ok(
        if package_metadata(&manifest)
            .and_then(|metadata| metadata.get("cross-compile"))
            .and_then(toml::Value::as_bool)
            == Some(true)
        {
            Environment::NixRobot
        } else {
            Environment::PathCargo
        },
    )
}

async fn resolve_manifest_path(
    manifest: impl AsRef<OsStr>,
    repository: &Repository,
) -> Result<PathBuf> {
    let manifest = manifest.as_ref();

    let manifest_path = manifest
        .to_str()
        .and_then(|manifest| {
            MANIFEST_PATHS
                .get(manifest)
                .map(|manifest_path| repository.root.join(manifest_path))
        })
        .unwrap_or(manifest.into());

    let manifest_path =
        absolute(manifest_path).wrap_err("failed to get absolute path of manifest")?;

    let metadata = tokio::fs::metadata(&manifest_path)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to retrieve metadata for {manifest_path}",
                manifest_path = manifest_path.to_string_lossy()
            )
        })?;

    Ok(if metadata.is_dir() {
        manifest_path.join("Cargo.toml")
    } else {
        manifest_path
    })
}

fn package_metadata(table: &Table) -> Option<&Table> {
    table
        .get("package")?
        .get("metadata")?
        .get("pepsi")?
        .as_table()
}
