use std::str::FromStr;

use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use tokio::process::Command;

use crate::Repository;

#[derive(Debug, Clone, Copy)]
pub enum ContainerRuntime {
    Podman,
    Docker,
}

impl ContainerRuntime {
    pub fn default_for_host() -> Self {
        if cfg!(target_os = "macos") {
            Self::Docker
        } else {
            Self::Podman
        }
    }

    pub fn binary(self) -> &'static str {
        match self {
            Self::Podman => "podman",
            Self::Docker => "docker",
        }
    }

    pub fn allocate_tty(self) -> bool {
        matches!(self, Self::Podman)
    }
}

impl FromStr for ContainerRuntime {
    type Err = String;

    fn from_str(runtime: &str) -> Result<Self, Self::Err> {
        match runtime {
            "podman" => Ok(Self::Podman),
            "docker" => Ok(Self::Docker),
            _ => Err(format!("unknown container runtime {runtime}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SDKImage {
    pub registry: String,
    pub name: String,
    pub tag: String,
}

impl SDKImage {
    pub fn image_url(&self) -> String {
        format!("{}/{}:{}", self.registry, self.name, self.tag)
    }

    pub fn name_tagged(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }

    pub fn name_latest(&self) -> String {
        format!("{}:latest", self.name)
    }

    pub fn parse_and_update(mut self, mut image: &str) -> Self {
        if let Some((registry, rest)) = image.rsplit_once("/") {
            self.registry = registry.to_string();
            image = rest;
        }
        if let Some((rest, tag)) = image.rsplit_once(":") {
            self.tag = tag.to_string();
            image = rest;
        }

        self.name = image.to_string();

        self
    }
}

/// Pulls the SDK image with the specified version.
pub async fn pull_sdk_image(sdk_image: &SDKImage) -> Result<()> {
    pull_sdk_image_with(sdk_image, ContainerRuntime::Podman).await
}

pub async fn pull_sdk_image_with(sdk_image: &SDKImage, runtime: ContainerRuntime) -> Result<()> {
    let image_url = sdk_image.image_url();
    let name_tagged = sdk_image.name_tagged();
    let mut command = Command::new(runtime.binary());
    command.arg("pull");

    if matches!(runtime, ContainerRuntime::Podman) {
        command.args(["--policy", "missing"]);
    }

    let status = command.arg(&image_url).status().await?;

    if !status.success() {
        bail!("{} failed with {status}", runtime.binary());
    }

    let status = Command::new(runtime.binary())
        .args(["tag", &image_url, &name_tagged])
        .status()
        .await?;

    if !status.success() {
        bail!("{} failed with {status}", runtime.binary());
    }

    Ok(())
}

pub async fn build_sdk_container(repository: &Repository, sdk_image: &SDKImage) -> Result<()> {
    build_sdk_container_with(repository, sdk_image, ContainerRuntime::Podman).await
}

pub async fn build_sdk_container_with(
    repository: &Repository,
    sdk_image: &SDKImage,
    runtime: ContainerRuntime,
) -> Result<()> {
    let containerfile_path = repository.root.join("tools/sdk_container/");
    let containerfile_path_str = containerfile_path
        .to_str()
        .wrap_err("failed to convert containerfile path to string")?;
    let containerfile = containerfile_for_host(repository);
    let containerfile_str = containerfile
        .to_str()
        .wrap_err("failed to convert containerfile path to string")?;

    let status = Command::new(runtime.binary())
        .args([
            "build",
            "-f",
            containerfile_str,
            "-t",
            &sdk_image.name_latest(),
            "-t",
            &sdk_image.name_tagged(),
            containerfile_path_str,
        ])
        .status()
        .await?;

    if !status.success() {
        bail!("{} failed with {status}", runtime.binary());
    }
    Ok(())
}

pub async fn image_exists_locally(sdk_image: &SDKImage) -> Result<bool> {
    image_exists_locally_with(sdk_image, ContainerRuntime::Podman).await
}

pub async fn image_exists_locally_with(
    sdk_image: &SDKImage,
    runtime: ContainerRuntime,
) -> Result<bool> {
    let image_name = sdk_image.name_tagged();
    let images = Command::new(runtime.binary())
        .args([
            "image",
            "list",
            "--quiet",
            "--filter",
            &format!("reference={image_name}"),
        ])
        .output()
        .await
        .wrap_err_with(|| format!("failed to list SDK images with {}", runtime.binary()))?;

    if !images.status.success() {
        bail!(
            "{} could not list SDK images: {}",
            runtime.binary(),
            String::from_utf8_lossy(&images.stderr).trim()
        );
    }
    if images.stdout.is_empty() {
        return Ok(false);
    }

    let output = Command::new(runtime.binary())
        .args([
            "image",
            "inspect",
            "--format",
            "{{.Architecture}}",
            &image_name,
        ])
        .output()
        .await
        .wrap_err_with(|| format!("failed to inspect SDK image with {}", runtime.binary()))?;

    if !output.status.success() {
        bail!(
            "{} could not inspect SDK image {image_name}: {}",
            runtime.binary(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim() == expected_image_architecture())
}

pub async fn list_sdk_images_with(runtime: ContainerRuntime) -> Result<()> {
    let status = Command::new(runtime.binary())
        .args(["image", "list", "--filter", "reference=k1sdk"])
        .status()
        .await?;

    if !status.success() {
        bail!("{} failed with {status}", runtime.binary());
    }

    Ok(())
}

fn containerfile_for_host(repository: &Repository) -> std::path::PathBuf {
    let containerfile = if expected_image_architecture() == "arm64" {
        "Containerfile.arm64"
    } else {
        "Containerfile"
    };

    repository
        .root
        .join("tools/sdk_container")
        .join(containerfile)
}

fn expected_image_architecture() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "amd64",
        architecture => architecture,
    }
}
