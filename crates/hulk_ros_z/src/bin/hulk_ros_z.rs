use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use hulk_ros_z::{IntoEyreResultExt, nodes};
use ros_z::prelude::*;
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    robot: String,
    #[arg(long)]
    location: String,
    #[arg(long, default_value = "parameter/ros_z")]
    parameter_root: PathBuf,
    #[arg(long)]
    router: Option<String>,
}

struct RunningStack {
    join_set: JoinSet<Result<()>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let namespace = derive_namespace(&args.robot);
    let parameter_layers =
        derive_parameter_layers(&args.parameter_root, &args.location, &args.robot);

    let mut builder = ContextBuilder::default()
        .with_namespace(&namespace)
        .with_parameter_layers(parameter_layers);

    builder = match args.router {
        Some(router) => builder
            .with_mode("client")
            .with_router_endpoint(router)
            .into_eyre()?,
        None => builder
            .with_mode("router")
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/127.0.0.1:7447"]),
    };

    let ctx = Arc::new(builder.build().await.into_eyre()?);
    let mut running = spawn_all(ctx.clone()).await?;

    let result = tokio::select! {
        result = monitor(&mut running.join_set) => result,
        _ = tokio::signal::ctrl_c() => {
            Ok(())
        }
    };

    running.join_set.abort_all();
    ctx.shutdown().into_eyre()?;
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
    let components = robot
        .split('/')
        .filter(|component| !component.is_empty())
        .map(sanitize_namespace_component)
        .collect::<Vec<_>>();

    if components.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", components.join("/"))
    }
}

fn sanitize_namespace_component(component: &str) -> String {
    let mut sanitized = String::new();

    for character in component.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            sanitized.push(character);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        sanitized.insert(0, '_');
    }

    sanitized
}

async fn spawn_all(ctx: Arc<Context>) -> Result<RunningStack> {
    let mut join_set = JoinSet::new();

    join_set.spawn(nodes::active_vision::run(ctx.clone()));
    join_set.spawn(nodes::ball_filter::run(ctx.clone()));
    join_set.spawn(nodes::ball_state_composer::run(ctx.clone()));
    join_set.spawn(nodes::behavior_node::run(ctx.clone()));
    join_set.spawn(nodes::button_event_handler::run(ctx.clone()));
    join_set.spawn(nodes::camera_matrix_calculator::run(ctx.clone()));
    join_set.spawn(nodes::command_sender::run(ctx.clone()));
    join_set.spawn(nodes::fake_odometry::run(ctx.clone()));
    join_set.spawn(nodes::fall_down_state_receiver::run(ctx.clone()));
    join_set.spawn(nodes::field_border_detection::run(ctx.clone()));
    join_set.spawn(nodes::game_controller_filter::run(ctx.clone()));
    join_set.spawn(nodes::game_controller_state_filter::run(ctx.clone()));
    join_set.spawn(nodes::ground_provider::run(ctx.clone()));
    join_set.spawn(nodes::head_motion::run(ctx.clone()));
    join_set.spawn(nodes::image_receiver::run(ctx.clone()));
    join_set.spawn(nodes::image_segmenter::run(ctx.clone()));
    join_set.spawn(nodes::inference::run(ctx.clone()));
    join_set.spawn(nodes::kick::run(ctx.clone()));
    join_set.spawn(nodes::kinematics_provider::run(ctx.clone()));
    join_set.spawn(nodes::led_handler::run(ctx.clone()));
    join_set.spawn(nodes::line_detection::run(ctx.clone()));
    join_set.spawn(nodes::localization::run(ctx.clone()));
    join_set.spawn(nodes::look_around::run(ctx.clone()));
    join_set.spawn(nodes::look_at::run(ctx.clone()));
    join_set.spawn(nodes::message_filter::run(ctx.clone()));
    join_set.spawn(nodes::message_receiver::run(ctx.clone()));
    join_set.spawn(nodes::microphone_recorder::run(ctx.clone()));
    join_set.spawn(nodes::motor_commands_collector::run(ctx.clone()));
    join_set.spawn(nodes::obstacle_filter::run(ctx.clone()));
    join_set.spawn(nodes::obstacle_receiver::run(ctx.clone()));
    join_set.spawn(nodes::odometer_receiver::run(ctx.clone()));
    join_set.spawn(nodes::primary_state_filter::run(ctx.clone()));
    join_set.spawn(nodes::robot_mode_handler::run(ctx.clone()));
    join_set.spawn(nodes::rotate_head::run(ctx.clone()));
    join_set.spawn(nodes::rule_obstacle_composer::run(ctx.clone()));
    join_set.spawn(nodes::safe_pose_checker::run(ctx.clone()));
    join_set.spawn(nodes::search_suggestor::run(ctx.clone()));
    join_set.spawn(nodes::segment_filter::run(ctx.clone()));
    join_set.spawn(nodes::sensor_data_receiver::run(ctx.clone()));
    join_set.spawn(nodes::stand_up::run(ctx.clone()));
    join_set.spawn(nodes::team_ball_receiver::run(ctx.clone()));
    join_set.spawn(nodes::time_to_reach_kick_position::run(ctx.clone()));
    join_set.spawn(nodes::trigger::run(ctx.clone()));
    join_set.spawn(nodes::walking::run(ctx.clone()));
    join_set.spawn(nodes::whistle_detection::run(ctx.clone()));
    join_set.spawn(nodes::whistle_filter::run(ctx.clone()));
    join_set.spawn(nodes::world_state_composer::run(ctx.clone()));
    join_set.spawn(nodes::world_to_field_provider::run(ctx.clone()));

    Ok(RunningStack { join_set })
}

async fn monitor(join_set: &mut JoinSet<Result<()>>) -> Result<()> {
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(error),
            Err(join_error) => return Err(eyre!("monitor join failed: {join_error}")),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_namespace_replaces_invalid_characters() {
        assert_eq!(derive_namespace("robot-01"), "/robot_01");
    }
}
