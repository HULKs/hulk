use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster_sdk_interface::HighLevelCommand;
use linear_algebra::{Orientation2, Point2};
use std::{f32::consts::PI, sync::Arc, time::Duration};

use coordinate_systems::Ground;

use ros_z::prelude::*;

use types::{
    motion_command::{MotionCommand, OrientationMode},
    parameters::RLWalkingParameters,
    path::traits::{Length, PathProgress},
    step::Step,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: RLWalkingParameters,
    pub move_robot_message_interval: Duration,
    pub deceleration_distance: f32,
    pub hybrid_align_distance: f32,
    pub max_alignment_rate: f32,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("walking").build().await.into_eyre()?;

    let parameters = node.bind_parameter_as::<Parameters>("walking")?;

    let motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;
    let step_pub = node
        .publisher::<Step>("additional_outputs/walking_step")?
        .build()
        .await?;
    let high_level_command_pub = node
        .publisher::<HighLevelCommand>("commands/high_level_command")?
        .build()
        .await?;

    let mut last_published_move_robot_time = node.clock().now();

    loop {
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        let motion_command = motion_command_sub.recv().await?;

        let step = compute_step(parameters, &motion_command);
        step_pub.publish(&step).await?;

        if last_published_move_robot_time.duration_since(node.clock().now())
            > parameters.move_robot_message_interval
        {
            move_robot(&high_level_command_pub, step).await;
            last_published_move_robot_time = node.clock().now();
        }
    }
}

pub fn compute_step(parameters: &Parameters, motion_command: &MotionCommand) -> Step {
    match motion_command {
        MotionCommand::Walk {
            path,
            orientation_mode,
            target_orientation,
            distance_to_be_aligned,
            speed,
            ..
        } => {
            let forward = path.forward(Point2::origin());
            let distance_to_target = path.length();
            let deceleration_factor =
                (distance_to_target / parameters.deceleration_distance).clamp(0.0, 1.0);
            let velocity = forward * *speed * deceleration_factor;

            let (walk_orientation, _tolerance): (Orientation2<Ground>, f32) = match orientation_mode
            {
                OrientationMode::Unspecified => todo!(),
                OrientationMode::AlignWithPath => (Orientation2::from_vector(forward), 0.0),
                OrientationMode::LookTowards {
                    direction,
                    tolerance,
                } => (*direction, *tolerance),
                OrientationMode::LookAt { target, tolerance } => (
                    Orientation2::from_vector(target - Point2::origin()),
                    *tolerance,
                ),
            };

            let target_alignment_importance = target_alignment_importance(
                *distance_to_be_aligned,
                parameters.hybrid_align_distance,
                distance_to_target,
            );

            let orientation =
                walk_orientation.slerp(*target_orientation, target_alignment_importance);

            let angular_velocity = orientation.as_unit_vector().y() * parameters.max_alignment_rate;
            Step {
                forward: velocity.x(),
                left: velocity.y(),
                turn: angular_velocity,
            }
        }
        MotionCommand::WalkWithVelocity {
            velocity,
            angular_velocity,
            ..
        } => Step {
            forward: velocity.x(),
            left: velocity.y(),
            turn: *angular_velocity,
        },
        MotionCommand::Stand { .. } => Step::ZERO,
        _ => Step::ZERO,
    }
}

async fn move_robot(
    publisher: &Publisher<HighLevelCommand, SerdeCdrCodec<HighLevelCommand>>,
    step: Step,
) {
    let _ = publisher
        .publish(&HighLevelCommand::MoveRobot {
            forward: step.forward,
            left: step.left,
            turn: step.turn,
        })
        .await
        .inspect_err(|err| log::error!("{err:?}"));
}

// https://www.desmos.com/calculator/ng03egi9mp
fn target_alignment_importance(
    distance_to_be_aligned: f32,
    hybrid_align_distance: f32,
    distance_to_target: f32,
) -> f32 {
    if distance_to_target < distance_to_be_aligned {
        1.0
    } else if distance_to_target < distance_to_be_aligned + hybrid_align_distance {
        (1.0 + f32::cos(PI * (distance_to_target - distance_to_be_aligned) / hybrid_align_distance))
            * 0.5
    } else {
        0.0
    }
}
