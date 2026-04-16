use std::{path::PathBuf, process::Stdio, time::Duration};

use clap::Args;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail, eyre},
};

use argument_parsers::RobotAddress;
use pathdiff::diff_paths;
use repository::Repository;
use robot::Robot;
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::{
    cargo::{
        self, CargoCommand, build, construct_cargo_command, environment::EnvironmentArguments,
    },
    gammaray::CommandExt,
    progress_indicator::{ProgressIndicator, Task},
};

#[derive(Args)]
pub struct Arguments {
    #[command(flatten)]
    pub tensorrt_compile: TensorRtCompileArguments,

    #[command(flatten)]
    pub environment: EnvironmentArguments,
    #[command(flatten, next_help_heading = "Cargo Options")]
    pub build: build::Arguments,
}

#[derive(Args)]
pub struct TensorRtCompileArguments {
    /// Path to onnx model
    pub onnx_path: PathBuf,

    // The robot to use for compiling the model
    pub host: RobotAddress,

    /// Do not build before uploading
    #[arg(long)]
    pub no_build: bool,
}

pub async fn tensorrt_compile(arguments: Arguments, repository: &Repository) -> Result<()> {
    let robot = Robot::try_new_with_ping(arguments.tensorrt_compile.host.ip).await?;
    let multiprogres = ProgressIndicator::new();
    let progress_build = multiprogres.task("Build compiler", false);
    let progress_upload = multiprogres.task("Upload", false);
    let progress_compile = multiprogres.task("Compile", false);
    let progress_download = multiprogres.task("Download", false);

    let binary_path = get_binary_path(arguments.build.profile());
    if !arguments.tensorrt_compile.no_build {
        progress_build.enable_steady_tick();
        let cargo_arguments = cargo::Arguments {
            manifest: Some(
                repository
                    .root
                    .join("tools/tensorrt-compile/Cargo.toml")
                    .into_os_string(),
            ),
            environment: arguments.environment,
            cargo: arguments.build,
        };
        build_binary(cargo_arguments, repository, &progress_build)
            .await
            .wrap_err("failed to build")
            .inspect_err(|_| progress_build.progress.finish())?;
    } else {
        progress_build.finish_with_success("Skipped");
    }

    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    progress_upload.enable_steady_tick();
    tokio::time::sleep(Duration::from_secs(1)).await;
    repository
        .populate_upload_directory(&upload_directory, &[&binary_path])
        .await
        .wrap_err("failed to populate upload directory")?;
    robot
        .upload(upload_directory, "hulk", true, |status| {
            progress_upload.set_message(format!("Uploading: {status}"))
        })
        .await
        .wrap_err_with(|| {
            format!(
                "failed to upload binary to {}",
                arguments.tensorrt_compile.host.ip
            )
        })
        .inspect_err(|_| progress_upload.progress.finish())?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    progress_upload.finish_with_success(());

    progress_compile.enable_steady_tick();
    let onnx_path = if arguments.tensorrt_compile.onnx_path.is_absolute() {
        diff_paths(arguments.tensorrt_compile.onnx_path, &repository.root)
            .wrap_err("could not determinte relative onnx path")?
    } else {
        arguments.tensorrt_compile.onnx_path
    };
    robot
        .ssh_to_robot()?
        .arg(format!(
            r#"launch-hulk --executable "./bin/tensorrt-compile {}" 2>&1"#,
            onnx_path.display(),
        ))
        .run_with_log("", &progress_compile.progress, b'\n')
        .await
        .inspect_err(|_| progress_compile.progress.finish())?;
    progress_compile.finish_with_success(progress_compile.progress.message());

    progress_download.enable_steady_tick();
    robot
        .rsync_with_robot()?
        .arg(format!("{}:hulk/etc/neural_networks/", robot.address))
        .arg(format!(
            "{}/",
            repository.root.join("etc/neural_networks").display()
        ))
        .rsync_with_log("", &progress_download.progress)
        .await
        .inspect_err(|_| progress_download.progress.finish())?;
    progress_download.finish_with_success(());

    Ok(())
}

pub fn get_binary_path(profile: &str) -> String {
    // the target directory is "debug" with --profile dev...
    let profile_directory = match profile {
        "dev" => "debug",
        other => other,
    };

    format!("target/aarch64-unknown-linux-gnu/{profile_directory}/tensorrt-compile")
}

async fn build_binary(
    cargo_arguments: cargo::Arguments<build::Arguments>,
    repository: &Repository,
    progress_bar: &Task,
) -> Result<()> {
    let binary_path = get_binary_path(cargo_arguments.cargo.profile());
    let mut command = construct_cargo_command(cargo_arguments, repository, &[binary_path])
        .await
        .expect("failed to construct cargo command");

    command.stdout(Stdio::null());
    command.stderr(Stdio::piped());
    command.stdin(Stdio::null());
    command.kill_on_drop(true);

    let mut process = command.spawn().unwrap();

    process.stdin.take();
    process.stdout.take();
    let mut lines = BufReader::new(process.stderr.take().unwrap()).lines();
    while let Ok(Some(text)) = lines.next_line().await {
        progress_bar.progress.println(&text);
    }
    let status = process.wait().await.unwrap();
    if !status.success() {
        progress_bar.finish_with_error(&eyre!("failed with code {}", status.code().unwrap()));
        bail!("process failed");
    }
    progress_bar.finish_with_success(());

    Ok(())
}
