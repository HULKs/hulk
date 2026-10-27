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
use repository::{Repository, cargo::Cargo, sdk::ContainerRuntime};
use serde::Deserialize;
use tokio::{fs::read_to_string, process::Command};
use toml::Table;
use tracing::debug;

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
    fn selected_packages(&self) -> &[String];
    fn workspace_requested(&self) -> bool;

    fn executes_target(&self) -> bool {
        matches!(Self::SUB_COMMAND, "run" | "test" | "nextest run")
    }
}

impl<CargoArguments: Args + CargoCommand> Arguments<CargoArguments> {
    async fn manifest_path(&self, repository: &Repository) -> Result<Option<PathBuf>> {
        match self.manifest.as_ref() {
            Some(manifest) => resolve_manifest_path(manifest, repository)
                .await
                .wrap_err("failed to resolve manifest path")
                .map(Some),
            None => Ok(None),
        }
    }

    async fn resolve_environment(
        &self,
        manifest_path: Option<&Path>,
        repository: &Repository,
    ) -> Result<repository::cargo::Environment> {
        match self.environment.env.clone() {
            Some(environment) => environment,
            // The SDK builds AArch64 binaries. Remote execution uses the remote's
            // native toolchain because its architecture is not known locally.
            None if self.cargo.executes_target()
                && (self.environment.remote || !cfg!(target_arch = "aarch64")) =>
            {
                Environment::Native
            }
            None => read_requested_environment(
                manifest_path,
                CargoArguments::SUB_COMMAND,
                self.cargo.selected_packages(),
                self.cargo.workspace_requested(),
                self.environment.remote,
            )
            .await
            .wrap_err("failed to read requested environment")?,
        }
        .resolve(repository)
        .await
        .wrap_err("failed to resolve environment")
    }
}

impl Arguments<build::Arguments> {
    pub async fn binary_path(&self, repository: &Repository, binary_name: &str) -> Result<PathBuf> {
        let manifest_path = self.manifest_path(repository).await?;
        let environment = self
            .resolve_environment(manifest_path.as_deref(), repository)
            .await?;
        let is_native = matches!(environment, repository::cargo::Environment::Native);
        let target_dir = self.cargo.common.target_dir.clone().or_else(|| {
            if is_native && !self.environment.remote {
                std::env::var_os("CARGO_TARGET_DIR").map(PathBuf::from)
            } else {
                None
            }
        });
        let target_dir = match target_dir {
            None if is_native => PathBuf::from("target"),
            None => PathBuf::from(repository::cargo::CONTAINER_TARGET_DIRECTORY),
            Some(path) if path.is_absolute() && !is_native => path
                .strip_prefix("/hulk")
                .wrap_err("container target directory must be under /hulk to retrieve binaries")?
                .to_path_buf(),
            Some(path) if path.is_absolute() && self.environment.remote => {
                bail!("use a relative --target-dir to retrieve binaries from a remote native build")
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

pub async fn construct_cargo_command<CargoArguments: Args + CargoCommand>(
    arguments: Arguments<CargoArguments>,
    repository: &Repository,
    compiler_artifacts: &[impl AsRef<Path>],
) -> Result<tokio::process::Command, color_eyre::eyre::Error> {
    let manifest_path = arguments.manifest_path(repository).await?;
    let environment = arguments
        .resolve_environment(manifest_path.as_deref(), repository)
        .await?;
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
        let manifest_path = diff_paths(
            manifest_path,
            &current_dir().wrap_err("failed to get current directory")?,
        )
        .wrap_err("failed to express manifest relative to current directory")?;
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
    let cargo_command = cargo
        .command(repository, compiler_artifacts)
        .wrap_err("failed to create cargo command")?;

    Ok(cargo_command)
}

async fn read_requested_environment(
    manifest_path: Option<&Path>,
    cargo_subcommand: &str,
    selected_packages: &[String],
    workspace_requested: bool,
    remote: bool,
) -> Result<Environment> {
    if cargo_subcommand == "install" && manifest_path.is_none() {
        return Ok(Environment::Native);
    }

    let manifest_path = match manifest_path {
        Some(path) => path.to_path_buf(),
        None => {
            locate_manifest(&current_dir().wrap_err("failed to get current directory")?).await?
        }
    };
    let manifest = read_to_string(&manifest_path)
        .await
        .wrap_err_with(|| format!("failed to read manifest at {}", manifest_path.display()))?;
    let manifest: Table = toml::from_str(&manifest).wrap_err("failed to parse manifest")?;

    if workspace_requested && workspace_command_uses_sdk(cargo_subcommand, remote) {
        return Ok(sdk_environment(remote));
    }

    if !selected_packages.is_empty() {
        return Ok(
            if selected_packages_request_cross_compile(selected_packages, &manifest_path).await? {
                sdk_environment(remote)
            } else {
                Environment::Native
            },
        );
    }

    if manifest_requests_cross_compile(&manifest)
        || (manifest.get("package").is_none()
            && manifest.get("workspace").is_some()
            && workspace_command_uses_sdk(cargo_subcommand, remote))
    {
        return Ok(sdk_environment(remote));
    }

    Ok(Environment::Native)
}

async fn locate_manifest(directory: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["locate-project", "--message-format", "plain"])
        .current_dir(directory)
        .output()
        .await
        .wrap_err("failed to locate Cargo manifest")?;
    if !output.status.success() {
        bail!(
            "cargo locate-project failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim(),
        );
    }
    let path = String::from_utf8(output.stdout).wrap_err("invalid Cargo manifest path")?;
    Ok(PathBuf::from(path.trim()))
}

fn manifest_requests_cross_compile(manifest: &Table) -> bool {
    package_metadata(manifest)
        .and_then(|metadata| metadata.get("cross-compile"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn workspace_command_uses_sdk(cargo_subcommand: &str, remote: bool) -> bool {
    !remote
        && cfg!(target_os = "macos")
        && matches!(
            cargo_subcommand,
            "build" | "check" | "clippy" | "nextest run" | "test"
        )
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
}

#[derive(Deserialize)]
struct CargoMetadataPackage {
    id: String,
    name: String,
    metadata: serde_json::Value,
}

impl CargoMetadataPackage {
    fn requests_cross_compile(&self) -> bool {
        self.metadata
            .get("pepsi")
            .and_then(|metadata| metadata.get("cross-compile"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }
}

async fn selected_packages_request_cross_compile(
    selected_packages: &[String],
    manifest_path: &Path,
) -> Result<bool> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(manifest_path)
        .output()
        .await
        .wrap_err("failed to read Cargo workspace metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed with {status}: {stderr}",
            status = output.status,
            stderr = String::from_utf8_lossy(&output.stderr),
        );
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .wrap_err("failed to parse Cargo workspace metadata")?;

    for spec in selected_packages {
        if spec.contains(['*', '?', '[', ']']) {
            let pattern = glob::Pattern::new(spec)
                .wrap_err_with(|| format!("invalid Cargo package pattern {spec}"))?;
            if metadata
                .packages
                .iter()
                .any(|package| pattern.matches(&package.name) && package.requests_cross_compile())
            {
                return Ok(true);
            }
            continue;
        }

        let output = Command::new("cargo")
            .args(["pkgid", "--manifest-path"])
            .arg(manifest_path)
            .args(["--package", spec])
            .output()
            .await
            .wrap_err_with(|| format!("failed to resolve Cargo package {spec}"))?;
        if !output.status.success() {
            bail!(
                "cargo pkgid failed for {spec} with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim(),
            );
        }
        let id = String::from_utf8(output.stdout)
            .wrap_err_with(|| format!("invalid package ID for {spec}"))?;
        if metadata
            .packages
            .iter()
            .any(|package| package.id == id.trim() && package.requests_cross_compile())
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn sdk_environment(remote: bool) -> Environment {
    // The remote compiler uses Podman, independently of the client's OS.
    if remote {
        return Environment::Podman { image: None };
    }

    match ContainerRuntime::default_for_host() {
        ContainerRuntime::Podman => Environment::Podman { image: None },
        ContainerRuntime::Docker => Environment::Docker { image: None },
    }
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
