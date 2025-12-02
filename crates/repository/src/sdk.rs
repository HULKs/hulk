use color_eyre::{eyre::bail, Result};
use tokio::process::Command;

/// Downloads and installs a specified SDK version.
///
/// This function ensures an installed SDK version. If it is not installed, it will download the
/// SDK installer and run it to install the SDK. If the SDK installation is incomplete, it will
/// remove the incomplete installation and try again.
pub async fn download_and_install(image: &str) -> Result<()> {
    let status = Command::new("podman")
        .args(["pull", "--policy", "missing", image])
        .status()
        .await?;

    if !status.success() {
        bail!("podman failed with {status}");
    }
    Ok(())
}
