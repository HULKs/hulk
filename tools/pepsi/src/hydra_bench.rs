use std::path::{Component, PathBuf};

use argument_parsers::RobotAddress;
use clap::Args;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, ensure},
};
use pathdiff::diff_paths;
use repository::{Repository, upload::get_binary};
use robot::Robot;
use tempfile::tempdir;
use tokio::fs::{create_dir_all, symlink};

use crate::{
    cargo::{self, CargoCommand, build, cargo, environment::EnvironmentArguments},
    gammaray::CommandExt,
    progress_indicator::ProgressIndicator,
    tensorrt_compile,
};

#[derive(Args)]
pub struct Arguments {
    #[command(flatten)]
    pub hydra_bench: HydraBenchArguments,

    #[command(flatten)]
    pub environment: EnvironmentArguments,

    #[command(flatten, next_help_heading = "Cargo Options")]
    pub build: build::Arguments,
}

#[derive(Args)]
pub struct HydraBenchArguments {
    /// The robot to use for benchmarking
    pub host: RobotAddress,

    /// Paths to onnx models
    #[arg(required = true, num_args = 1..)]
    pub onnx_paths: Vec<PathBuf>,

    /// Local directory for JSON benchmark results
    #[arg(long, default_value = "target/hydra-bench-results")]
    pub output: PathBuf,

    /// Remote directory for JSON benchmark results, relative to ~/hulk
    #[arg(long, default_value = "logs/hydra-bench-results")]
    pub remote_output: PathBuf,

    /// Path to the TensorRT cache folder inside the runtime container
    #[arg(long, default_value = "/home/booster/hulk/etc/neural_networks/")]
    pub cache_path: PathBuf,

    /// Warmup inferences before measuring
    #[arg(long, default_value_t = 10)]
    pub warmup: usize,

    /// Measured inferences
    #[arg(short, long, default_value_t = 100)]
    pub iterations: usize,

    /// Number of TensorRT compile jobs to run concurrently
    #[arg(short, long, default_value_t = 1)]
    pub jobs: usize,

    /// Do not build before uploading
    #[arg(long)]
    pub no_build: bool,
}

pub async fn hydra_bench(arguments: Arguments, repository: &Repository) -> Result<()> {
    ensure!(
        !arguments.hydra_bench.remote_output.is_absolute(),
        "remote output directory must be relative to ~/hulk"
    );
    if arguments.hydra_bench.output.is_file() {
        color_eyre::eyre::bail!(
            "output must be a directory, but {} is a file",
            arguments.hydra_bench.output.display()
        );
    }
    let robot = Robot::try_new_with_ping(arguments.hydra_bench.host.ip).await?;
    let multiprogress = ProgressIndicator::new();
    let progress_build = multiprogress.task("Build hydra-bench", false);
    let progress_upload = multiprogress.task("Upload", false);
    let progress_compile = multiprogress.task("Compile TensorRT", false);
    let progress_benchmark = multiprogress.task("Benchmark", false);
    let progress_download = multiprogress.task("Download", false);

    let hydra_bench_binary_path = get_binary(arguments.build.profile(), "hydra-bench");
    let tensorrt_compile_binary_path =
        get_binary(arguments.build.profile(), tensorrt_compile::BINARY_NAME);
    if !arguments.hydra_bench.no_build {
        progress_build.enable_steady_tick();
        let hydra_bench_cargo_arguments = cargo::Arguments {
            manifest: Some(
                repository
                    .root
                    .join("tools/hydra-bench/Cargo.toml")
                    .into_os_string(),
            ),
            environment: arguments.environment.clone(),
            cargo: arguments.build.clone(),
        };
        cargo(
            hydra_bench_cargo_arguments,
            repository,
            &[&hydra_bench_binary_path],
        )
        .await
        .wrap_err("failed to build hydra-bench")
        .inspect_err(|_| progress_build.progress.finish())?;
        let tensorrt_compile_cargo_arguments = cargo::Arguments {
            manifest: Some(tensorrt_compile::manifest(repository)),
            environment: arguments.environment,
            cargo: arguments.build,
        };
        tensorrt_compile::build_binary(
            tensorrt_compile_cargo_arguments,
            repository,
            &progress_build,
        )
        .await
        .wrap_err("failed to build tensorrt-compile")
        .inspect_err(|_| progress_build.progress.finish())?;
        progress_build.finish_with_success(());
    } else {
        progress_build.finish_with_success("Skipped");
    }

    let onnx_paths = arguments
        .hydra_bench
        .onnx_paths
        .iter()
        .map(|onnx_path| relative_to_repository(onnx_path, repository))
        .collect::<Result<Vec<_>>>()?;

    let remote_output = &arguments.hydra_bench.remote_output;
    let remote_output_path = format!("hulk/{}", remote_output.display());
    let upload_directory = tempdir().wrap_err("failed to get temporary directory")?;
    progress_upload.enable_steady_tick();
    robot
        .ssh_to_robot()?
        .arg(format!("sudo rm -rf {}", shell_quote(&remote_output_path)))
        .run_with_log(
            "removing old benchmark results",
            &progress_upload.progress,
            b'\n',
        )
        .await
        .inspect_err(|_| progress_upload.progress.finish())?;
    repository
        .populate_upload_directory(
            &upload_directory,
            &[&hydra_bench_binary_path, &tensorrt_compile_binary_path],
        )
        .await
        .wrap_err("failed to populate upload directory")?;
    let uploaded_models = link_benchmark_models(&onnx_paths, &upload_directory, repository).await?;
    robot
        .upload(upload_directory, "hulk", true, |status| {
            progress_upload.set_message(format!("Uploading: {status}"))
        })
        .await
        .wrap_err_with(|| {
            format!(
                "failed to upload benchmark inputs to {}",
                arguments.hydra_bench.host.ip
            )
        })
        .inspect_err(|_| progress_upload.progress.finish())?;
    progress_upload.finish_with_success(());

    progress_benchmark.enable_steady_tick();
    robot
        .ssh_to_robot()?
        .arg(format!("mkdir -p {}", shell_quote(&remote_output_path)))
        .run_with_log(
            "preparing output directory",
            &progress_benchmark.progress,
            b'\n',
        )
        .await
        .inspect_err(|_| progress_benchmark.progress.finish())?;

    progress_compile.enable_steady_tick();
    tensorrt_compile::compile_models(
        &robot,
        &uploaded_models,
        arguments.hydra_bench.jobs,
        &progress_compile.progress,
    )
    .await
    .inspect_err(|_| progress_compile.progress.finish())?;
    progress_compile.finish_with_success(());

    let mut result_paths = Vec::new();
    for (onnx_path, uploaded_model) in onnx_paths.iter().zip(uploaded_models) {
        let result_path = remote_output.join(result_file_name(&onnx_path)?);
        let command = format!(
            "sudo podman exec --user $(id -u booster) hulk ./bin/hydra-bench --json --output {} --cache-path {} --warmup {} --iterations {} {}",
            shell_quote(&result_path.display().to_string()),
            shell_quote(&arguments.hydra_bench.cache_path.display().to_string()),
            arguments.hydra_bench.warmup,
            arguments.hydra_bench.iterations,
            shell_quote(&uploaded_model.display().to_string()),
        );
        robot
            .ssh_to_robot()?
            .arg(format!("cd hulk && {command}"))
            .run_with_log("benchmarking", &progress_benchmark.progress, b'\n')
            .await
            .inspect_err(|_| progress_benchmark.progress.finish())?;
        result_paths.push(result_path);
    }
    progress_benchmark.finish_with_success(());

    progress_download.enable_steady_tick();
    tensorrt_compile::download_neural_networks(&robot, repository, &progress_download.progress)
        .await
        .inspect_err(|_| progress_download.progress.finish())?;
    tokio::fs::create_dir_all(&arguments.hydra_bench.output)
        .await
        .wrap_err_with(|| {
            format!(
                "failed to create output directory {}",
                arguments.hydra_bench.output.display()
            )
        })?;
    for result_path in result_paths {
        robot
            .rsync_with_robot()?
            .arg("--mkpath")
            .arg("--info=progress2")
            .arg(format!("{}:hulk/{}", robot.address, result_path.display()))
            .arg(format!("{}/", arguments.hydra_bench.output.display()))
            .rsync_with_log("downloading benchmark results", &progress_download.progress)
            .await
            .inspect_err(|_| progress_download.progress.finish())?;
    }
    progress_download.finish_with_success(arguments.hydra_bench.output.display().to_string());

    Ok(())
}

fn relative_to_repository(path: &PathBuf, repository: &Repository) -> Result<PathBuf> {
    if path.is_absolute() {
        diff_paths(path, &repository.root).wrap_err("could not determine relative onnx path")
    } else {
        Ok(path.clone())
    }
}

fn result_file_name(onnx_path: &PathBuf) -> Result<String> {
    let mut name_parts = onnx_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(component) => component.to_str().map(sanitize_file_name_part),
            _ => None,
        })
        .collect::<Vec<_>>();
    let file_name = name_parts
        .pop()
        .wrap_err_with(|| format!("could not determine file name of {}", onnx_path.display()))?;
    let stem = file_name.strip_suffix(".onnx").unwrap_or(&file_name);
    name_parts.push(stem.to_string());
    Ok(format!("{}.json", name_parts.join("__")))
}

async fn link_benchmark_models(
    onnx_paths: &[PathBuf],
    upload_directory: impl AsRef<std::path::Path>,
    repository: &Repository,
) -> Result<Vec<PathBuf>> {
    let upload_directory = upload_directory.as_ref();
    let model_directory = PathBuf::from("benchmark-models");
    create_dir_all(upload_directory.join(&model_directory))
        .await
        .wrap_err("failed to create benchmark model upload directory")?;

    let mut uploaded_models = Vec::new();
    for onnx_path in onnx_paths {
        let uploaded_model =
            model_directory.join(result_file_name(onnx_path)?.replace(".json", ".onnx"));
        symlink(
            repository.root.join(onnx_path),
            upload_directory.join(&uploaded_model),
        )
        .await
        .wrap_err_with(|| format!("failed to link model {}", onnx_path.display()))?;
        uploaded_models.push(uploaded_model);
    }

    Ok(uploaded_models)
}

fn sanitize_file_name_part(part: &str) -> String {
    part.chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => character,
            _ => '_',
        })
        .collect()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
