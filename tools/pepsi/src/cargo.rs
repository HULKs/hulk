use std::{
    env::current_dir,
    ffi::{OsStr, OsString},
    path::{absolute, Path, PathBuf},
};

use clap::Args;
use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};
use environment::{Environment, EnvironmentArguments};
use pathdiff::diff_paths;
use repository::{cargo::Cargo, Repository};
use tokio::fs::read_to_string;
use toml::Table;
use tracing::debug;

pub mod build;
pub mod check;
pub mod clippy;
pub mod common;
pub mod environment;
pub mod install;
pub mod run;
pub mod test;
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
    #[command(flatten, next_help_heading = "Cargo Options")]
    pub cargo: CargoArguments,
}

pub trait CargoCommand {
    const SUB_COMMAND: &'static str;

    fn apply(&self, cmd: &mut Cargo);
    fn profile(&self) -> &str;
}

pub async fn cargo<CargoArguments: Args + CargoCommand>(
    arguments: Arguments<CargoArguments>,
    repository: &Repository,
    compiler_artifacts: &[impl AsRef<Path>],
) -> Result<()> {
    // Map with async closures would be nice here (not yet stabilized)
    let manifest_path = match arguments.manifest {
        Some(manifest) => {
            let absolute_manifest = resolve_manifest_path(&manifest, repository)
                .await
                .wrap_err("failed to resolve manifest path")?;
            let relative_manifest = diff_paths(
                absolute_manifest,
                &current_dir().wrap_err("failed to get current directory")?,
            )
            .wrap_err("failed to express manifest relative to repository root")?;

            Some(relative_manifest)
        }
        None => None,
    };

    let environment = match arguments.environment.env {
        Some(environment) => environment,
        None => read_requested_environment(&manifest_path)
            .await
            .wrap_err("failed to read requested environment")?,
    }
    .resolve(repository)
    .await
    .wrap_err("failed to resolve environment")?;

    eprintln!("Using cargo from {environment}");

    let mut cargo = if arguments.environment.remote {
        Cargo::remote(environment)
    } else {
        Cargo::local(environment)
    };

    cargo
        .setup(repository)
        .await
        .wrap_err("failed to set up cargo environment")?;

    cargo.arg(CargoArguments::SUB_COMMAND);

    if let Some(manifest_path) = manifest_path {
        if CargoArguments::SUB_COMMAND == "install" {
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

    let mut cargo_command = cargo
        .command(repository, compiler_artifacts)
        .wrap_err("failed to create cargo command")?;

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

async fn read_requested_environment(manifest_path: &Option<PathBuf>) -> Result<Environment> {
    let Some(manifest_path) = manifest_path else {
        return Ok(Environment::Native);
    };

    let manifest = read_to_string(manifest_path).await.wrap_err_with(|| {
        format!(
            "failed to read manifest at {path}",
            path = manifest_path.display()
        )
    })?;
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

    if cfg!(target_os = "linux") {
        Ok(Environment::Sdk { version: None })
    } else {
        Ok(Environment::Docker { image: None })
    }
}

async fn resolve_manifest_path(
    manifest: impl AsRef<OsStr>,
    repository: &Repository,
) -> Result<PathBuf> {
    let manifest = manifest.as_ref();

    Ok(match manifest.to_str() {
        Some("imagine") => repository.root.join("crates/hulk_imagine/Cargo.toml"),
        Some("nao") => repository.root.join("crates/hulk_nao/Cargo.toml"),
        Some("replayer") => repository.root.join("crates/hulk_replayer/Cargo.toml"),
        Some("webots") => repository.root.join("crates/hulk_webots/Cargo.toml"),

        Some("aliveness") => repository.root.join("services/aliveness/Cargo.toml"),
        Some("breeze") => repository.root.join("services/breeze/Cargo.toml"),
        Some("hula") => repository.root.join("services/hula/Cargo.toml"),
        Some("power-panic") => repository.root.join("services/power-panic/Cargo.toml"),

        Some("annotato") => repository.root.join("tools/annotato/Cargo.toml"),
        Some("camera_matrix_extractor") => repository
            .root
            .join("tools/camera_matrix_extractor/Cargo.toml"),
        Some("depp") => repository.root.join("tools/depp/Cargo.toml"),
        Some("fanta") => repository.root.join("tools/fanta/Cargo.toml"),
        Some("parameter_tester") => repository.root.join("tools/parameter_tester/Cargo.toml"),
        Some("pepsi") => repository.root.join("tools/pepsi/Cargo.toml"),
        Some("twix") => repository.root.join("tools/twix/Cargo.toml"),
        Some("vista") => repository.root.join("tools/vista/Cargo.toml"),
        Some("widget_gallery") => repository.root.join("tools/widget_gallery/Cargo.toml"),

        _ => compose_manifest_path(manifest).await.wrap_err_with(|| {
            format!(
                "failed to resolve manifest path for {manifest}",
                manifest = manifest.to_string_lossy()
            )
        })?,
    })
}

async fn compose_manifest_path(manifest: impl AsRef<OsStr>) -> Result<PathBuf> {
    let manifest_path =
        absolute(manifest.as_ref()).wrap_err("failed to get absolute path of manifest")?;
    Ok(
        if tokio::fs::metadata(&manifest_path)
            .await
            .wrap_err("failed to retrieve metadata")?
            .is_dir()
        {
            manifest_path.join("Cargo.toml")
        } else {
            manifest_path
        },
    )
}

fn package_metadata(table: &Table) -> Option<&Table> {
    table
        .get("package")?
        .get("metadata")?
        .get("pepsi")?
        .as_table()
}
