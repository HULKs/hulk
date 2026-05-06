use std::{ffi::OsString, path::PathBuf, process::Stdio, time::Duration};

use clap::Args;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail, ensure, eyre},
};

use argument_parsers::RobotAddress;
use indicatif::ProgressBar;
use pathdiff::diff_paths;
use repository::{Repository, upload::get_binary};
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

pub const BINARY_NAME: &str = "tensorrt-compile";

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

    let binary_path = get_binary(arguments.build.profile(), BINARY_NAME);
    if !arguments.tensorrt_compile.no_build {
        progress_build.enable_steady_tick();
        let cargo_arguments = cargo::Arguments {
            manifest: Some(manifest(repository)),
            environment: arguments.environment,
            cargo: arguments.build,
        };
        build_binary(cargo_arguments, repository, &progress_build)
            .await
            .wrap_err("failed to build")
            .inspect_err(|_| progress_build.progress.finish())?;
        progress_build.finish_with_success(());
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
    let onnx_path = relative_to_repository(arguments.tensorrt_compile.onnx_path, repository)?;
    compile_model(&robot, &onnx_path, &progress_compile.progress)
        .await
        .inspect_err(|_| progress_compile.progress.finish())?;
    progress_compile.finish_with_success(progress_compile.progress.message());

    progress_download.enable_steady_tick();
    download_neural_networks(&robot, repository, &progress_download.progress)
        .await
        .inspect_err(|_| progress_download.progress.finish())?;
    progress_download.finish_with_success(());

    Ok(())
}

pub fn manifest(repository: &Repository) -> OsString {
    repository
        .root
        .join("tools/tensorrt-compile/Cargo.toml")
        .into_os_string()
}

pub fn relative_to_repository(onnx_path: PathBuf, repository: &Repository) -> Result<PathBuf> {
    if onnx_path.is_absolute() {
        diff_paths(onnx_path, &repository.root).wrap_err("could not determine relative onnx path")
    } else {
        Ok(onnx_path)
    }
}

pub async fn compile_models(
    robot: &Robot,
    onnx_paths: &[PathBuf],
    jobs: usize,
    progress_bar: &ProgressBar,
) -> Result<()> {
    ensure!(jobs > 0, "jobs must be > 0");
    if onnx_paths.len() == 1 {
        return compile_model(robot, &onnx_paths[0], progress_bar).await;
    }

    let mut command = String::from(
        "cd hulk || exit 1; \
         pids=; count=0; status=0",
    );
    for onnx_path in onnx_paths {
        let compile_command = format!(
            "sudo podman exec --user $(id -u booster) hulk ./bin/{BINARY_NAME} {}",
            shell_quote(&onnx_path.display().to_string()),
        );
        command.push_str(&format!(
            "; (printf 'compiling {}\\n'; {compile_command}) & \
             pids=\"$pids $!\"; count=$((count + 1)); \
             if [ \"$count\" -ge \"{jobs}\" ]; then \
                for pid in $pids; do wait $pid || status=1; done; \
                pids=; count=0; \
             fi",
            shell_escape_for_double_quotes(&onnx_path.display().to_string()),
        ));
    }
    command.push_str("; for pid in $pids; do wait $pid || status=1; done; exit $status");

    robot
        .ssh_to_robot()?
        .arg(command)
        .run_with_log("compiling TensorRT engines", progress_bar, b'\n')
        .await
        .wrap_err("failed to compile TensorRT engines")
}

pub async fn compile_model(
    robot: &Robot,
    onnx_path: &PathBuf,
    progress_bar: &ProgressBar,
) -> Result<()> {
    let compile_command = format!(
        "sudo podman exec --user $(id -u booster) hulk ./bin/{BINARY_NAME} {}",
        shell_quote(&onnx_path.display().to_string()),
    );
    let name = format!("compiling {}", onnx_path.display());
    robot
        .ssh_to_robot()?
        .arg(format!("cd hulk && {compile_command}"))
        .run_with_log(&name, progress_bar, b'\n')
        .await
}

pub async fn download_neural_networks(
    robot: &Robot,
    repository: &Repository,
    progress_bar: &ProgressBar,
) -> Result<()> {
    robot
        .rsync_with_robot()?
        .arg(format!("{}:hulk/etc/neural_networks/", robot.address))
        .arg(format!(
            "{}/",
            repository.root.join("etc/neural_networks").display()
        ))
        .rsync_with_log("downloading neural networks", progress_bar)
        .await
}

pub async fn build_binary(
    cargo_arguments: cargo::Arguments<build::Arguments>,
    repository: &Repository,
    progress_bar: &Task,
) -> Result<()> {
    let binary_path = get_binary(cargo_arguments.cargo.profile(), BINARY_NAME);
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
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_escape_for_double_quotes(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}
