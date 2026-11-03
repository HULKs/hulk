use std::{
    env,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use clap::Parser;
use color_eyre::{
    Result,
    eyre::{Context as _, ContextCompat, bail, eyre},
};
use repository::{Repository, team::Team};
use ros_z::prelude::*;
use tokio::{
    task::{JoinHandle, JoinSet},
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

mod motion_runtime;

const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const RECORDER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    location: String,
    #[arg(long, default_value = "parameters")]
    parameter_root: PathBuf,
    #[arg(long)]
    router: Option<String>,
    #[arg(long)]
    log_path: Option<PathBuf>,
    /// Run motion, sensor input, and safety nodes on these Linux CPUs (comma-separated).
    #[arg(long, value_delimiter = ',')]
    motion_cpus: Vec<usize>,
}

struct RunningStack {
    join_set: JoinSet<Result<()>>,
    recorder: Option<JoinHandle<Result<()>>>,
    stop: CancellationToken,
}

impl RunningStack {
    async fn shutdown(&mut self) -> Result<()> {
        self.stop.cancel();
        self.join_set.abort_all();
        let Some(mut recorder) = self.recorder.take() else {
            return Ok(());
        };
        match timeout(RECORDER_SHUTDOWN_TIMEOUT, &mut recorder).await {
            Ok(result) => result.wrap_err("MCAP recorder task failed to join")?,
            Err(_) => {
                recorder.abort();
                bail!(
                    "MCAP finalization exceeded {}s. Recording may need recovery",
                    RECORDER_SHUTDOWN_TIMEOUT.as_secs()
                );
            }
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let (motion_runtimes, thread_starts) = if args.motion_cpus.is_empty() {
        (Vec::new(), None)
    } else {
        let motion = motion_runtime::build(&args.motion_cpus)?;
        (motion.runtimes, Some(motion.thread_starts))
    };
    let motion_handles = motion_runtimes
        .iter()
        .map(|runtime| runtime.handle().clone())
        .collect();
    run_with_shutdown_timeout(
        run(args, motion_handles, thread_starts),
        motion_runtimes,
        RUNTIME_SHUTDOWN_TIMEOUT,
    )?
}

fn run_with_shutdown_timeout<F>(
    future: F,
    motion_runtimes: Vec<tokio::runtime::Runtime>,
    shutdown_timeout: Duration,
) -> Result<F::Output>
where
    F: Future,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to build Tokio runtime")?;
    let output = runtime.block_on(future);
    let deadline = Instant::now() + shutdown_timeout;
    for motion_runtime in motion_runtimes {
        motion_runtime.shutdown_timeout(deadline.saturating_duration_since(Instant::now()));
    }
    runtime.shutdown_timeout(shutdown_timeout);
    Ok(output)
}

async fn run(
    args: Args,
    motion_handles: Vec<tokio::runtime::Handle>,
    thread_starts: Option<tokio::sync::mpsc::UnboundedReceiver<Result<()>>>,
) -> Result<()> {
    let Some(hardware_id) = env::var_os("HARDWARE_ID") else {
        bail!("environment variable HARDWARE_ID not set");
    };
    let hardware_id = hardware_id
        .into_string()
        .ok()
        .wrap_err("id was not valid UTF-8")?;
    let robot_number = load_robot_number(&hardware_id).await?;
    let namespace = derive_namespace(&robot_number.to_string());

    let parameter_layers =
        derive_parameter_layers(&args.parameter_root, &args.location, &hardware_id);

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
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .wrap_err("failed to register SIGTERM handler")?;
    let mut running = spawn_all(ctx.clone(), args.log_path, motion_handles).await?;
    if let Some(mut thread_starts) = thread_starts {
        running.join_set.spawn(async move {
            while let Some(result) = thread_starts.recv().await {
                result?;
            }
            bail!("motion runtime stopped unexpectedly")
        });
    }

    let result = tokio::select! {
        result = monitor(&mut running) => result,
        result = tokio::signal::ctrl_c() => result.wrap_err("Ctrl-C handler failed"),
        _ = terminate.recv() => Ok(()),
    };

    let recorder_result = running.shutdown().await;
    if let Err(error) = &recorder_result {
        tracing::error!(?error, "MCAP recorder shutdown failed");
    }
    let result = result.and(recorder_result);
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

async fn load_robot_number(hardware_id: &str) -> Result<u8> {
    let repository =
        Repository::new(env::current_dir().wrap_err("failed to get current directory")?);
    let team = repository.read_team_configuration().await?;
    robot_number_for_hardware_id(&team, hardware_id)
}

fn robot_number_for_hardware_id(team: &Team, hardware_id: &str) -> Result<u8> {
    team.robots
        .iter()
        .find(|robot| robot.id == hardware_id)
        .map(|robot| robot.number)
        .ok_or_else(|| eyre!(r#"ID "{hardware_id}" not found in team.toml"#))
}

fn derive_namespace(robot: &str) -> String {
    if robot.starts_with('/') {
        robot.to_string()
    } else {
        format!("/{robot}")
    }
}

async fn spawn_all(
    ctx: Arc<Context>,
    log_path: Option<PathBuf>,
    mut motion_handles: Vec<tokio::runtime::Handle>,
) -> Result<RunningStack> {
    let mut join_set = JoinSet::new();
    let stop = CancellationToken::new();

    if motion_handles.is_empty() {
        motion_handles.push(tokio::runtime::Handle::current());
    }
    type NodeMain = fn(Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
    let motion_nodes: [NodeMain; 11] = [
        button_event_bridge::run_boxed,
        button_event_handler::run_boxed,
        controller_handler::run_boxed,
        fall_detection::run_boxed,
        hardware_interface::run_boxed,
        head_motion::run_boxed,
        low_state_bridge::run_boxed,
        motion::run_boxed,
        motion_inference::run_boxed,
        primary_state_filter::run_boxed,
        safe_pose_checker::run_boxed,
    ];
    // Assign nodes, rather than new threads, across CPU-owned runtimes. Nested
    // tasks and blocking-pool handoffs then retain each node's assigned CPU.
    for (run, handle) in motion_nodes.into_iter().zip(motion_handles.iter().cycle()) {
        join_set.spawn_on(run(ctx.clone()), handle);
    }

    // Behavior includes tactical decisions and Voronoi computation, so it stays
    // on the general runtime alongside perception and planning.
    join_set.spawn(active_vision::run_boxed(ctx.clone()));
    join_set.spawn(ball_filter::run_boxed(ctx.clone()));
    join_set.spawn(ball_state_composer::run_boxed(ctx.clone()));
    join_set.spawn(behavior_node::run_boxed(ctx.clone()));
    join_set.spawn(camera_matrix_calculator::run_boxed(ctx.clone()));
    join_set.spawn(detection::run_boxed(ctx.clone()));
    join_set.spawn(fall_down_state_receiver::run_boxed(ctx.clone()));
    join_set.spawn(field_mark_association::run_boxed(ctx.clone()));
    join_set.spawn(game_controller_filter::run_boxed(ctx.clone()));
    join_set.spawn(game_controller_state_filter::run_boxed(ctx.clone()));
    join_set.spawn(global_parameter_provider::run_boxed(ctx.clone()));
    join_set.spawn(ground_provider::run_boxed(ctx.clone()));
    join_set.spawn(image_receiver::run_boxed(ctx.clone()));
    join_set.spawn(kinematics_provider::run_boxed(ctx.clone()));
    join_set.spawn(led_handler::run_boxed(ctx.clone()));
    join_set.spawn(localization_2d::run_boxed(ctx.clone()));
    join_set.spawn(localization_3d::run_boxed(ctx.clone()));
    let recorder = tokio::spawn(mcap_recorder::run_boxed(
        ctx.clone(),
        log_path,
        stop.child_token(),
    ));
    join_set.spawn(message_filter::run_boxed(ctx.clone()));
    join_set.spawn(message_handler::run_boxed(ctx.clone()));
    join_set.spawn(microphone_recorder::run_boxed(ctx.clone()));
    join_set.spawn(motor_commands_collector::run_boxed(ctx.clone()));
    join_set.spawn(obstacle_filter::run_boxed(ctx.clone()));
    join_set.spawn(odometer_bridge::run_boxed(ctx.clone()));
    join_set.spawn(odometry::run_boxed(ctx.clone()));
    join_set.spawn(player_states_receiver::run_boxed(ctx.clone()));
    join_set.spawn(visual_kick_ball_selector::run_boxed(ctx.clone()));
    join_set.spawn(rule_obstacle_composer::run_boxed(ctx.clone()));
    join_set.spawn(search_suggestor::run_boxed(ctx.clone()));
    join_set.spawn(stereo_visual_odometry::run_boxed(ctx.clone()));
    join_set.spawn(support_foot_estimator::run_boxed(ctx.clone()));
    join_set.spawn(system_info::run_boxed(ctx.clone()));
    join_set.spawn(team_ball_filter::run_boxed(ctx.clone()));
    join_set.spawn(time_to_reach_kick_position::run_boxed(ctx.clone()));
    join_set.spawn(trigger::run_boxed(ctx.clone()));
    join_set.spawn(whistle_detection::run_boxed(ctx.clone()));
    join_set.spawn(whistle_filter::run_boxed(ctx.clone()));
    join_set.spawn(world_state_composer::run_boxed(ctx.clone()));
    join_set.spawn(world_to_field_provider::run_boxed(ctx.clone()));

    Ok(RunningStack {
        join_set,
        recorder: Some(recorder),
        stop,
    })
}

async fn monitor(running: &mut RunningStack) -> Result<()> {
    loop {
        let result = tokio::select! {
            result = running.join_set.join_next() => {
                let Some(result) = result else { return Ok(()); };
                result
            }
            result = async {
                match &mut running.recorder {
                    Some(recorder) => recorder.await,
                    None => std::future::pending().await,
                }
            } => {
                running.recorder = None;
                result
            }
        };
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(join_error) => return Err(join_error).wrap_err("monitor join failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_cpus_accepts_a_list_and_defaults_to_shared_runtime() {
        let args =
            Args::try_parse_from(["hulk_ros_z", "--location", "test", "--motion-cpus", "4,5"])
                .unwrap();
        assert_eq!(args.motion_cpus, [4, 5]);
        let args = Args::try_parse_from(["hulk_ros_z", "--location", "test"]).unwrap();
        assert!(args.motion_cpus.is_empty());
    }

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
        for motion_runtime_count in [0, 1, 2] {
            check_blocking_shutdown(motion_runtime_count);
        }
    }

    fn check_blocking_shutdown(motion_runtime_count: usize) {
        let motion_runtimes: Vec<_> = (0..motion_runtime_count)
            .map(|_| {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()
                    .unwrap()
            })
            .collect();
        let mut motion_handles: Vec<_> = motion_runtimes
            .iter()
            .map(|runtime| runtime.handle().clone())
            .collect();
        let (started_sender, started_receiver) = std::sync::mpsc::channel();
        let mut releases = Vec::new();
        let started_at = std::time::Instant::now();

        let result = run_with_shutdown_timeout(
            async move {
                if motion_handles.is_empty() {
                    motion_handles.push(tokio::runtime::Handle::current());
                }
                let count = motion_handles.len();
                for handle in motion_handles {
                    let started = started_sender.clone();
                    let (release, wait) = std::sync::mpsc::channel::<()>();
                    releases.push(release);
                    handle.spawn_blocking(move || {
                        started.send(()).expect("started signal should send");
                        let _ = wait.recv();
                    });
                }
                for _ in 0..count {
                    started_receiver.recv().expect("blocking task should start");
                }
                releases
            },
            motion_runtimes,
            std::time::Duration::from_millis(10),
        );

        drop(result.expect("runtime should build and run"));
        assert!(started_at.elapsed() < std::time::Duration::from_secs(1));
    }
}
