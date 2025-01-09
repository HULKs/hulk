use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use clap::Args;
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use repository::cargo::Cargo;

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
    fn profile(&self) -> &str;
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

    if let Some(manifest) = arguments.manifest {
        let manifest_path = resolve_manifest_path(&manifest, &repository_root)
            .await
            .wrap_err("failed to resolve manifest path")?;

        cargo_command.arg("--manifest-path");
        cargo_command.arg(manifest_path);
    }

    let status = tokio::process::Command::from(cargo_command)
        .status()
        .await
        .wrap_err("failed to run cargo build")?;

    if !status.success() {
        bail!("pepsi build failed with {status}");
    }

    Ok(())
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
