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
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
};

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

    /// Arguments passed to tensorrt-compile, for example: -- --raw_bytes_input 224,272,6
    #[arg(value_name = "tensorrt-compile args", trailing_var_arg = true, num_args = 0..)]
    pub compile_args: Vec<String>,
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
    let compile_args = shell_args(
        std::iter::once(onnx_path.to_string_lossy().into_owned())
            .chain(arguments.tensorrt_compile.compile_args.iter().cloned()),
    );
    let mut command = robot.ssh_to_robot()?;
    command.arg(format!(
        "sudo podman exec hulk ./bin/tensorrt-compile {compile_args} 2>&1"
    ));
    run_with_printed_log(&mut command, &progress_compile)
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

async fn run_with_printed_log(command: &mut Command, task: &Task) -> Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut process = command.spawn().wrap_err("failed to spawn process")?;
    let stdout = process.stdout.take().wrap_err("failed to get stdout")?;
    let stderr = process.stderr.take().wrap_err("failed to get stderr")?;
    let (stdout, stderr) = tokio::try_join!(
        collect_log(stdout, task.progress.clone()),
        collect_log(stderr, task.progress.clone())
    )?;

    let status = process.wait().await?;
    if !status.success() {
        let message = status.code().map_or_else(
            || "process was killed".to_string(),
            |code| format!("process failed with code {code}"),
        );
        bail!("{message}\n{stdout}{stderr}");
    }
    Ok(())
}

async fn collect_log(
    reader: impl AsyncRead + Unpin,
    progress: indicatif::ProgressBar,
) -> Result<String> {
    let mut lines = BufReader::new(reader).lines();
    let mut log = String::new();
    while let Some(text) = lines.next_line().await? {
        progress.println(&text);
        progress.set_message(text.clone());
        log.push_str(&text);
        log.push('\n');
    }
    Ok(log)
}

fn shell_args(arguments: impl IntoIterator<Item = String>) -> String {
    arguments
        .into_iter()
        .map(|argument| shell_quote(&argument))
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_quote(argument: &str) -> String {
    if argument
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "_+-=.,/:".contains(character))
    {
        argument.to_string()
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
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
