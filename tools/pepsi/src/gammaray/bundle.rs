use std::{path::PathBuf, process::Stdio};

use color_eyre::{
    Result,
    eyre::{WrapErr, bail},
};
use repository::Repository;
use tokio::{fs, process::Command};

const NIX_INSTALLER_URL: &str =
    "https://releases.nixos.org/nix/nix-2.34.8/nix-2.34.8-aarch64-linux.tar.xz";

pub(super) async fn prepare(repository: &Repository) -> Result<PathBuf> {
    let destination = repository
        .resolve_data_home()
        .await?
        .join("image")
        .join(repository.read_runtime_version().await?);
    if fs::try_exists(&destination).await? {
        return fs::canonicalize(&destination)
            .await
            .wrap_err("failed to resolve package bundle");
    }

    let destination = std::path::absolute(destination)?;
    let parent = destination
        .parent()
        .expect("absolute bundle path has a parent");
    fs::create_dir_all(parent).await?;
    // Publish only a finished bundle. Failed preparation leaves no cache entry.
    let temporary = tempfile::Builder::new()
        .prefix(".hulks-")
        .tempdir_in(parent)?;
    let result = temporary.path().join("result");
    eprintln!("Preparing package bundle at {}", destination.display());
    run(
        Command::new("nix")
            .current_dir(&repository.root)
            .args([
                "--extra-experimental-features",
                "nix-command flakes",
                "build",
                ".#packages.aarch64-linux.hulk-runtime",
                "--no-update-lock-file",
                "--out-link",
            ])
            .arg(&result),
        "building hulk-runtime environment",
    )
    .await?;

    let root = fs::canonicalize(&result).await?;
    fs::write(
        temporary.path().join("root-path"),
        format!("{}\n", root.display()),
    )
    .await?;
    let output = Command::new("nix-store")
        .args(["--query", "--requisites"])
        .arg(&root)
        .stderr(Stdio::inherit())
        .output()
        .await
        .wrap_err("failed to query hulk-runtime closure")?;
    if !output.status.success() {
        bail!("querying hulk-runtime closure failed: {}", output.status);
    }
    let closure = String::from_utf8(output.stdout).wrap_err("invalid closure paths")?;
    let export = fs::File::create(temporary.path().join("nix.export")).await?;
    run(
        Command::new("nix-store")
            .arg("--export")
            .args(closure.lines())
            .stdout(Stdio::from(export.into_std().await)),
        "exporting hulk-runtime closure",
    )
    .await?;
    run(
        Command::new("curl")
            .args(["--fail", "--location", "--output"])
            .arg(temporary.path().join("nix-installer.tar.xz"))
            .arg(NIX_INSTALLER_URL),
        "downloading Nix installer",
    )
    .await?;
    fs::remove_file(&result).await?;
    fs::rename(temporary.path(), &destination)
        .await
        .wrap_err("failed to publish package bundle")?;
    eprintln!("Prepared offline bundle: {}", destination.display());
    Ok(destination)
}

async fn run(command: &mut Command, operation: &str) -> Result<()> {
    let status = command
        .status()
        .await
        .wrap_err_with(|| format!("failed to start {operation}"))?;
    if !status.success() {
        bail!("{operation} failed: {status}");
    }
    Ok(())
}
