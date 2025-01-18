use std::path::Path;

use color_eyre::{eyre::Context, Result};
use serde::Deserialize;
use tokio::fs::read_to_string;

#[derive(Debug, Deserialize)]
pub struct Configuration {
    pub os_version: String,
    pub sdk_version: String,
}

/// Get the OS version configured in the `hulk.toml`.
pub async fn read_os_version(repository_root: impl AsRef<Path>) -> Result<String> {
    let hulk = read_hulk_toml(repository_root).await?;
    Ok(hulk.os_version)
}

/// Get the SDK version configured in the `hulk.toml`.
pub async fn read_sdk_version(repository_root: impl AsRef<Path>) -> Result<String> {
    let hulk = read_hulk_toml(repository_root).await?;
    Ok(hulk.sdk_version)
}

pub async fn read_hulk_toml(repository_root: impl AsRef<Path>) -> Result<Configuration> {
    let hulk_toml = repository_root.as_ref().join("hulk.toml");
    let hulk_toml = read_to_string(hulk_toml)
        .await
        .wrap_err("failed to read hulk.toml")?;
    let hulk: Configuration = toml::from_str(&hulk_toml).wrap_err("failed to parse hulk.toml")?;
    Ok(hulk)
}
