use std::time::Duration;

use coordinate_systems::{Field, Ground};
use linear_algebra::{Orientation2, Point2, Vector2, point};
use types::{
    behavior_tree::Status,
    motion_command::{BodyMotion, HeadMotion, ImageRegion, MotionCommand},
    motion_type::MotionType,
};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    node::Blackboard,
    selection, sequence, subtree,
    switch_motion_type::{is_last_motion_type, switch_motion_type},
    walk::walk_to_ball,
};

pub fn kick_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Kick,
        sequence!(
            action!(kick),
            action!(select_kick_target),
            subtree!(kick_strength_subtree),
        ),
        subtree!(kick_alternatives_subtree),
    )
}

pub fn kick_alternatives_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Walk),
            action!(walk_to_ball)
        ),
        action!(stand)
    )
}

pub fn kick(blackboard: &mut Blackboard) -> Status {
    blackboard.kick_target = None;
    if blackboard.world_state.robot.ground_to_field.is_some()
        && let Some(ball_in_ground) = kick_ball_position_in_ground(blackboard)
    {
        blackboard.body_motion = Some(BodyMotion::Kick {
            ball_position: ball_in_ground,
            ball_velocity: blackboard
                .world_state
                .ball
                .map_or(Vector2::zeros(), |ball| ball.ball_in_ground_velocity),
            target_speed: blackboard.parameters.kicking.target_speed,
            soft: blackboard.parameters.kicking.soft,
            quick: blackboard.parameters.kicking.quick,
            kick_direction: Default::default(),
            strong: false,
        });
        if blackboard.last_motion_type == Some(MotionType::Kick) {
            use_last_kick_settings(blackboard);
        }
        blackboard.head_motion = Some(HeadMotion::LookAt {
            target: ball_in_ground,
            height_above_ground: blackboard.field_dimensions.ball_radius,
            image_region_target: ImageRegion::Center,
        });

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn select_kick_target(blackboard: &mut Blackboard) -> Status {
    let goal_position: Point2<Field> = point!(blackboard.field_dimensions.length / 2.0, 0.0);

    apply_kick_target(blackboard, goal_position)
}

pub fn apply_kick_target(
    blackboard: &mut Blackboard,
    target_position_in_field: Point2<Field>,
) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(BodyMotion::Kick {
            ball_position,
            kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        let target_position = ground_to_field.inverse() * target_position_in_field;
        *kick_direction = Orientation2::from_vector(target_position - *ball_position);
        blackboard.kick_target = Some(target_position);
        return Status::Success;
    }

    Status::Failure
}

pub fn kick_strength_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            action!(use_last_kick_settings)
        ),
        sequence!(
            condition!(is_target_in_strong_kick_range),
            condition!(allow_strong_kicks),
            action!(use_strong_kick)
        ),
        action!(disable_strong_kick)
    )
}

pub fn is_target_in_strong_kick_range(blackboard: &mut Blackboard) -> bool {
    if let Some(target_position) = blackboard.kick_target
        && let Some(BodyMotion::Kick { ball_position, .. }) = &blackboard.body_motion
    {
        (target_position - *ball_position).norm()
            >= blackboard
                .parameters
                .kicking
                .strong_kick_min_target_distance
    } else {
        false
    }
}

pub fn allow_strong_kicks(blackboard: &mut Blackboard) -> bool {
    blackboard.parameters.kicking.allow_strong_kicks
}

pub fn use_last_kick_settings(blackboard: &mut Blackboard) -> Status {
    if let MotionCommand::Kick {
        strong: last_strong,
        soft: last_soft,
        quick: last_quick,
        target_speed: last_target_speed,
        ..
    } = blackboard.last_motion_command
        && let Some(BodyMotion::Kick {
            strong: motion_strong,
            soft,
            quick,
            target_speed,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        *motion_strong = last_strong;
        *soft = last_soft;
        *quick = last_quick;
        *target_speed = last_target_speed;

        return Status::Success;
    }
    Status::Failure
}

pub fn use_kick(blackboard: &mut Blackboard, strong: bool) -> Status {
    if let Some(BodyMotion::Kick {
        strong: motion_strong,
        ..
    }) = blackboard.body_motion.as_mut()
    {
        *motion_strong = strong;

        return Status::Success;
    }
    Status::Failure
}

pub fn use_strong_kick(blackboard: &mut Blackboard) -> Status {
    use_kick(blackboard, true)
}

pub fn disable_strong_kick(blackboard: &mut Blackboard) -> Status {
    use_kick(blackboard, false)
}

pub fn intercept(blackboard: &mut Blackboard) -> Status {
    if let Some(BodyMotion::Kick {
        ball_position,
        ball_velocity,
        ..
    }) = &blackboard.body_motion
    {
        let ball_in_ground = *ball_position;
        let velocity = *ball_velocity;
        if velocity.norm() < f32::EPSILON {
            return Status::Failure;
        }
        let time_to_closest_approach =
            -ball_in_ground.coords().dot(&velocity) / velocity.norm_squared();
        if time_to_closest_approach < 0.0 {
            return Status::Failure;
        }

        let interception_point = ball_in_ground + velocity * time_to_closest_approach;
        if interception_point.x()
            < blackboard
                .parameters
                .kicking
                .minimum_interception_forward_distance
        {
            return Status::Failure;
        }

        if interception_point.coords().norm()
            > blackboard
                .parameters
                .ball
                .interception
                .maximum_intercept_distance
        {
            return Status::Failure;
        }

        let kick_direction = Orientation2::from_vector(ball_in_ground - interception_point);

        if let Some(BodyMotion::Kick {
            kick_direction: motion_kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
        {
            // The policy needs the observed position and velocity together.
            // Use the predicted interception point only to choose kick direction.
            blackboard.kick_target = Some(ball_in_ground);
            *motion_kick_direction = kick_direction;
            return Status::Success;
        }
    }
    Status::Failure
}
pub fn set_kick_target_beyond_ball(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field
        && let Some(ball_in_ground) = kick_ball_position_in_ground(blackboard)
        && let Some(BodyMotion::Kick {
            kick_direction: motion_kick_direction,
            ..
        }) = blackboard.body_motion.as_mut()
    {
        if blackboard.last_motion_type != Some(MotionType::Kick) {
            let Some(direction) = ball_in_ground.coords().try_normalize(f32::EPSILON) else {
                return Status::Failure;
            };
            let kick_target = ground_to_field * (ball_in_ground + direction * 10.0);
            blackboard.last_kick_target = Some(kick_target);
        }

        if let Some(target_in_field) = blackboard.last_kick_target {
            let field_to_ground = ground_to_field.inverse();
            let target_position = field_to_ground * target_in_field;
            let kick_direction = Orientation2::from_vector(target_position - ball_in_ground);

            blackboard.kick_target = Some(target_position);
            *motion_kick_direction = kick_direction;

            return Status::Success;
        }
    }
    Status::Failure
}

/// This is an exposure-age limit, not an age of the selector's publication.
/// Recheck on every behavior tick so a stopped selector cannot authorize kicks.
const MAXIMUM_VISUAL_KICK_AGE: Duration = Duration::from_millis(100);

fn kick_ball_position_in_ground(blackboard: &Blackboard) -> Option<Point2<Ground>> {
    blackboard
        .visual_kick_ball_position
        .as_ref()
        .filter(|ball| {
            ball.age_at(blackboard.world_state.now)
                .is_some_and(|age| age <= MAXIMUM_VISUAL_KICK_AGE)
        })
        .filter(|ball| {
            ball.position
                .coords()
                .inner
                .iter()
                .all(|value| value.is_finite())
        })
        .map(|ball| ball.position)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::LastBall;
    use linear_algebra::Isometry2;
    use ros_z::time::Time;
    use types::{ball_position::BallPosition, field_dimensions::Side, world_state::WorldState};

    fn blackboard() -> Blackboard {
        let mut world_state = WorldState::default();
        world_state.now = Time::from_nanos(1_000_000_000);
        world_state.robot.ground_to_field = Some(Isometry2::identity());
        Blackboard {
            field_dimensions: Default::default(),
            parameters: Default::default(),
            world_state,
            controller_input: None,
            remote_control_enabled: false,
            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),
            ball: Some(LastBall {
                position: point![0.3, 0.0],
                velocity: Vector2::zeros(),
                age: Time::from_nanos(1_000_000_000),
                field_side: Side::Left,
            }),
            visual_kick_ball_position: None,
            last_ball: None,
            last_close_enough_to_kick: false,
            kick_target: None,
            last_kick_target: None,
            last_motion_command: MotionCommand::default(),
            last_motion_switch_time: Time::zero(),
            last_motion_type: None,
            last_sent_game_controller_return_message_time: None,
            last_sent_hsl_message_time: None,
            last_closest_to_ball: false,
            closest_to_ball_entered_area_since: None,
            closest_to_ball_left_area_since: None,
            is_injected_motion_command: false,
            walk_position: None,
            body_motion: None,
            head_motion: None,
            voronoi_map: None,
        }
    }

    #[test]
    fn model_only_ball_cannot_start_or_reauthorize_kick() {
        let mut board = blackboard();
        assert_eq!(kick(&mut board), Status::Failure);
        assert!(board.body_motion.is_none());
        board.last_motion_type = Some(MotionType::Kick);
        assert_eq!(kick(&mut board), Status::Failure);
        assert!(board.body_motion.is_none());
    }

    #[test]
    fn visual_authorization_uses_exposure_age_and_expires_during_selector_silence() {
        let mut board = blackboard();
        let exposure = board.world_state.now;
        board.visual_kick_ball_position = Some(BallPosition {
            position: point![0.4, 0.1],
            velocity: Vector2::zeros(),
            last_seen: exposure,
        });
        board.world_state.now = exposure + Duration::from_millis(100);
        assert_eq!(kick(&mut board), Status::Success);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::Kick {ball_position, ..}) if ball_position == point![0.4, 0.1])
        );
        board.body_motion = None;
        board.world_state.now = exposure + Duration::from_millis(101);
        assert_eq!(kick(&mut board), Status::Failure);
        assert!(board.body_motion.is_none());
        board.world_state.now = exposure - Duration::from_millis(1);
        assert_eq!(kick(&mut board), Status::Failure);
        board.world_state.now = exposure;
        board.visual_kick_ball_position.as_mut().unwrap().position = point![f32::INFINITY, 0.0];
        assert_eq!(kick(&mut board), Status::Failure);
    }
}
