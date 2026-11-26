use std::{env, future::Future, path::PathBuf, sync::Arc, time::Duration};

use clap::Parser;
use color_eyre::{
    Result, Section,
    eyre::{Context as _, ContextCompat, Report, bail, eyre},
};
use repository::{Repository, team::Team};
use ros_z::prelude::*;
use tokio::{
    task::{JoinHandle, JoinSet},
    time::timeout,
};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

mod booster_session;
mod motion_runtime;

use booster_session::open_booster_session;

const NODE_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const RECORDER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(8);
const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

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
    recorder_stop: CancellationToken,
}

impl RunningStack {
    async fn stop_tasks(&mut self) -> Vec<Report> {
        self.join_set.abort_all();

        let mut errors = Vec::new();
        let drain = async {
            while let Some(result) = self.join_set.join_next().await {
                match result {
                    Ok(Ok(())) => {}
                    Err(error) if error.is_cancelled() => {}
                    Ok(Err(error)) => {
                        errors.push(error.wrap_err("node task failed during shutdown"))
                    }
                    Err(error) => errors.push(
                        Report::new(error).wrap_err("node task failed to join during shutdown"),
                    ),
                }
            }
        };
        if timeout(NODE_SHUTDOWN_TIMEOUT, drain).await.is_err() {
            errors.push(eyre!(
                "node shutdown timed out: {} tasks remaining",
                self.join_set.len()
            ));
        }

        self.recorder_stop.cancel();
        if let Some(mut recorder) = self.recorder.take() {
            match timeout(RECORDER_SHUTDOWN_TIMEOUT, &mut recorder).await {
                Ok(Ok(Ok(()))) => {}
                Ok(Ok(Err(error))) => errors.push(error.wrap_err("MCAP recorder shutdown failed")),
                Ok(Err(error)) => {
                    errors.push(Report::new(error).wrap_err("MCAP recorder task failed to join"))
                }
                Err(_) => {
                    recorder.abort();
                    errors.push(eyre!(
                        "MCAP finalization exceeded {}s. Recording may need recovery",
                        RECORDER_SHUTDOWN_TIMEOUT.as_secs()
                    ));
                }
            }
        }
        errors
    }
}

fn report_errors(mut errors: Vec<Report>) -> Result<()> {
    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.remove(0)),
        count => {
            let mut summary = eyre!("{count} failures while stopping HULK");
            for error in errors {
                summary = summary.section(format!("Failure: {error:#}"));
            }
            Err(summary)
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let (motion_runtime, thread_starts) = if args.motion_cpus.is_empty() {
        (None, None)
    } else {
        let motion = motion_runtime::build(&args.motion_cpus)?;
        (Some(motion.runtime), Some(motion.thread_starts))
    };
    let motion_handle = motion_runtime
        .as_ref()
        .map(|runtime| runtime.handle().clone());
    run_with_shutdown_timeout(
        run(args, motion_handle, thread_starts),
        motion_runtime,
        RUNTIME_SHUTDOWN_TIMEOUT,
    )?
}

fn run_with_shutdown_timeout<F>(
    future: F,
    motion_runtime: Option<tokio::runtime::Runtime>,
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
    if let Some(motion_runtime) = motion_runtime {
        motion_runtime.shutdown_timeout(shutdown_timeout);
    }
    runtime.shutdown_timeout(shutdown_timeout);
    Ok(output)
}

async fn run(
    args: Args,
    motion_handle: Option<tokio::runtime::Handle>,
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
    let booster_session = open_booster_session().await?;
    let mut running =
        spawn_all(ctx.clone(), &booster_session, args.log_path, motion_handle).await?;
    if let Some(mut thread_starts) = thread_starts {
        running.join_set.spawn(async move {
            while let Some(result) = thread_starts.recv().await {
                result?;
            }
            bail!("motion runtime stopped unexpectedly")
        });
    }

    let run_result = tokio::select! {
        result = monitor(&mut running) => result,
        result = tokio::signal::ctrl_c() => result.wrap_err("Ctrl-C handler failed"),
        _ = terminate.recv() => Ok(()),
    };

    if let Err(error) = &run_result {
        tracing::error!(?error, "HULK failed; shutting down");
    }

    let mut errors = running.stop_tasks().await;
    if errors.is_empty() {
        let close_result = ctx.shutdown().wrap_err("failed to close ROS-Z context");
        if let Err(error) = close_result {
            errors.push(error);
        }
    }
    if let Err(error) = run_result {
        errors.insert(0, error);
    }
    report_errors(errors)
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
    booster_session: &zenoh::Session,
    log_path: Option<PathBuf>,
    motion_handle: Option<tokio::runtime::Handle>,
) -> Result<RunningStack> {
    let mut join_set = JoinSet::new();
    let recorder_stop = CancellationToken::new();

    let motion_handle = motion_handle.unwrap_or_else(tokio::runtime::Handle::current);
    // Poll these nodes on the isolated runtime from the start, so their nested
    // tasks and lazily created blocking/inference threads inherit its affinity.
    join_set.spawn_on(
        button_event_bridge::run_boxed(ctx.clone(), booster_session.clone()),
        &motion_handle,
    );
    join_set.spawn_on(button_event_handler::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(controller_handler::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(fall_detection::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(
        hardware_interface::run_boxed(ctx.clone(), booster_session.clone()),
        &motion_handle,
    );
    join_set.spawn_on(head_motion::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(
        low_state_bridge::run_boxed(ctx.clone(), booster_session.clone()),
        &motion_handle,
    );
    join_set.spawn_on(motion::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(motion_inference::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(primary_state_filter::run_boxed(ctx.clone()), &motion_handle);
    join_set.spawn_on(safe_pose_checker::run_boxed(ctx.clone()), &motion_handle);

    // Behavior includes tactical decisions and Voronoi computation, so it stays
    // on the general runtime alongside perception and planning.
    join_set.spawn(active_vision::run_boxed(ctx.clone()));
    join_set.spawn(ball_filter::run_boxed(ctx.clone()));
    join_set.spawn(ball_state_composer::run_boxed(ctx.clone()));
    join_set.spawn(behavior_node::run_boxed(ctx.clone()));
    join_set.spawn(camera_matrix_calculator::run_boxed(ctx.clone()));
    join_set.spawn(detection::run_boxed(ctx.clone()));
    join_set.spawn(fall_down_state_receiver::run_boxed(
        ctx.clone(),
        booster_session.clone(),
    ));
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
        recorder_stop.child_token(),
    ));
    join_set.spawn(message_filter::run_boxed(ctx.clone()));
    join_set.spawn(message_handler::run_boxed(ctx.clone()));
    join_set.spawn(microphone_recorder::run_boxed(ctx.clone()));
    join_set.spawn(motor_commands_collector::run_boxed(ctx.clone()));
    join_set.spawn(obstacle_filter::run_boxed(ctx.clone()));
    join_set.spawn(odometer_bridge::run_boxed(
        ctx.clone(),
        booster_session.clone(),
    ));
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
        recorder_stop,
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn shutdown_drains_blocked_node_before_stopping_recorder() {
        let mut join_set = JoinSet::new();
        let (started, entered) = tokio::sync::oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel::<()>();
        join_set.spawn(async move {
            tokio::task::block_in_place(|| {
                started.send(()).unwrap();
                let _ = gate.recv();
            });
            std::future::pending().await
        });
        entered.await.unwrap();
        let recorder_stop = CancellationToken::new();
        let token = recorder_stop.child_token();
        let recorder = tokio::spawn(async move {
            token.cancelled().await;
            bail!("recorder finalization failed in test")
        });
        let mut stack = RunningStack {
            join_set,
            recorder: Some(recorder),
            recorder_stop: recorder_stop.clone(),
        };

        let mut shutdown = std::pin::pin!(stack.stop_tasks());
        tokio::select! {
            biased;
            result = &mut shutdown => panic!("shutdown returned before node release: {result:?}"),
            () = std::future::ready(()) => {}
        }
        assert!(!recorder_stop.is_cancelled());
        drop(release);
        let errors = shutdown.await;
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].root_cause().to_string(),
            "recorder finalization failed in test"
        );
    }

    #[test]
    fn runtime_shutdown_timeout_does_not_wait_forever_for_blocking_tasks() {
        for separate_motion_runtime in [false, true] {
            check_blocking_shutdown(separate_motion_runtime);
        }
    }

    fn check_blocking_shutdown(separate_motion_runtime: bool) {
        let motion_runtime = separate_motion_runtime.then(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap()
        });
        let motion_handle = motion_runtime
            .as_ref()
            .map(|runtime| runtime.handle().clone());
        let (started_sender, started_receiver) = std::sync::mpsc::channel();
        let (release_sender, release_receiver) = std::sync::mpsc::channel::<()>();
        let started_at = std::time::Instant::now();

        let result = run_with_shutdown_timeout(
            async move {
                let handle = motion_handle.unwrap_or_else(tokio::runtime::Handle::current);
                handle.spawn_blocking(move || {
                    started_sender.send(()).expect("started signal should send");
                    let _ = release_receiver.recv();
                });
                started_receiver.recv().expect("blocking task should start");
            },
            motion_runtime,
            std::time::Duration::from_millis(10),
        );

        drop(release_sender);
        result.expect("runtime should build and run");
        assert!(started_at.elapsed() < std::time::Duration::from_secs(1));
    }
}
