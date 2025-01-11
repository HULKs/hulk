use std::{
    ffi::{OsStr, OsString},
    path::{absolute, Path, PathBuf},
    process::Command,
};

use clap::Args;
use color_eyre::{
    eyre::{bail, Context, ContextCompat},
    Result,
};
use environment::{Environment, EnvironmentArguments};
use pathdiff::diff_paths;
use repository::cargo::Cargo;
use tokio::fs::read_to_string;
use toml::Table;

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
    compiler_artifacts: &[impl AsRef<Path>],
) -> Result<()> {
    // Map with async closures would be nice here (not yet stabilized)
    let manifest_path = match arguments.manifest {
        Some(manifest) => {
            let absolute_manifest = resolve_manifest_path(&manifest, &repository_root)
                .await
                .wrap_err("failed to resolve manifest path")?;
            let relative_manifest = diff_paths(absolute_manifest, &repository_root)
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
    .resolve(&repository_root)
    .await
    .wrap_err("failed to resolve environment")?;

    eprintln!("Using cargo from {environment}");

    let cargo = if arguments.environment.remote {
        Cargo::remote(environment)
    } else {
        Cargo::local(environment)
    };

    let mut cargo_command = cargo
        .command(&repository_root, compiler_artifacts)
        .wrap_err("failed to create cargo command")?;

    cargo_command.arg(CargoArguments::SUB_COMMAND);

    if let Some(manifest_path) = manifest_path {
        if CargoArguments::SUB_COMMAND == "install" {
            cargo_command.arg("--path");
            cargo_command.arg(
                manifest_path
                    .parent()
                    .wrap_err("failed to retrieve package path from manifest path")?,
            );
        } else {
            cargo_command.arg("--manifest-path");
            cargo_command.arg(manifest_path);
        }
    }

    arguments.cargo.apply(&mut cargo_command);

    cargo
        .setup(&repository_root)
        .await
        .wrap_err("failed to set up cargo environment")?;

    let status = tokio::process::Command::from(cargo_command)
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
        Some("imagine") => repository_root.join("crates/hulk_imagine/Cargo.toml"),
        Some("nao") => repository_root.join("crates/hulk_nao/Cargo.toml"),
        Some("replayer") => repository_root.join("crates/hulk_replayer/Cargo.toml"),
        Some("webots") => repository_root.join("crates/hulk_webots/Cargo.toml"),

        Some("aliveness") => repository_root.join("services/aliveness/Cargo.toml"),
        Some("breeze") => repository_root.join("services/breeze/Cargo.toml"),
        Some("hula") => repository_root.join("services/hula/Cargo.toml"),

        Some("annotato") => repository_root.join("tools/annotato/Cargo.toml"),
        Some("camera_matrix_extractor") => {
            repository_root.join("tools/camera_matrix_extractor/Cargo.toml")
        }
        Some("depp") => repository_root.join("tools/depp/Cargo.toml"),
        Some("fanta") => repository_root.join("tools/fanta/Cargo.toml"),
        Some("parameter_tester") => repository_root.join("tools/parameter_tester/Cargo.toml"),
        Some("pepsi") => repository_root.join("tools/pepsi/Cargo.toml"),
        Some("twix") => repository_root.join("tools/twix/Cargo.toml"),
        Some("widget_gallery") => repository_root.join("tools/widget_gallery/Cargo.toml"),

        _ => {
            let manifest_path =
                absolute(manifest).wrap_err("failed to get absolute path of manifest")?;

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
