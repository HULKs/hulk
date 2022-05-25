use std::{net::Ipv4Addr, path::PathBuf, sync::Arc};

use anyhow::{bail, Context};
use log::info;
use tempfile::{tempdir, TempDir};
use tokio::{
    fs::{create_dir_all, symlink},
    process::Command,
};

use crate::commands::{
    build::BuildType,
    hulk::{self, hulk_service},
    logs::delete_logs,
};

pub async fn create_upload_directory(
    build_type: BuildType,
    exclude_configuration: bool,
    project_root: PathBuf,
) -> anyhow::Result<TempDir> {
    let upload_directory = tempdir().context("Failed to create temporary directory")?;
    let hulk_directory = upload_directory.path().join("hulk");
    create_dir_all(hulk_directory.join("bin"))
        .await
        .context("Failed to create directory")?;
    if !exclude_configuration {
        symlink(project_root.join("etc"), hulk_directory.join("etc")).await?;
    }
    let lower_case_build_type = build_type.to_string().to_lowercase();
    symlink(
        project_root.join(format!(
            "target/x86_64-aldebaran-linux/{}/nao",
            lower_case_build_type
        )),
        hulk_directory.join("bin/hulk"),
    )
    .await?;
    Ok(upload_directory)
}

pub async fn upload(
    nao: Ipv4Addr,
    restart_service_after_upload: bool,
    clean_target_before_upload: bool,
    directory_to_upload: Arc<TempDir>,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    hulk_service(nao, hulk::Command::Stop, project_root.clone())
        .await
        .context("Failed to stop HULKs service")?;

    let mut command = Command::new("rsync");
    command.args([
        "--times",
        "--recursive",
        "--compress",
        "--keep-dirlinks",
        "--copy-links",
        "--partial",
        "--progress",
        &format!(
            "--rsh=ssh -q -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -l nao -i {}",
            project_root.join("scripts/ssh_key").to_str().unwrap()
        ),
    ]);

    if clean_target_before_upload {
        command.args(["--delete", "--delete-excluded"]);
        delete_logs(nao, project_root.clone())
            .await
            .context("Failed to delete logs")?;
    }

    command.args([
        &format!("{}/hulk", directory_to_upload.path().to_str().unwrap()),
        &format!("{}:", nao),
    ]);
    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to upload to {}", nao))?;
    if !output.status.success() {
        bail!("rsync for {} exited with {}", nao, output.status);
    }
    info!("Uploaded to {}", nao);

    if restart_service_after_upload {
        hulk_service(nao, hulk::Command::Start, project_root).await?;
    }

    Ok(())
}
