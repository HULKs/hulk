use color_eyre::{
    Result,
    eyre::{WrapErr, ensure},
};
use repository::Repository;
use tokio::process::Command;

pub async fn mujoco_viewer(repository: &Repository) -> Result<()> {
    let status = Command::new("uv")
        .current_dir(
            repository
                .root
                .join("tools/mujoco-simulator/mujoco-simulator"),
        )
        .args(["run", "viewer.py"])
        .status()
        .await
        .wrap_err("failed to launch the MuJoCo viewer with uv")?;
    ensure!(status.success(), "MuJoCo viewer exited with {status}");
    Ok(())
}
