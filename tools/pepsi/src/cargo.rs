use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

use clap::Args;
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use environment::{Environment, EnvironmentArguments};
use repository::cargo::Cargo;
use toml::Table;

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

#[derive(Args)]
#[group(skip)]
pub struct Arguments<CargoArguments: Args> {
    pub manifest: Option<OsString>,
    #[command(flatten)]
    pub environment: EnvironmentArguments,
    #[command(flatten)]
    pub cargo: CargoArguments,
}

pub trait CargoCommand {
    const SUB_COMMAND: &'static str;

    fn apply(&self, cmd: &mut Command);
    fn profile(&self) -> &str;
}

pub async fn cargo<CargoArguments: Args + CargoCommand>(
    arguments: Arguments<CargoArguments>,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    // Map with async closures would be nice here (not yet stabilized)
    let manifest_path = match arguments.manifest {
        Some(manifest) => Some(
            resolve_manifest_path(&manifest, &repository_root)
                .await
                .wrap_err("failed to resolve manifest path")?,
        ),
        None => None,
    };

    let environment = match arguments.environment.env {
        Some(environment) => environment,
        None => read_requested_environment(&manifest_path)
            .await
            .wrap_err("failed to read requested environment")?,
    }
    .resolve(&repository_root)
    .await
    .wrap_err("failed to resolve enviroment")?;

    eprintln!("Using cargo from {environment}");

    let cargo = if arguments.environment.remote {
        Cargo::remote(environment)
    } else {
        Cargo::local(environment)
    };

    let mut cargo_command = cargo
        .command(&repository_root)
        .wrap_err("failed to create cargo command")?;

    cargo_command.arg(CargoArguments::SUB_COMMAND);

    if let Some(manifest_path) = manifest_path {
        cargo_command.arg("--manifest-path");
        cargo_command.arg(manifest_path);
    }

    // TODO: Build extension trait for readability
    arguments.cargo.apply(&mut cargo_command);

    cargo
        .setup()
        .await
        .wrap_err("failed to set up cargo environment")?;

    let status = tokio::process::Command::from(cargo_command)
        .status()
        .await
        .wrap_err("failed to run cargo")?;

    if !status.success() {
        bail!("pepsi build failed with {status}");
    }

    Ok(())
}

async fn read_requested_environment(manifest_path: &Option<PathBuf>) -> Result<Environment> {
    let Some(manifest_path) = manifest_path else {
        return Ok(Environment::Native);
    };

    let manifest = tokio::fs::read_to_string(manifest_path)
        .await
        .wrap_err("failed to read manifest at {manifest_path}")?;
    let manifest: Table = toml::from_str(&manifest).wrap_err("failed to parse manifest")?;
    let Some(package_metadata) = package_metadata(&manifest) else {
        return Ok(Environment::Native);
    };

    let is_cross_compile_requested = package_metadata
        .get("cross-compile")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);

    if !is_cross_compile_requested {
        return Ok(Environment::Native);
    }

    #[cfg(target_os = "linux")]
    if cfg!(target_os = "linux") {
        Ok(Environment::Sdk { version: None })
    } else {
        Ok(Environment::Docker { image: None })
    }
}

async fn resolve_manifest_path(
    manifest: impl AsRef<OsStr>,
    repository_root: impl AsRef<Path>,
) -> Result<PathBuf> {
    let manifest = manifest.as_ref();
    let repository_root = repository_root.as_ref();

    Ok(match manifest.to_str() {
        Some("nao") => repository_root.join("crates/hulk_nao/Cargo.toml"),
        Some("imagine") => repository_root.join("crates/hulk_imagine/Cargo.toml"),
        Some("replayer") => repository_root.join("crates/hulk_replayer/Cargo.toml"),
        Some("webots") => repository_root.join("crates/hulk_webots/Cargo.toml"),
        _ => {
            let manifest_path = PathBuf::from(manifest);

            if tokio::fs::metadata(&manifest_path)
                .await
                .wrap_err("failed to retrieve metadata")?
                .is_dir()
            {
                manifest_path.join("Cargo.toml")
            } else {
                manifest_path
            }
        }
    })
}

fn package_metadata(table: &Table) -> Option<&Table> {
    table
        .get("package")?
        .get("metadata")?
        .get("pepsi")?
        .as_table()
}
