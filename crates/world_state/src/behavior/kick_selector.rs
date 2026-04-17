use coordinate_systems::{Field, Ground};
use linear_algebra::{Orientation2, Point2, point};
use serde::Serialize;
use types::{behavior_tree::Status, motion_command::{KickPower, MotionCommand}};

use crate::behavior::node::Blackboard;

#[derive(Debug, Clone, Serialize)]
pub struct KickTarget {
    pub position: Point2<Ground>,
    pub direction: Orientation2<Ground>,
}

pub fn select_kick_target(context: &mut Blackboard) -> Status {
    if let (Some(ground_to_field), Some(ball)) =
        (context.world_state.robot.ground_to_field, &context.ball)
    {
        let goal_position: Point2<Field> = point!(context.field_dimensions.length / 2.0, 0.0);
        let field_to_ground = ground_to_field.inverse();

        let target_position = field_to_ground * goal_position;

        let ball_in_ground = field_to_ground * ball.position;
        let kick_direction = Orientation2::from_vector(target_position - ball_in_ground);

        context.kick_target = Some(KickTarget {
            position: target_position,
            direction: kick_direction,
        });

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn is_close_to_target(context: &mut Blackboard) -> bool {
    if let Some(kick_target) = &context.kick_target {
        kick_target.position.coords().norm()
            < context
                .parameters
                .kicking
                .target_distance_kick_power_threshold
    } else {
        false
    }
}

pub fn allow_schlong(context: &mut Blackboard) -> bool {
   context.parameters.kicking.allow_schlong
}

pub fn use_last_kick_power(context: &mut Blackboard) -> Status {
    if let MotionCommand::VisualKick { kick_power, .. } = context.last_motion_command {
        context.last_kick_power = Some(kick_power);
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn use_kick_power(context: &mut Blackboard, kick_power: KickPower) -> Status {
    context.last_kick_power = Some(kick_power);
    Status::Success
}