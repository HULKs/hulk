use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use ros_z::{IntoEyreResultExt, prelude::*};
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
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

    join_set.spawn(active_vision::run(ctx.clone()));
    join_set.spawn(ball_filter::run(ctx.clone()));
    join_set.spawn(ball_state_composer::run(ctx.clone()));
    join_set.spawn(behavior_node::run(ctx.clone()));
    join_set.spawn(button_event_handler::run(ctx.clone()));
    join_set.spawn(button_event_bridge::run(ctx.clone()));
    join_set.spawn(camera_matrix_calculator::run(ctx.clone()));
    join_set.spawn(low_command_publisher::run(ctx.clone()));
    join_set.spawn(fake_odometry::run(ctx.clone()));
    join_set.spawn(fall_down_state_receiver::run(ctx.clone()));
    join_set.spawn(field_border_detection::run(ctx.clone()));
    join_set.spawn(game_controller_filter::run(ctx.clone()));
    join_set.spawn(game_controller_state_filter::run(ctx.clone()));
    join_set.spawn(ground_provider::run(ctx.clone()));
    join_set.spawn(head_motion::run(ctx.clone()));
    join_set.spawn(image_receiver::run(ctx.clone()));
    join_set.spawn(image_segmenter::run(ctx.clone()));
    join_set.spawn(inference::run(ctx.clone()));
    join_set.spawn(kick::run(ctx.clone()));
    join_set.spawn(kinematics_provider::run(ctx.clone()));
    join_set.spawn(led_handler::run(ctx.clone()));
    join_set.spawn(line_detection::run(ctx.clone()));
    join_set.spawn(localization::run(ctx.clone()));
    join_set.spawn(look_around::run(ctx.clone()));
    join_set.spawn(look_at::run(ctx.clone()));
    join_set.spawn(message_filter::run(ctx.clone()));
    join_set.spawn(message_receiver::run(ctx.clone()));
    join_set.spawn(microphone_recorder::run(ctx.clone()));
    join_set.spawn(motor_commands_collector::run(ctx.clone()));
    join_set.spawn(obstacle_filter::run(ctx.clone()));
    join_set.spawn(obstacle_receiver::run(ctx.clone()));
    join_set.spawn(odometer_bridge::run(ctx.clone()));
    join_set.spawn(primary_state_filter::run(ctx.clone()));
    join_set.spawn(robot_mode_handler::run(ctx.clone()));
    join_set.spawn(rotate_head::run(ctx.clone()));
    join_set.spawn(rule_obstacle_composer::run(ctx.clone()));
    join_set.spawn(safe_pose_checker::run(ctx.clone()));
    join_set.spawn(search_suggestor::run(ctx.clone()));
    join_set.spawn(segment_filter::run(ctx.clone()));
    join_set.spawn(booster_sdk_interface::run(ctx.clone()));
    join_set.spawn(low_state_bridge::run(ctx.clone()));
    join_set.spawn(stand_up::run(ctx.clone()));
    join_set.spawn(team_ball_receiver::run(ctx.clone()));
    join_set.spawn(time_to_reach_kick_position::run(ctx.clone()));
    join_set.spawn(trigger::run(ctx.clone()));
    join_set.spawn(walking::run(ctx.clone()));
    join_set.spawn(whistle_detection::run(ctx.clone()));
    join_set.spawn(whistle_filter::run(ctx.clone()));
    join_set.spawn(world_state_composer::run(ctx.clone()));
    join_set.spawn(world_to_field_provider::run(ctx.clone()));

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
