use color_eyre::{
    eyre::{bail, ContextCompat},
    Result,
};
use tokio::process::Command;

use crate::Repository;

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
    let status = Command::new("podman")
        .args(["pull", "--policy", "missing", &sdk_image.image_url()])
        .status()
        .await?;

    if !status.success() {
        bail!("podman failed with {status}");
    }
    Ok(())
}

pub async fn build_sdk_container(repository: &Repository, sdk_image: &SDKImage) -> Result<()> {
    let containerfile_path = repository.root.join("tools/sdk_container/");
    let containerfile_path_str = containerfile_path
        .to_str()
        .wrap_err("failed to convert containerfile path to string")?;

    let status = Command::new("podman")
        .args([
            "build",
            "-t",
            &sdk_image.name_latest(),
            "-t",
            &sdk_image.name_tagged(),
            containerfile_path_str,
        ])
        .status()
        .await?;

    if !status.success() {
        bail!("podman failed with {status}");
    }
    Ok(())
}

pub async fn image_exists_locally(sdk_image: &SDKImage) -> bool {
    let status = Command::new("podman")
        .args(["image", "exists", &sdk_image.name_latest()])
        .status()
        .await
        .expect("failed to check for image existence");

    status.success()
}
