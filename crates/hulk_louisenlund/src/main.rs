use std::{path::PathBuf, sync::Arc, time::Duration};

use clap::Parser;
use color_eyre::{Result, eyre::Context as _};
use ros_z::prelude::*;
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

mod arm_animator;
mod image_receiver;

const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Parser)]
struct Args {
    #[arg(
        long,
        help = "Robot graph namespace. Bare values like '42' become '/42'; invalid ros-z names are rejected."
    )]
    robot: String,
    #[arg(long)]
    location: String,
    #[arg(long, default_value = "parameters/ros_z")]
    parameter_root: PathBuf,
    #[arg(long)]
    router: Option<String>,
}

struct RunningStack {
    join_set: JoinSet<Result<()>>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    run_with_shutdown_timeout(run(), RUNTIME_SHUTDOWN_TIMEOUT)?
}

fn run_with_shutdown_timeout<F>(future: F, shutdown_timeout: Duration) -> Result<F::Output>
where
    F: Future,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to build Tokio runtime")?;
    let output = runtime.block_on(future);
    runtime.shutdown_timeout(shutdown_timeout);
    Ok(output)
}

async fn run() -> Result<()> {
    let args = Args::parse();
    let namespace = derive_namespace(&args.robot);
    let parameter_layers =
        derive_parameter_layers(&args.parameter_root, &args.location, &args.robot);

    let mut builder = ContextBuilder::default()
        .with_namespace(&namespace)
        .with_parameter_layers(parameter_layers);

    builder = match args.router {
        Some(router) => builder.with_mode("client").with_router_endpoint(router)?,
        None => builder
            .with_mode("router")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:7447"]),
    };

    let ctx = Arc::new(builder.build().await?);
    let mut running = spawn_all(ctx.clone()).await?;

    let result = tokio::select! {
        result = monitor(&mut running.join_set) => result,
        _ = tokio::signal::ctrl_c() => {
            Ok(())
        }
    };

    running.join_set.abort_all();
    if result.is_ok() {
        ctx.shutdown()?;
    }
    result
}

fn derive_parameter_layers(
    parameter_root: &std::path::Path,
    location: &str,
    robot: &str,
) -> Vec<PathBuf> {
    vec![
        parameter_root.join("base"),
        parameter_root.join("location").join(location),
        parameter_root.join("robot").join(robot),
    ]
}

fn derive_namespace(robot: &str) -> String {
    if robot.starts_with('/') {
        robot.to_string()
    } else {
        format!("/{robot}")
    }
}

async fn spawn_all(ctx: Arc<Context>) -> Result<RunningStack> {
    let mut join_set = JoinSet::new();

    join_set.spawn(arm_animator::run_boxed(ctx.clone()));
    join_set.spawn(image_receiver::run_boxed(ctx.clone()));
    join_set.spawn(camera_matrix_calculator::run_boxed(ctx.clone()));
    join_set.spawn(ground_provider::run_boxed(ctx.clone()));
    join_set.spawn(kinematics_provider::run_boxed(ctx.clone()));
    join_set.spawn(low_state_bridge::run_boxed(ctx.clone()));
    join_set.spawn(support_foot_estimator::run_boxed(ctx.clone()));

    Ok(RunningStack { join_set })
}

async fn monitor(join_set: &mut JoinSet<Result<()>>) -> Result<()> {
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(join_error) => return Err(join_error).wrap_err("monitor join failed"),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_namespace_prefixes_bare_robot_without_sanitizing() {
        assert_eq!(derive_namespace("42"), "/42");
        assert_eq!(derive_namespace("robot-01"), "/robot-01");
        assert_eq!(derive_namespace("robot//42"), "/robot//42");
        assert_eq!(derive_namespace("/robot/42"), "/robot/42");
        assert_eq!(derive_namespace("robot%01"), "/robot%01");
    }

    #[test]
    fn runtime_shutdown_timeout_does_not_wait_forever_for_blocking_tasks() {
        let (started_sender, started_receiver) = std::sync::mpsc::channel();
        let (release_sender, release_receiver) = std::sync::mpsc::channel::<()>();
        let started_at = std::time::Instant::now();

        let result = run_with_shutdown_timeout(
            async move {
                tokio::task::spawn_blocking(move || {
                    started_sender.send(()).expect("started signal should send");
                    let _ = release_receiver.recv();
                });
                started_receiver.recv().expect("blocking task should start");
            },
            std::time::Duration::from_millis(10),
        );

        drop(release_sender);
        result.expect("runtime should build and run");
        assert!(started_at.elapsed() < std::time::Duration::from_secs(1));
    }
}
