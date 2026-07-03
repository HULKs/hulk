use std::{env, future::Future, net::Ipv4Addr, path::PathBuf, sync::Arc, time::Duration};

use clap::Parser;
use color_eyre::{
    Result,
    eyre::{Context as _, ContextCompat, bail, eyre},
};
use repository::{Repository, team::Team};
use ros_z::prelude::*;
use tokio::task::JoinSet;
use tracing::Instrument;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

const RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    location: String,
    #[arg(long, default_value = "parameters/ros_z")]
    parameter_root: PathBuf,
    #[arg(long)]
    router: Option<String>,
    #[arg(long)]
    log_path: Option<PathBuf>,
}

struct RunningStack {
    join_set: JoinSet<Result<()>>,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    let console_layer = console_subscriber::Builder::default()
        .server_addr((Ipv4Addr::UNSPECIFIED, 6669))
        .spawn();

    let env_filter = EnvFilter::from_default_env()
        .add_directive("tokio=trace".parse()?)
        .add_directive("runtime=trace".parse()?);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(console_layer)
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
    let mut running = spawn_all(ctx.clone(), args.log_path).await?;

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

async fn spawn_all(ctx: Arc<Context>, log_path: Option<PathBuf>) -> Result<RunningStack> {
    let mut join_set = JoinSet::new();

    macro_rules! spawn_node {
        ($node:ident $(, $arg:expr)?) => {
            spawn_node_task(
                &mut join_set,
                stringify!($node),
                $node::run_boxed(ctx.clone() $(, $arg)?),
            );
        };
    }

    spawn_node!(active_vision);
    spawn_node!(ball_filter);
    spawn_node!(ball_state_composer);
    spawn_node!(behavior_node);
    spawn_node!(booster_sdk_interface);
    spawn_node!(button_event_bridge);
    spawn_node!(button_event_handler);
    spawn_node!(camera_matrix_calculator);
    spawn_node!(detection);
    spawn_node!(fake_odometry);
    spawn_node!(fall_down_state_receiver);
    spawn_node!(field_mark_association);
    spawn_node!(game_controller_filter);
    spawn_node!(game_controller_state_filter);
    spawn_node!(global_parameter_provider);
    spawn_node!(ground_provider);
    spawn_node!(head_motion);
    spawn_node!(image_receiver);
    spawn_node!(kinematics_provider);
    spawn_node!(led_handler);
    spawn_node!(localization_2d);
    spawn_node!(localization_3d);
    spawn_node!(look_around);
    spawn_node!(look_at);
    spawn_node!(low_state_bridge);
    spawn_node!(mcap_recorder, log_path);
    spawn_node!(message_filter);
    spawn_node!(message_handler);
    spawn_node!(microphone_recorder);
    spawn_node!(motor_commands_collector);
    spawn_node!(obstacle_filter);
    spawn_node!(odometer_bridge);
    spawn_node!(odometry);
    spawn_node!(player_states_receiver);
    spawn_node!(primary_state_filter);
    spawn_node!(visual_kick_ball_selector);
    spawn_node!(rule_obstacle_composer);
    spawn_node!(safe_pose_checker);
    spawn_node!(search_suggestor);
    spawn_node!(segment_filter);
    spawn_node!(stereo_visual_odometry);
    spawn_node!(support_foot_estimator);
    spawn_node!(team_ball_receiver);
    spawn_node!(time_to_reach_kick_position);
    spawn_node!(trigger);
    spawn_node!(whistle_detection);
    spawn_node!(whistle_filter);
    spawn_node!(world_state_composer);
    spawn_node!(world_to_field_provider);

    Ok(RunningStack { join_set })
}

fn spawn_node_task<F>(join_set: &mut JoinSet<Result<()>>, node: &'static str, future: F)
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    let span = tracing::info_span!(target: "runtime::hulk_ros_z", "hulk_ros_z_node", node);
    join_set.spawn(future.instrument(span));
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
