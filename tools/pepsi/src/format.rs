use std::path::Path;

use clap::Args;
use color_eyre::{
    Result,
    eyre::{Context, bail},
};

use repository::Repository;
use tokio::process::Command;

use crate::{progress_indicator::ProgressIndicator, sdk};

#[derive(Args)]
pub struct Arguments {
    /// Run only in check mode and do not make changes in the filesystem
    #[arg(long, visible_alias = "prüf")]
    check: bool,
}

async fn rustfmt(root: impl AsRef<Path>, check: bool) -> Result<()> {
    let mut command = Command::new("cargo");
    command.current_dir(root.as_ref()).arg("fmt");
    if check {
        command.arg("--check");
    }
    let output = command.output().await.wrap_err("rustfmt failed")?;

    if output.status.success() {
        return Ok(());
    }

    bail!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

async fn taplo_fmt(root: impl AsRef<Path>, check: bool) -> Result<()> {
    let format_or_check = if check { "check" } else { "format" };
    let output = Command::new("bash")
        .current_dir(root.as_ref())
        .args(["-o", "pipefail", "-c"])
        .arg(format!(
            "git ls-files -z '*.toml' | xargs -0r taplo {format_or_check}"
        ))
        .output()
        .await
        .wrap_err("taplo fmt failed")?;

    if output.status.success() {
        return Ok(());
    }

    bail!(String::from_utf8(output.stderr).expect("stderr was not utf8"))
}

async fn ruff_fmt(root: impl AsRef<Path>, check: bool) -> Result<()> {
    let check_flag = if check { "--check" } else { "" };
    let output = Command::new("bash")
        .current_dir(root.as_ref())
        .args(["-o", "pipefail", "-c"])
        .arg(format!(
            "git ls-files -z '*.py' '*.pyi' | xargs -0r ruff format {check_flag}"
        ))
        .output()
        .await
        .wrap_err("ruff fmt failed")?;

    if output.status.success() {
        return Ok(());
    }

    bail!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Format the repository or check its formatting in the workstation Nix shell.
pub async fn format(arguments: Arguments, repository: &Repository) -> Result<()> {
    if !sdk::in_native_shell() {
        let mut command = sdk::native_command(&repository.root, std::env::current_exe()?);
        command.current_dir(&repository.root).arg("fmt");
        if arguments.check {
            command.arg("--check");
        }
        let status = command
            .status()
            .await
            .wrap_err("failed to run formatters")?;
        if !status.success() {
            bail!("formatting failed with {status}");
        }
        return Ok(());
    }

    let progress_indicator = ProgressIndicator::new();

    let (rustfmt_result, taplo_result, ruff_result) = tokio::join!(
        async {
            let task = progress_indicator.task("rustfmt", true);
            let result = rustfmt(&repository.root, arguments.check).await;
            task.finish_with(result.as_ref());
            result
        },
        async {
            let task = progress_indicator.task("taplo", true);
            let result = taplo_fmt(&repository.root, arguments.check).await;
            task.finish_with(result.as_ref());
            result
        },
        async {
            let task = progress_indicator.task("ruff", true);
            let result = ruff_fmt(&repository.root, arguments.check).await;
            task.finish_with(result.as_ref());
            result
        }
    );

    if rustfmt_result.is_err() || taplo_result.is_err() || ruff_result.is_err() {
        bail!("formatting failed");
    }

    Ok(())
}
