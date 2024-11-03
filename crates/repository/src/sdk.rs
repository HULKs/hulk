use std::{
    env::{self, consts::ARCH},
    fs::Permissions,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use log::info;
use tokio::{
    fs::{create_dir_all, remove_dir_all, remove_file, rename, set_permissions, File},
    process::Command,
};

use crate::download::{download_with_fallback, CONNECT_TIMEOUT};

/// Downloads and installs a specified SDK version.
///
/// This function ensures an installed SDK version. If it is not installed, it will download the
/// SDK installer and run it to install the SDK. If the SDK installation is incomplete, it will
/// remove the incomplete installation and try again.
pub async fn download_and_install(version: &str, data_home: impl AsRef<Path>) -> Result<()> {
    let data_home = data_home.as_ref();
    let sdk_home = data_home.join("sdk/");
    let installation_directory = sdk_home.join(version);

    let incomplete_marker = sdk_home.join(format!("{version}.incomplete"));
    if installation_directory.exists() && incomplete_marker.exists() {
        info!("Removing incomplete SDK ({version}) of previous installation attempt...");
        remove_dir_all(&installation_directory)
            .await
            .wrap_err("failed to remove incomplete SDK directory")?;
    }

    if !installation_directory.exists() {
        let installer_path = download(version, &sdk_home)
            .await
            .wrap_err("failed to download SDK")?;

        File::create(&incomplete_marker)
            .await
            .wrap_err("failed to create marker")?;
        run_installer(installer_path, &installation_directory)
            .await
            .wrap_err("failed to install SDK")?;
        remove_file(&incomplete_marker)
            .await
            .wrap_err("failed to remove marker")?;
    }
    Ok(())
}

async fn download(version: &str, sdk_home: impl AsRef<Path>) -> Result<PathBuf> {
    let downloads_directory = sdk_home.as_ref().join("downloads");
    let installer_name = format!("HULKs-OS-{ARCH}-toolchain-{version}.sh");
    let installer_path = downloads_directory.join(&installer_name);
    let download_path = installer_path.with_extension("tmp");

    create_dir_all(&downloads_directory)
        .await
        .wrap_err("failed to create download directory")?;

    let urls = [
        format!("http://bighulk.hulks.dev/sdk/{installer_name}"),
        format!("https://github.com/HULKs/meta-nao/releases/download/{version}/{installer_name}"),
    ];
    download_with_fallback(urls, &download_path, CONNECT_TIMEOUT).await?;

    set_permissions(&download_path, Permissions::from_mode(0o755))
        .await
        .wrap_err("failed to mark installer executable")?;

    rename(download_path, &installer_path)
        .await
        .wrap_err("failed to rename sdk installer")?;

    Ok(installer_path)
}

async fn run_installer(
    installer: impl AsRef<Path>,
    target_directory: impl AsRef<Path>,
) -> Result<()> {
    let var_name = Command::new(installer.as_ref().as_os_str());
    let mut command = var_name;
    command.arg("-d");
    command.arg(target_directory.as_ref().as_os_str());
    if env::var("NAOSDK_AUTOMATIC_YES")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        command.arg("-y");
    }
    let status = command.status().await.wrap_err("failed to spawn command")?;

    if !status.success() {
        bail!("SDK installer exited with {status}");
    }
    Ok(())
}
