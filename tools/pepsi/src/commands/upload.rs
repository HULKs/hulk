use std::{net::Ipv4Addr, path::PathBuf, sync::Arc};

use anyhow::Context;
use log::{debug, info};
use tempfile::{tempdir, TempDir};
use tokio::{
    fs::{create_dir_all, symlink},
    process::Command,
    try_join,
};

use crate::commands::{
    compile::BuildType,
    hulk::{self, hulk_service},
    logs::delete_logs,
};

pub async fn create_upload_directory(
    build_type: BuildType,
    exclude_configuration: bool,
    project_root: PathBuf,
) -> anyhow::Result<TempDir> {
    let upload_dir = tempdir()?;
    debug!(
        "Created temporary directory for upload at {:?}",
        upload_dir.path()
    );
    let naoqi = upload_dir.path().join("naoqi");
    create_dir_all(naoqi.join("bin")).await?;
    if !exclude_configuration {
        try_join!(
            symlink(
                project_root.join("etc/configuration"),
                naoqi.join("configuration"),
            ),
            symlink(
                project_root.join("etc/neuralnets"),
                naoqi.join("neuralnets"),
            )
        )?;
    }
    debug!("Using build-type: {:?}", build_type);
    try_join!(
        symlink(project_root.join("etc/motions"), naoqi.join("motions")),
        symlink(project_root.join("etc/poses"), naoqi.join("poses")),
        symlink(project_root.join("etc/sounds"), naoqi.join("sounds")),
        symlink(
            project_root.join(format!("build/NAO/{:?}/hulk", build_type)),
            naoqi.join("bin/hulk")
        )
    )?;
    Ok(upload_dir)
}

pub async fn upload(
    nao: Ipv4Addr,
    restart_service_after_upload: bool,
    clean_target_before_upload: bool,
    directory_to_upload: Arc<TempDir>,
    project_root: PathBuf,
) -> anyhow::Result<()> {
    hulk_service(nao, hulk::Command::Stop, project_root.clone()).await?;
    let mut command = Command::new("rsync");
    command.args([
        "-trzKLP",
        "--exclude=*webots*",
        "--exclude=*.gitkeep",
        "--exclude=*.touch",
        &format!(
            "--rsh=ssh -q -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -l nao -i {}",
            project_root.join("scripts/ssh_key").to_str().unwrap()
        ),
    ]);
    if clean_target_before_upload {
        command.args(["--delete", "--delete-excluded"]);
        delete_logs(nao, project_root.clone()).await?;
    }
    command.args([
        &format!("{}/naoqi", directory_to_upload.path().to_str().unwrap()),
        &format!("{}:", nao),
    ]);
    info!("Starting upload to {}", nao);
    let output = command
        .output()
        .await
        .with_context(|| format!("Upload to {} failed", nao))?;
    if !output.status.success() {
        anyhow::bail!("rsync for {} exited with {}", nao, output.status);
    }
    info!("Upload to {} finished", nao);
    if restart_service_after_upload {
        hulk_service(nao, hulk::Command::Start, project_root).await?;
    }
    Ok(())
}
