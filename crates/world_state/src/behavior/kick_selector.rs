use coordinate_systems::{Field, Ground};
use linear_algebra::{Orientation2, Point2, point};
use serde::Serialize;
use types::{
    behavior_tree::Status,
    motion_command::{KickPower, MotionCommand},
    motion_type::MotionType,
};

use crate::{
    action,
    behavior::{behavior_tree::Node, node::Blackboard, switch_motion_type::is_last_motion_type},
    condition, negation, selection, sequence,
};

#[derive(Debug, Clone, Serialize)]
pub struct KickTarget {
    pub position: Point2<Ground>,
    pub direction: Orientation2<Ground>,
}

pub fn kick_power_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            action!(use_last_kick_power)
        ),
        sequence!(
            negation!(condition!(is_close_to_target)),
            condition!(allow_schlong),
            action!(use_kick_power, KickPower::Schlong)
        ),
        action!(use_kick_power, KickPower::Rumpelstilzchen)
    )
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
