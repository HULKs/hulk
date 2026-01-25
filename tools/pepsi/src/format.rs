use std::{path::Path, process::Stdio};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};

use repository::Repository;
use tokio::process::Command;

use crate::progress_indicator::ProgressIndicator;

async fn rustfmt(root: impl AsRef<Path>) -> Result<()> {
    let output = Command::new("sh")
        .stderr(Stdio::piped())
        .arg("-c")
        .arg(format!("cd {} && cargo fmt", root.as_ref().display()))
        .output()
        .await
        .wrap_err("rustfmt failed")?;

    if output.status.success() {
        return Ok(());
    }

    bail!(String::from_utf8(output.stderr).expect("stderr was not utf8"))
}

async fn taplo_fmt(root: impl AsRef<Path>) -> Result<()> {
    let output = Command::new("sh")
        .stderr(Stdio::piped())
        .arg("-c")
        .arg(format!(
            "cd {} && git ls-files -z '*.toml' | xargs -0 taplo fmt",
            root.as_ref().display()
        ))
        .output()
        .await
        .wrap_err("taplo fmt failed")?;

    if output.status.success() {
        return Ok(());
    }

    bail!(String::from_utf8(output.stderr).expect("stderr was not utf8"))
}

async fn ruff_fmt(root: impl AsRef<Path>) -> Result<()> {
    let output = Command::new("sh")
        .stderr(Stdio::piped())
        .arg("-c")
        .arg(format!(
            "cd {} && git ls-files -z '*.py' '*.pyi' | xargs -0 uvx ruff format",
            root.as_ref().display()
        ))
        .output()
        .await
        .wrap_err("taplo fmt failed")?;

    if output.status.success() {
        return Ok(());
    }

    bail!(String::from_utf8(output.stderr).expect("stderr was not utf8"))
}

pub async fn format(repository: &Repository) -> Result<()> {
    let progress_indicator = ProgressIndicator::new();

    tokio::join!(
        async {
            let task = progress_indicator.task("rustfmt");
            task.finish_with(rustfmt(&repository.root).await);
        },
        async {
            let task = progress_indicator.task("taplo");
            task.finish_with(taplo_fmt(&repository.root).await);
        },
        async {
            let task = progress_indicator.task("ruff");
            task.finish_with(ruff_fmt(&repository.root).await);
        }
    );

    Ok(())
}
