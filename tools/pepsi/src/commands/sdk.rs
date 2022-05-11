use std::{fs::Permissions, os::unix::prelude::PermissionsExt, path::PathBuf, string::String};

use anyhow::{bail, Context};
use log::info;
use tokio::{
    fs::{create_dir, remove_file, set_permissions, symlink},
    process::Command,
};

pub const SDK_VERSION: &str = "4.0";
pub const INSTALLATION_DIRECTORY: &str = "/opt/nao";

async fn create_symlink(
    sdk_directory: PathBuf,
    installation_directory: PathBuf,
) -> anyhow::Result<()> {
    let symlink_path = sdk_directory.join("current");
    if symlink_path.exists() {
        remove_file(&symlink_path)
            .await
            .context("Failed to remove current SDK symlink")?;
    }
    symlink(&installation_directory, &symlink_path)
        .await
        .context("Failed to symlink current SDK to installation directory")?;
    Ok(())
}

pub async fn install(
    project_root: PathBuf,
    force_reinstall: bool,
    alternative_sdk_version: Option<String>,
    alternative_installation_directory: Option<PathBuf>,
    _is_verbose: bool,
) -> anyhow::Result<()> {
    let sdk_directory = project_root.join("sdk");
    let sdk_version = alternative_sdk_version.unwrap_or_else(|| SDK_VERSION.to_string());
    let installation_directory =
        alternative_installation_directory.unwrap_or_else(|| INSTALLATION_DIRECTORY.into());
    let installation_directory = installation_directory.join(&sdk_version);
    let needs_installation = force_reinstall || !installation_directory.exists();
    if !needs_installation {
        let current_symlink = sdk_directory.join("current");
        if !current_symlink.exists() {
            create_symlink(sdk_directory, installation_directory).await?;
        }
        return Ok(());
    }

    let downloads_directory = sdk_directory.join("downloads");
    let installer_name = format!("HULKs-OS-toolchain-{}.sh", sdk_version);
    let download_file_path = downloads_directory.join(&installer_name);
    if !download_file_path.exists() {
        if !downloads_directory.exists() {
            create_dir(downloads_directory)
                .await
                .context("Failed to create download directory")?;
        }
        let url = format!("http://bighulk/sdk/{}", installer_name);
        info!("GET {}", url);
        let exit_status = Command::new("curl")
            .arg("--progress-bar")
            .arg("--output")
            .arg(&download_file_path)
            .arg(url)
            .status()
            .await
            .context("Failed to download SDK")?;
        if !exit_status.success() {
            bail!("curl exited with {}", exit_status);
        }
        set_permissions(&download_file_path, Permissions::from_mode(0o755))
            .await
            .context("Failed to make installer executable")?;
    }

    let exit_status = Command::new(download_file_path)
        .arg("-d")
        .arg(&installation_directory)
        .status()
        .await
        .context("Failed to install SDK")?;
    if !exit_status.success() {
        bail!("SDK installer exited with {}", exit_status);
    }

    create_symlink(sdk_directory, installation_directory).await?;

    Ok(())
}
