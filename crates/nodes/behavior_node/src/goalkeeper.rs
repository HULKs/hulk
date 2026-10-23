use coordinate_systems::{Field, Ground};
use geometry::line_segment::LineSegment;
use hsl_network_messages::SubState;
use linear_algebra::{Orientation2, Point2, Pose2, Vector2, point, vector};
use path_planner::path_planner::PathPlanner;
use types::{
    behavior_tree::Status,
    field_dimensions::FieldDimensions,
    motion_command::{BodyMotion, OrientationMode},
    motion_type::MotionType,
    obstacles::ObstacleKind,
    path::PathSegment,
};

use crate::{
    action,
    behavior_tree::Node,
    condition,
    conditions::{
        hulks_is_kicking_team, is_closest_to_ball, opponent_near_ball, opponent_shot_direction,
    },
    head::look_at_ball_subtree,
    kick::{
        apply_kick_target, disable_strong_kick,
        execution::{execute_ball_action, goalward_target},
        geometry::travel_openings,
        intercept, kick, kick_alternatives_subtree,
        strategy::{BallAction, select_ball_action},
    },
    negation,
    node::Blackboard,
    selection, sequence,
    substates::{is_in_sub_state, is_sub_state},
    subtree,
    switch_motion_type::{is_allowed_to_switch, switch_motion_type},
    tree::striker_subtree,
    voronoi::calculate_voronoi_grid,
    walk::{
        adaptive_blocking_velocity, walk_alternatives_subtree, walk_to, walk_to_block_position,
    },
};

pub fn goalkeeper_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(
                condition!(is_ball_behind_own_goal_line),
                subtree!(goalkeeper_center_position_subtree),
            ),
            sequence!(
                condition!(is_in_sub_state),
                selection!(
                    negation!(condition!(hulks_is_kicking_team)),
                    condition!(is_sub_state, SubState::GoalKick),
                ),
                subtree!(goalkeeper_sub_state_subtree)
            ),
            sequence!(
                condition!(is_goalkeeper_kick_away_needed),
                selection!(
                    sequence!(
                        condition!(needs_walk_before_goalkeeper_clearance),
                        switch_motion_type(
                            MotionType::Walk,
                            action!(walk_to_goalkeeper_clearance_ball),
                            Node::Failure,
                        )
                    ),
                    sequence!(
                        negation!(condition!(needs_walk_before_goalkeeper_clearance)),
                        selection!(
                            action!(try_goalkeeper_pass),
                            switch_motion_type(
                                MotionType::Kick,
                                sequence!(
                                    action!(kick),
                                    action!(select_goalkeeper_kick_away_target),
                                    action!(enable_quick_clearance),
                                ),
                                subtree!(kick_alternatives_subtree),
                            )
                        )
                    ),
                )
            ),
            sequence!(
                condition!(is_in_sub_state),
                subtree!(goalkeeper_sub_state_subtree)
            ),
            sequence!(
                condition!(is_goalkeeper_interception_candidate),
                switch_motion_type(
                    MotionType::Kick,
                    sequence!(
                        action!(kick),
                        action!(intercept),
                        action!(disable_strong_kick),
                    ),
                    Node::Failure,
                )
            ),
            subtree!(goalkeeper_shot_block_position_subtree),
            subtree!(goalkeeper_pre_shot_block_position_subtree),
            sequence!(
                condition!(is_own_corner_cutback),
                subtree!(goalkeeper_center_position_subtree),
            ),
            sequence!(
                condition!(is_ball_close_enough_to_goal_to_become_striker),
                selection!(sequence!(
                    action!(calculate_voronoi_grid),
                    condition!(is_closest_to_ball),
                    subtree!(striker_subtree),
                ),),
            ),
            sequence!(
                condition!(is_ball_near_own_goal),
                subtree!(goalkeeper_active_defense_position_subtree),
            ),
            subtree!(goalkeeper_default_position_subtree),
        ),
    )
}

fn needs_walk_before_goalkeeper_clearance(blackboard: &mut Blackboard) -> bool {
    let (Some(ball), Some(transform)) = (
        blackboard.ball.as_ref(),
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return true;
    };
    let Some(target) = goalkeeper_clearance_target_for_pose(blackboard, transform.as_pose()) else {
        return true;
    };
    let ball_in_ground = transform.inverse() * ball.position;
    ball_in_ground.x() <= 0.0
        || ball_in_ground.coords().norm()
            >= blackboard
                .parameters
                .goalkeeper
                .kick_away_ball_maximum_robot_distance
        || (transform.inverse() * (target - ball.position)).x() <= 0.0
        || is_goalkeeper_kick_approach_obstructed(blackboard, target)
}

fn is_goalkeeper_kick_approach_obstructed(
    blackboard: &mut Blackboard,
    target: Point2<Field>,
) -> bool {
    if goalkeeper_shot_block_position(blackboard).is_some()
        || (is_in_sub_state(blackboard) && !hulks_is_kicking_team(blackboard))
    {
        return false;
    }
    let (Some(ball), Some(transform)) = (
        blackboard.ball.as_ref(),
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return false;
    };
    let ball = transform.inverse() * ball.position;
    if ball.coords().norm()
        > blackboard.parameters.kicking.kick_activation_distance.max(
            blackboard
                .parameters
                .goalkeeper
                .kick_away_ball_maximum_robot_distance,
        )
    {
        return false;
    }
    // The kick policy cannot avoid posts or the net during its final approach.
    let dimensions = &blackboard.field_dimensions;
    let mut planner = PathPlanner::default();
    for mut post in blackboard
        .world_state
        .obstacles
        .iter()
        .copied()
        .filter(|obstacle| obstacle.kind == ObstacleKind::GoalPost)
    {
        post.radius_at_hip_height = dimensions.goal_post_diameter / 2.0;
        planner.with_obstacles(
            &[post],
            blackboard.parameters.walking.path_planning.robot_radius * 0.75,
        );
    }
    planner.with_goal_support_structures(transform.inverse(), dimensions);
    let direction = (transform.inverse() * target - ball).normalize();
    let standoff = blackboard.parameters.kicking.approach_ball_standoff;
    let stance = ball - direction * standoff;
    planner.obstacles.iter().any(|obstacle| {
        obstacle
            .shape
            .intersects_line_segment(LineSegment(Point2::origin(), ball))
            || obstacle
                .shape
                .intersects_line_segment(LineSegment(Point2::origin(), stance))
    })
}

fn walk_to_goalkeeper_clearance_ball(blackboard: &mut Blackboard) -> Status {
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(target) = goalkeeper_clearance_target_for_pose(blackboard, transform.as_pose()) else {
        return Status::Failure;
    };
    walk_to_goalkeeper_ball(blackboard, target)
}

fn walk_to_goalkeeper_ball(blackboard: &mut Blackboard, target: Point2<Field>) -> Status {
    let (Some(ball), Some(transform)) = (
        blackboard.ball.as_ref(),
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return Status::Failure;
    };
    let projected = blackboard.projected_ball_position(ball.position, ball.velocity);
    let direction = (target - projected).normalize();
    let stance = projected - direction * blackboard.parameters.kicking.approach_ball_standoff;
    let walking = blackboard.parameters.walking.walk_and_stand;
    blackboard.walk_position = Some(transform.inverse() * stance);
    walk_to(
        blackboard,
        transform.inverse() * Pose2::from_parts(stance, Orientation2::from_vector(direction)),
        blackboard.parameters.walking.speed.kicking,
        OrientationMode::AlignWithPath,
        walking.normal_distance_to_be_aligned,
        walking.hysteresis,
    )
}

fn execute_goalkeeper_goal_kick(blackboard: &mut Blackboard) -> Status {
    let status = execute_ball_action(blackboard);
    if !matches!(blackboard.body_motion, Some(BodyMotion::Kick { .. })) {
        return status;
    }
    let (Some(transform), Some(target)) = (
        blackboard.world_state.robot.ground_to_field,
        blackboard.kick_target,
    ) else {
        return status;
    };
    let target = transform * target;
    if !is_goalkeeper_kick_approach_obstructed(blackboard, target) {
        return status;
    }
    if blackboard.last_motion_type != Some(MotionType::Walk) && !is_allowed_to_switch(blackboard) {
        return crate::actions::stand(blackboard);
    }
    walk_to_goalkeeper_ball(blackboard, target)
}

fn goalkeeper_sub_state_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_sub_state, SubState::PenaltyKick),
            negation!(condition!(hulks_is_kicking_team)),
            switch_motion_type(
                MotionType::Walk,
                action!(walk_to_goalkeeper_penalty_position),
                subtree!(walk_alternatives_subtree),
            )
        ),
        sequence!(
            negation!(condition!(hulks_is_kicking_team)),
            condition!(is_sub_state, SubState::CornerKick),
            selection!(
                subtree!(goalkeeper_shot_block_position_subtree),
                subtree!(goalkeeper_pre_shot_block_position_subtree),
                subtree!(goalkeeper_center_position_subtree)
            )
        ),
        sequence!(
            negation!(condition!(hulks_is_kicking_team)),
            selection!(
                condition!(is_sub_state, SubState::ThrowIn),
                condition!(is_sub_state, SubState::IndirectFreeKick),
                condition!(is_sub_state, SubState::DirectFreeKick),
            ),
            selection!(
                subtree!(goalkeeper_shot_block_position_subtree),
                subtree!(goalkeeper_pre_shot_block_position_subtree),
                sequence!(
                    condition!(is_ball_near_own_goal),
                    subtree!(goalkeeper_active_defense_position_subtree)
                ),
            )
        ),
        sequence!(
            condition!(hulks_is_kicking_team),
            condition!(is_sub_state, SubState::GoalKick),
            action!(calculate_voronoi_grid),
            condition!(is_closest_to_ball),
            action!(select_ball_action),
            action!(execute_goalkeeper_goal_kick),
        ),
        subtree!(goalkeeper_default_position_subtree),
    )
}

pub(crate) fn goalkeeper_shot_block_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        sequence!(
            action!(set_goalkeeper_shot_block_position),
            action!(walk_to_goalkeeper_shot_block_position)
        ),
        Node::Failure,
    )
}

fn goalkeeper_pre_shot_block_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_goalkeeper_pre_shot_block_position),
        Node::Failure,
    )
}

fn goalkeeper_center_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        sequence!(
            action!(set_goalkeeper_center_position),
            action!(walk_to_block_position)
        ),
        subtree!(walk_alternatives_subtree),
    )
}

fn is_own_corner_cutback(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let goal_x = -blackboard.field_dimensions.length / 2.0;
        let penalty_area_x = goal_x + blackboard.field_dimensions.penalty_area_length;
        (goal_x..penalty_area_x).contains(&ball.position.x())
            && ball.velocity.norm()
                > blackboard
                    .parameters
                    .ball
                    .interception
                    .minimum_ball_velocity
            && ball.velocity.x() > 0.0
            && ball.position.y() * ball.velocity.y() < 0.0
            && ball.velocity.y().abs() > ball.velocity.x().abs()
    })
}

fn set_goalkeeper_center_position(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let position = point!(
            -blackboard.field_dimensions.length / 2.0 + blackboard.parameters.goalkeeper.x_offset,
            0.0
        );
        blackboard.walk_position = Some(ground_to_field.inverse() * position);
        Status::Success
    } else {
        Status::Failure
    }
}

fn goalkeeper_active_defense_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        sequence!(
            action!(set_goalkeeper_active_defense_position),
            action!(walk_to_block_position)
        ),
        Node::Failure,
    )
}

fn goalkeeper_default_position_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_goalkeeper_default_position),
        subtree!(walk_alternatives_subtree),
    )
}

fn is_ball_behind_own_goal_line(blackboard: &mut Blackboard) -> bool {
    blackboard
        .ball
        .as_ref()
        .is_some_and(|ball| ball.position.x() < -blackboard.field_dimensions.length / 2.0)
}

fn is_goalkeeper_interception_candidate(blackboard: &mut Blackboard) -> bool {
    if goalkeeper_shot_block_position(blackboard).is_none() {
        return false;
    }

    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        if ball_in_ground.coords().norm()
            > blackboard
                .parameters
                .ball
                .interception
                .maximum_ball_distance
                .min(blackboard.parameters.kicking.kick_activation_distance)
        {
            return false;
        }
        let velocity = ground_to_field.inverse() * ball.velocity;
        let time_to_closest_approach =
            -ball_in_ground.coords().dot(&velocity) / velocity.norm_squared();
        if time_to_closest_approach < 0.0 {
            return false;
        }

        let interception_point = ball_in_ground + velocity * time_to_closest_approach;
        interception_point.x()
            >= blackboard
                .parameters
                .kicking
                .minimum_interception_forward_distance
            && interception_point.coords().norm()
                <= blackboard
                    .parameters
                    .ball
                    .interception
                    .maximum_intercept_distance
    } else {
        false
    }
}

fn shot_y_at_defense_line(
    ball: Point2<Field>,
    velocity: Vector2<Field>,
    goal_x: f32,
    defense_x: f32,
    goal_half_width: f32,
) -> Option<f32> {
    if ![ball.x(), ball.y(), velocity.x(), velocity.y()]
        .into_iter()
        .all(f32::is_finite)
        || velocity.x() >= 0.0
        || ball.x() <= defense_x
    {
        return None;
    }
    let goal_y = ball.y() + velocity.y() * (goal_x - ball.x()) / velocity.x();
    if goal_y.abs() >= goal_half_width {
        return None;
    }
    Some(ball.y() + velocity.y() * (defense_x - ball.x()) / velocity.x())
}

fn goalkeeper_shot_block_position(blackboard: &Blackboard) -> Option<Point2<Field>> {
    let ball = blackboard.ball.as_ref()?;
    if !is_interception_velocity_towards_own_half(blackboard, ball.velocity) {
        return None;
    }
    goalkeeper_block_position_for_direction(blackboard, ball.velocity)
}

fn goalkeeper_block_position_for_direction(
    blackboard: &Blackboard,
    direction: Vector2<Field>,
) -> Option<Point2<Field>> {
    let ball = blackboard.ball.as_ref()?;
    let dimensions = blackboard.field_dimensions;
    let parameters = &blackboard.parameters.goalkeeper;
    let goal_x = -dimensions.length / 2.0;
    let defense_x = goal_x + parameters.x_offset;
    let goal_half_width = dimensions.goal_inner_width / 2.0;
    let target_y = shot_y_at_defense_line(
        ball.position,
        direction,
        goal_x,
        defense_x,
        goal_half_width + dimensions.goal_post_diameter / 2.0 + dimensions.ball_radius,
    )?;
    let maximum_y = (goal_half_width - parameters.distance_to_goalpost).max(0.0);
    Some(point!(defense_x, target_y.clamp(-maximum_y, maximum_y)))
}

fn walk_to_goalkeeper_pre_shot_block_position(blackboard: &mut Blackboard) -> Status {
    let (Some(direction), Some(transform)) = (
        opponent_shot_direction(blackboard),
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return Status::Failure;
    };
    let Some(position) = goalkeeper_block_position_for_direction(blackboard, direction) else {
        return Status::Failure;
    };
    blackboard.walk_position = Some(transform.inverse() * position);
    walk_to_block_position(blackboard)
}

fn set_goalkeeper_shot_block_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(position), Some(ground_to_field)) = (
        goalkeeper_shot_block_position(blackboard),
        blackboard.world_state.robot.ground_to_field,
    ) {
        blackboard.walk_position = Some(ground_to_field.inverse() * position);
        Status::Success
    } else {
        Status::Failure
    }
}

fn walk_to_goalkeeper_shot_block_position(blackboard: &mut Blackboard) -> Status {
    let status = walk_to_block_position(blackboard);
    let limits = blackboard.walking_velocity_limits;
    let (
        Some(BodyMotion::Walk {
            path,
            target_orientation,
            ..
        }),
        Some(target),
        Some(ball),
        Some(transform),
    ) = (
        &blackboard.body_motion,
        blackboard.walk_position,
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    )
    else {
        return status;
    };
    // Follow planned detours normally; only accelerate along an unobstructed segment.
    let [PathSegment::LineSegment(line)] = path.segments.as_slice() else {
        return status;
    };
    if limits.validate().is_err()
        || line.0.coords().norm() > 1e-4
        || (line.1 - target).norm() > 1e-4
        || blackboard
            .path_obstacles_output
            .iter()
            .any(|obstacle| obstacle.shape.intersects_line_segment(*line))
    {
        return status;
    }
    let time_to_line = ((transform * target).x() - ball.position.x()) / ball.velocity.x();
    if !time_to_line.is_finite() || time_to_line <= 0.0 {
        return status;
    }
    let velocity = adaptive_blocking_velocity(
        blackboard,
        target.coords() / time_to_line,
        transform.inverse() * ball.position,
    );
    let angular_velocity = (target_orientation.angle() / time_to_line).clamp(
        -limits.angular_velocity_limit,
        limits.angular_velocity_limit,
    );
    blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
        velocity,
        angular_velocity,
    });
    status
}

fn is_goalkeeper_kick_away_needed(blackboard: &mut Blackboard) -> bool {
    if !is_ball_in_own_penalty_area(blackboard)
        || goalkeeper_shot_block_position(blackboard).is_some()
    {
        return false;
    }

    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_in_ground = ground_to_field.inverse() * ball.position;
        let distance = ball_in_ground.coords().norm();
        let parameters = &blackboard.parameters;
        let kick_distance = parameters.goalkeeper.kick_away_ball_maximum_robot_distance;
        if distance < kick_distance {
            return true;
        }
        let field = &blackboard.field_dimensions;
        let post = point![
            -field.length / 2.0,
            (field.goal_inner_width / 2.0).copysign(ball.position.y())
        ];
        // Retrieve loose balls beside a post without taking a closer teammate's possession.
        distance < parameters.goalkeeper.striker_distance
            && ball.position.x() < -field.length / 2.0 + field.goal_box_area_length
            && (ball.position - post).norm() < kick_distance
            && ball.velocity.norm()
                <= parameters
                    .ball
                    .interception
                    .minimum_ball_velocity_towards_robot
            && !blackboard
                .world_state
                .player_states
                .iter()
                .any(|(number, teammate)| {
                    number != blackboard.player_number
                        && teammate.as_ref().is_some_and(|teammate| {
                            teammate.time <= blackboard.world_state.now
                                && blackboard.world_state.now.duration_since(teammate.time)
                                    < parameters.kicking.strategy.passing.maximum_receiver_age
                                && (teammate.inner.pose.position() - ball.position).norm()
                                    < distance
                        })
                })
    } else {
        false
    }
}

fn is_ball_in_own_penalty_area(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let field_dimensions = blackboard.field_dimensions;
        let own_penalty_area_x =
            -field_dimensions.length / 2.0 + field_dimensions.penalty_area_length;

        ball.position.x() < own_penalty_area_x
            && ball.position.y().abs() < field_dimensions.penalty_area_width / 2.0
    })
}

fn is_ball_close_enough_to_goal_to_become_striker(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = -blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x + blackboard.parameters.goalkeeper.striker_distance;

        ball.position.x() < maximum_ball_x
            || (crate::replacement_goalkeeper::has_goal_cover(blackboard)
                && ball.age <= blackboard.world_state.now
                && blackboard.world_state.now.duration_since(ball.age)
                    < crate::node::ball_timeout(&blackboard.parameters, ball.source)
                && blackboard
                    .world_state
                    .robot
                    .ground_to_field
                    .is_some_and(|transform| {
                        (ball.position - transform.as_pose().position()).norm()
                            < blackboard.parameters.goalkeeper.striker_distance
                    }))
    })
}

fn is_ball_near_own_goal(blackboard: &mut Blackboard) -> bool {
    blackboard.ball.as_ref().is_some_and(|ball| {
        let own_goal_x = -blackboard.field_dimensions.length / 2.0;
        let maximum_ball_x = own_goal_x + blackboard.parameters.goalkeeper.passive_distance;

        ball.position.x() < maximum_ball_x
    })
}
pub fn set_goalkeeper_active_defense_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let field_dimensions = blackboard.field_dimensions;
        let parameters = &blackboard.parameters.goalkeeper;

        let own_goal_line_x = -field_dimensions.length / 2.0;
        let own_goal_center = point!(own_goal_line_x, 0.0);
        let goal_to_ball = ball.position - own_goal_center;
        let defense_radius =
            (field_dimensions.goal_inner_width / 2.0 - parameters.distance_to_goalpost).max(0.0);
        let unclamped_defense_position_in_field = if goal_to_ball.norm() < f32::EPSILON {
            point!(own_goal_line_x + defense_radius, 0.0)
        } else {
            own_goal_center + goal_to_ball.normalize() * defense_radius
        };
        let minimum_defense_x = own_goal_line_x + parameters.distance_to_goalpost;
        let defense_position_in_field = point!(
            unclamped_defense_position_in_field
                .x()
                .max(minimum_defense_x),
            unclamped_defense_position_in_field.y()
        );
        let defense_position_in_ground = ground_to_field.inverse() * defense_position_in_field;

        if defense_position_in_ground.coords().norm()
            > parameters.active_defense_maximum_robot_distance
        {
            return Status::Failure;
        }

        blackboard.walk_position = Some(defense_position_in_ground);

        Status::Success
    } else {
        Status::Failure
    }
}

pub fn walk_to_goalkeeper_default_position(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let field_dimensions = blackboard.field_dimensions;
        let own_goal_line_x = -field_dimensions.length / 2.0;
        let parameters = &blackboard.parameters.goalkeeper;
        let maximum_y =
            (field_dimensions.goal_inner_width / 2.0 - parameters.distance_to_goalpost).max(0.0);
        let target_y = blackboard.ball.as_ref().map_or(0.0, |ball| {
            let distance_from_goal = ball.position.x() - own_goal_line_x;
            if distance_from_goal > parameters.x_offset {
                (ball.position.y() * parameters.x_offset / distance_from_goal)
                    .clamp(-maximum_y, maximum_y)
            } else {
                0.0
            }
        });
        let default_position_in_field = point!(own_goal_line_x + parameters.x_offset, target_y);
        let default_pose_in_field =
            Pose2::from_parts(default_position_in_field, Orientation2::new(0.0));
        let default_pose_in_ground = ground_to_field.inverse() * default_pose_in_field;
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let blocking_speed = blackboard.parameters.walking.speed.blocking;
        let orientation_mode = if let Some(ball) = &blackboard.world_state.ball {
            OrientationMode::LookAt {
                target: ball.ball_in_ground,
                tolerance: walk_and_stand.orientation_tolerance,
            }
        } else if blackboard
            .ball
            .as_ref()
            .is_some_and(|ball| ball.position.x() <= 0.0)
        {
            OrientationMode::LookAt {
                target: ground_to_field.inverse() * point!(own_goal_line_x, 0.0),
                tolerance: walk_and_stand.orientation_tolerance,
            }
        } else {
            OrientationMode::AlignWithPath
        };

        walk_to(
            blackboard,
            default_pose_in_ground,
            blocking_speed,
            orientation_mode,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.goalkeeper_hysteresis,
        )
    } else {
        Status::Failure
    }
}
pub fn walk_to_goalkeeper_penalty_position(blackboard: &mut Blackboard) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let own_goal_line_x = -blackboard.field_dimensions.length / 2.0;
        let penalty_position_in_field = point!(own_goal_line_x, 0.0);
        let penalty_pose_in_field =
            Pose2::from_parts(penalty_position_in_field, Orientation2::new(0.0));
        let penalty_pose_in_ground = ground_to_field.inverse() * penalty_pose_in_field;
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let blocking_speed = blackboard.parameters.walking.speed.blocking;

        walk_to(
            blackboard,
            penalty_pose_in_ground,
            blocking_speed,
            OrientationMode::AlignWithPath,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.goalkeeper_hysteresis,
        )
    } else {
        Status::Failure
    }
}

fn select_goalkeeper_kick_away_target(blackboard: &mut Blackboard) -> Status {
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(target) = goalkeeper_clearance_target_for_pose(blackboard, transform.as_pose()) else {
        return Status::Failure;
    };
    apply_kick_target(blackboard, target)
}

fn is_goalkeeper_under_pressure(blackboard: &Blackboard) -> bool {
    let field = &blackboard.field_dimensions;
    let distance = 0.85
        * field
            .penalty_area_length
            .min(field.penalty_area_width / 2.0);
    opponent_near_ball(blackboard, distance)
}

fn try_goalkeeper_pass(blackboard: &mut Blackboard) -> Status {
    // Secure goal-line balls first; tolerate coordinate roundoff at the goal-box edge.
    let beside_goal_line = blackboard.ball.as_ref().is_some_and(|ball| {
        ball.position.x()
            < -blackboard.field_dimensions.length / 2.0
                + blackboard.field_dimensions.goal_box_area_length
                - 1e-4
    });
    if beside_goal_line
        || is_goalkeeper_under_pressure(blackboard)
        || select_ball_action(blackboard) != Status::Success
        || !matches!(blackboard.ball_action, Some(BallAction::Pass { .. }))
        || execute_ball_action(blackboard) != Status::Success
        || !matches!(blackboard.ball_action, Some(BallAction::Pass { .. }))
    {
        return Status::Failure;
    }
    let Some(transform) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };
    let Some(target) = blackboard.kick_target else {
        return Status::Failure;
    };
    if is_goalkeeper_kick_approach_obstructed(blackboard, transform * target) {
        return Status::Failure;
    }
    Status::Success
}

fn enable_quick_clearance(blackboard: &mut Blackboard) -> Status {
    if let Some(BodyMotion::Kick {
        quick,
        strong,
        soft,
        target_speed,
        ..
    }) = &mut blackboard.body_motion
    {
        *quick = true;
        *strong = blackboard.parameters.kicking.allow_strong_kicks;
        *soft = false;
        *target_speed = blackboard.parameters.kicking.target_speed;
        Status::Success
    } else {
        Status::Failure
    }
}

pub(crate) fn goalkeeper_clearance_target(
    ball: Point2<Field>,
    field: &FieldDimensions,
) -> Point2<Field> {
    point![
        0.0,
        ball.y().clamp(
            -field.width / 2.0 + field.ball_radius,
            field.width / 2.0 - field.ball_radius
        )
    ]
}

pub(crate) fn goalkeeper_clearance_target_for_pose(
    blackboard: &Blackboard,
    keeper_pose: Pose2<Field>,
) -> Option<Point2<Field>> {
    let ball = blackboard.ball.as_ref()?.position;
    let transform = blackboard.world_state.robot.ground_to_field?;
    let heading = keeper_pose.as_transform::<Ground>() * vector![1.0, 0.0];
    if !ball
        .inner
        .iter()
        .chain(heading.inner.iter())
        .all(|value| value.is_finite())
    {
        return None;
    }
    // Keep a forward component, but prefer a quick sideways clearance to a long turn.
    let preferred = ball + vector![heading.x().max(0.25), heading.y()];
    let field = &blackboard.field_dimensions;
    let kicking = &blackboard.parameters.kicking;
    let distance = kicking.strong_kick_min_target_distance.max(3.0);
    for distance in [distance, distance / 2.0] {
        if let Some(target) = travel_openings(
            ball,
            transform,
            &blackboard.world_state.obstacles,
            field,
            distance,
            kicking.strategy.obstacle_clearance,
        )
        .into_iter()
        .filter(|opening| opening.length() > f32::EPSILON)
        .map(|opening| goalward_target(ball, preferred, opening))
        .filter(|target| target.x() > ball.x())
        .min_by(|a, b| {
            (a - ball)
                .angle(&(preferred - ball))
                .total_cmp(&(b - ball).angle(&(preferred - ball)))
        }) {
            return Some(target);
        }
    }
    // A crowded penalty area must not turn an urgent clearance into waiting.
    Some(goalkeeper_clearance_target(ball, field))
}

fn is_interception_velocity_towards_own_half(
    blackboard: &Blackboard,
    ball_velocity: Vector2<Field>,
) -> bool {
    let parameters = &blackboard.parameters.ball.interception;

    ball_velocity.norm() > parameters.minimum_ball_velocity && ball_velocity.x() < 0.0
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use geometry::{Distance, line_segment::LineSegment};
    use hsl_network_messages::{PlayerNumber, SubState, Team};
    use linear_algebra::{Isometry2, Point2, Pose2, Vector2, point, vector};
    use types::{
        field_dimensions::{FieldDimensions, Side},
        filtered_game_controller_state::FilteredGameControllerState,
        filtered_game_state::FilteredGameState,
        motion_command::BodyMotion,
        motion_type::MotionType,
        obstacles::Obstacle,
        parameters::{KickStrategyParameters, PassingParameters},
        path::traits::EndPoints,
        primary_state::PrimaryState,
        time_wrapper::TimeWrapper,
        world_state::{BallSource, BallState, PlayerState},
    };

    use super::{goalkeeper_subtree, shot_y_at_defense_line};
    use crate::{
        kick::strategy::BallAction,
        node::{Blackboard, LastBall},
    };

    fn clearance_board() -> Blackboard {
        let ball = LastBall {
            position: point![-3.5, 1.1],
            velocity: Vector2::zeros(),
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball: Some(ball.clone()),
            last_ball: Some(ball),
            last_motion_type: Some(MotionType::Walk),
            ..Default::default()
        };
        board.player_number = PlayerNumber::One;
        board.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![-4.2, 0.0], 0.0));
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board
            .parameters
            .goalkeeper
            .kick_away_ball_maximum_robot_distance = 1.5;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.kicking.target_speed = 3.0;
        board.parameters.walking.path_planning.robot_radius = 0.2;
        board.parameters.walking.path_planning.ball_obstacle_radius = 0.05;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board
    }

    #[test]
    fn keeper_can_follow_possession_upfield_only_with_confirmed_goal_cover() {
        let mut board = clearance_board();
        board.field_dimensions.length = 22.0;
        board.field_dimensions.width = 14.0;
        board.field_dimensions.goal_inner_width = 3.0;
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.robot.ground_to_field =
            Some(Isometry2::from_parts(vector![0.0, 0.0], 0.0));
        board.parameters.goalkeeper.striker_distance = 3.5;
        board.parameters.goalkeeper.x_offset = 0.3;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.network.hsl_striker_message_receive_timeout = Duration::from_secs(3);
        board.ball.as_mut().unwrap().position = point![0.6, 0.0];
        board.ball.as_mut().unwrap().age = board.world_state.now;
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            ..crate::test_support::filtered_game_controller_state()
        });
        board.world_state.fall_detection = Some(types::fall_detection::FallDetection {
            time: board.world_state.now,
            valid_until: board.world_state.now + Duration::from_secs(10),
            posture: types::fall_detection::Posture::Upright,
        });
        for (number, position) in [
            (PlayerNumber::Two, point![-10.7, 0.0]),
            (PlayerNumber::Three, point![-2.0, 3.0]),
        ] {
            board.world_state.player_states[number] = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: position.into(),
                    replacement_goalkeeper_claim: number == PlayerNumber::Two,
                    ..crate::test_support::player_state()
                },
            });
        }
        assert!(!super::is_ball_close_enough_to_goal_to_become_striker(
            &mut board
        ));
        crate::replacement_goalkeeper::refresh_replacement_goalkeeper_role(&mut board);
        board.world_state.now = board.world_state.now + Duration::from_millis(500);
        crate::replacement_goalkeeper::refresh_replacement_goalkeeper_role(&mut board);
        assert!(super::is_ball_close_enough_to_goal_to_become_striker(
            &mut board
        ));
        for case in 0..5 {
            let mut invalid = board.clone();
            let replacement = invalid.world_state.player_states.two.as_mut().unwrap();
            match case {
                0 => replacement.time = invalid.world_state.now - Duration::from_secs(3),
                1 => replacement.inner.replacement_goalkeeper_claim = false,
                2 => replacement.inner.pose = point![-5.0, 0.0].into(),
                3 => {
                    invalid
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .two = Some(hsl_network_messages::Penalty::PickUp {
                        remaining: Duration::from_secs(1),
                    })
                }
                _ => invalid.ball.as_mut().unwrap().position = point![6.0, 0.0],
            }
            assert!(
                !super::is_ball_close_enough_to_goal_to_become_striker(&mut invalid),
                "case {case}"
            );
        }
    }

    #[test]
    fn does_not_chase_a_ball_into_the_goal_net() {
        let transform = Isometry2::from_parts(vector![-6.7, 0.4], 0.0);
        let ball = LastBall {
            position: point![-7.2, 0.7],
            velocity: vector![0.0, 0.0],
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                goal_inner_width: 2.4,
                ..FieldDimensions::SPL_2025
            },
            ball: Some(ball.clone()),
            last_ball: Some(ball),
            last_motion_type: Some(types::motion_type::MotionType::Walk),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(transform);
        board.parameters.goalkeeper.x_offset = 0.3;
        board
            .parameters
            .goalkeeper
            .kick_away_ball_maximum_robot_distance = 1.5;
        board
            .world_state
            .obstacles
            .push(types::obstacles::Obstacle::robot(
                point![-0.6, 0.4],
                0.2,
                0.2,
            ));
        let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
        assert_eq!(status, types::behavior_tree::Status::Success);
        approx::assert_relative_eq!(transform * board.walk_position.unwrap(), point![-6.7, 0.0]);
        assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
    }

    #[test]
    fn keeper_walks_around_post_before_entering_obstacle_blind_kick() {
        use types::{obstacles::Obstacle, path::traits::PathProgress, world_state::BallState};

        for side in [-1.0, 1.0] {
            for angle in [0.0, 0.7 * side] {
                let transform = Isometry2::from_parts(vector![-6.9, side * 1.7], angle);
                let ball = LastBall {
                    position: point![-6.6, side * 0.8],
                    velocity: Vector2::zeros(),
                    age: Default::default(),
                    field_side: Side::Left,
                    source: BallSource::Own,
                };
                let mut board = Blackboard {
                    field_dimensions: FieldDimensions {
                        length: 14.0,
                        width: 9.0,
                        goal_inner_width: 2.4,
                        ..FieldDimensions::SPL_2025
                    },
                    ball: Some(ball.clone()),
                    last_ball: Some(ball.clone()),
                    last_motion_type: Some(types::motion_type::MotionType::Walk),
                    ..Default::default()
                };
                board.world_state.robot.ground_to_field = Some(transform);
                board.world_state.now = ros_z::time::Time::from_nanos(1_000_000_000);
                board
                    .parameters
                    .goalkeeper
                    .kick_away_ball_maximum_robot_distance = 1.5;
                board.parameters.kicking.approach_ball_standoff = 0.3;
                board.parameters.walking.path_planning.robot_radius = 0.4;
                board
                    .parameters
                    .walking
                    .path_planning
                    .obstacle_escape_spline_segments = 5;
                board.parameters.walking.speed.velocity_fade_distance = 3.0;
                board.world_state.ball = Some(BallState {
                    ball_in_ground: transform.inverse() * ball.position,
                    ..types::world_state::BallState::new_at_center(Default::default())
                });
                let post = point![-7.025, side * 1.25];
                board.world_state.obstacles = vec![
                    Obstacle::goal_post(transform.inverse() * post, 0.2),
                    Obstacle::robot(transform.inverse() * point![-5.2, side * 0.8], 0.2, 0.2),
                ];

                let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
                assert_eq!(status, types::behavior_tree::Status::Success);
                let Some(BodyMotion::Walk { path, .. }) = &board.body_motion else {
                    panic!(
                        "post on final approach must keep keeper out of Kick: {:?}",
                        board.body_motion
                    );
                };
                assert!(path.segments.len() > 1, "must plan a post detour");
                let forward = transform * path.forward(Point2::origin());
                assert!(forward.x() > 0.0, "detour must start toward the field");

                let side_transform = Isometry2::from_parts(vector![-6.55, side * 1.39], 0.0);
                board.world_state.robot.ground_to_field = Some(side_transform);
                for obstacle in &mut board.world_state.obstacles {
                    obstacle.position = side_transform.inverse() * (transform * obstacle.position);
                }
                board.world_state.ball.as_mut().unwrap().ball_in_ground =
                    side_transform.inverse() * ball.position;
                goalkeeper_subtree().tick_with_trace(&mut board);
                assert!(
                    matches!(board.body_motion, Some(BodyMotion::Walk { .. })),
                    "clear ball line must not hand over before alignment beside the post"
                );

                let beside_post_transform = Isometry2::from_parts(vector![-6.9, side * 0.65], 0.0);
                board.world_state.robot.ground_to_field = Some(beside_post_transform);
                for obstacle in &mut board.world_state.obstacles {
                    obstacle.position =
                        beside_post_transform.inverse() * (side_transform * obstacle.position);
                }
                board.world_state.ball.as_mut().unwrap().ball_in_ground =
                    beside_post_transform.inverse() * ball.position;
                goalkeeper_subtree().tick_with_trace(&mut board);
                assert!(
                    matches!(board.body_motion, Some(BodyMotion::Kick { .. })),
                    "nearby post must not prevent a clear final approach"
                );

                let clear_transform = Isometry2::from_parts(vector![-6.9, side * 0.3], 0.0);
                board.world_state.robot.ground_to_field = Some(clear_transform);
                for obstacle in &mut board.world_state.obstacles {
                    obstacle.position =
                        clear_transform.inverse() * (beside_post_transform * obstacle.position);
                }
                board.world_state.ball.as_mut().unwrap().ball_in_ground =
                    clear_transform.inverse() * ball.position;
                goalkeeper_subtree().tick_with_trace(&mut board);
                assert!(
                    matches!(board.body_motion, Some(BodyMotion::Kick { .. })),
                    "clear approach should resume normal clearance"
                );
            }
        }
    }

    #[test]
    fn opponent_corners_keep_center_until_a_shot_without_changing_open_play() {
        for side in [-1.0, 1.0] {
            let transform = Isometry2::from_parts(vector![-6.7, 0.0], 0.0);
            let mut board = Blackboard {
                field_dimensions: FieldDimensions {
                    length: 14.0,
                    width: 9.0,
                    goal_inner_width: 2.4,
                    penalty_area_length: 3.0,
                    ..FieldDimensions::SPL_2025
                },
                ..Default::default()
            };
            board.world_state.robot.ground_to_field = Some(transform);
            board.last_motion_type = Some(types::motion_type::MotionType::Walk);
            board.parameters.goalkeeper.x_offset = 0.3;
            board.parameters.goalkeeper.distance_to_goalpost = 0.15;
            board.parameters.goalkeeper.passive_distance = 4.5;
            board
                .parameters
                .goalkeeper
                .active_defense_maximum_robot_distance = 2.0;
            board.parameters.ball.interception.minimum_ball_velocity = 1.0;
            board.parameters.walking.speed.blocking = 0.8;
            board.parameters.walking.speed.minimum_speed = 0.2;
            board.parameters.walking.speed.velocity_fade_distance = 3.0;
            board.walking_velocity_limits.forward_velocity_limits = [-0.5, 1.0];
            board.walking_velocity_limits.lateral_velocity_limit = 1.0;
            board.walking_velocity_limits.angular_velocity_limit = 2.0;
            let near_post_y = 2.8_f32 / 0.2_f32.hypot(2.8) * 1.05;
            for (sub_state, kicking_team, position, velocity, expected, is_shot) in [
                (
                    Some(SubState::CornerKick),
                    Team::Opponent,
                    point![-7.0, side * 4.5],
                    vector![0.0, 0.0],
                    point![-6.7, 0.0],
                    false,
                ),
                (
                    Some(SubState::CornerKick),
                    Team::Opponent,
                    point![-6.0, side * 2.0],
                    vector![1.0, -2.0 * side],
                    point![-6.7, 0.0],
                    false,
                ),
                (
                    Some(SubState::CornerKick),
                    Team::Opponent,
                    point![-5.5, side * 0.6],
                    vector![-3.0, -0.8 * side],
                    point![-6.7, side * 0.28],
                    true,
                ),
                (
                    Some(SubState::ThrowIn),
                    Team::Opponent,
                    point![-5.5, side * 0.6],
                    vector![-3.0, -0.8 * side],
                    point![-6.7, side * 0.28],
                    true,
                ),
                (
                    Some(SubState::IndirectFreeKick),
                    Team::Opponent,
                    point![-5.5, side * 0.6],
                    vector![-3.0, -0.8 * side],
                    point![-6.7, side * 0.28],
                    true,
                ),
                (
                    Some(SubState::DirectFreeKick),
                    Team::Opponent,
                    point![-5.5, side * 0.6],
                    vector![-3.0, -0.8 * side],
                    point![-6.7, side * 0.28],
                    true,
                ),
                (
                    None,
                    Team::Opponent,
                    point![-6.0, side * 2.0],
                    vector![1.0, -2.0 * side],
                    point![-6.7, 0.0],
                    false,
                ),
                (
                    None,
                    Team::Opponent,
                    point![-5.2, side * 0.8],
                    vector![1.0, -2.0 * side],
                    point![-6.7, 0.0],
                    false,
                ),
                (
                    None,
                    Team::Opponent,
                    point![-5.5, side * 0.6],
                    vector![-3.0, -0.8 * side],
                    point![-6.7, side * 0.28],
                    true,
                ),
                (
                    None,
                    Team::Opponent,
                    point![-6.8, side * 2.8],
                    vector![1.0, 2.0 * side],
                    point![-6.85, side * near_post_y],
                    false,
                ),
                (
                    None,
                    Team::Opponent,
                    point![-6.8, side * 2.8],
                    vector![0.0, 0.0],
                    point![-6.85, side * near_post_y],
                    false,
                ),
                (
                    Some(SubState::CornerKick),
                    Team::Hulks,
                    point![7.0, side * 4.5],
                    vector![0.0, 0.0],
                    point![-6.7, side * 4.5 * 0.3 / 14.0],
                    false,
                ),
            ] {
                board.world_state.filtered_game_controller_state =
                    Some(FilteredGameControllerState {
                        sub_state,
                        kicking_team: Some(kicking_team),
                        ..crate::test_support::filtered_game_controller_state()
                    });
                board.ball = Some(LastBall {
                    position,
                    velocity,
                    age: Default::default(),
                    field_side: Side::Left,
                    source: BallSource::Own,
                });
                board.last_ball = board.ball.clone();
                board.body_motion = None;
                board.walk_position = None;
                let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
                assert_eq!(status, types::behavior_tree::Status::Success);
                let target = board
                    .walk_position
                    .or_else(|| match &board.body_motion {
                        Some(BodyMotion::Walk { path, .. }) => Some(path.end_point()),
                        _ => None,
                    })
                    .unwrap();
                approx::assert_relative_eq!(transform * target, expected, epsilon = 1e-4);
                if is_shot {
                    assert!(matches!(
                        board.body_motion,
                        Some(BodyMotion::WalkWithVelocity { .. })
                    ));
                }
            }
        }
    }

    #[test]
    fn clears_unpressured_penalty_area_balls_with_three_or_five_players() {
        for players in [3, 5] {
            for sub_state in [None, Some(SubState::DirectFreeKick)] {
                for allow_strong_kicks in [false, true] {
                    let mut board = clearance_board();
                    board.parameters.kicking.allow_strong_kicks = allow_strong_kicks;
                    board.world_state.filtered_game_controller_state =
                        Some(FilteredGameControllerState {
                            sub_state,
                            kicking_team: Some(Team::Hulks),
                            ..crate::test_support::filtered_game_controller_state()
                        });
                    let ball = board.ball.as_ref().unwrap().position;
                    for (index, number) in [
                        PlayerNumber::Two,
                        PlayerNumber::Three,
                        PlayerNumber::Four,
                        PlayerNumber::Five,
                    ]
                    .into_iter()
                    .take(players - 1)
                    .enumerate()
                    {
                        board.world_state.player_states[number] = Some(TimeWrapper {
                            time: board.world_state.now,
                            inner: PlayerState {
                                // All teammates are closer; clearance must bypass election.
                                pose: Pose2::new(
                                    ball + vector![0.15 * (index + 1) as f32, 0.0],
                                    0.0,
                                ),
                                ..crate::test_support::player_state()
                            },
                        });
                    }
                    let transform = board.world_state.robot.ground_to_field.unwrap();
                    let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
                    assert_eq!(status, types::behavior_tree::Status::Success);
                    let Some(BodyMotion::Kick {
                        strong,
                        quick: true,
                        soft: false,
                        target_speed,
                        ..
                    }) = board.body_motion
                    else {
                        panic!(
                            "unpressured reachable ball must clear: {:?}",
                            board.body_motion
                        );
                    };
                    assert_eq!(strong, allow_strong_kicks);
                    approx::assert_relative_eq!(target_speed, 3.0);
                    assert!((transform * board.kick_target.unwrap()).x() > ball.x());
                    assert!(board.voronoi_map.is_none());
                }
            }
        }
    }

    #[test]
    fn clearance_requires_reach_and_own_penalty_area_but_not_a_stationary_ball() {
        let mut board = clearance_board();
        board.ball.as_mut().unwrap().velocity = vector![1.0, 0.0];
        assert!(super::is_goalkeeper_kick_away_needed(&mut board));

        for (keeper, ball) in [
            (point![-4.2, 0.0], point![-2.8, 0.0]),
            (point![-3.5, 1.0], point![-3.5, 2.0]),
            (point![-4.5, 0.0], point![-3.0, 0.0]),
        ] {
            board.world_state.robot.ground_to_field = Some(Pose2::new(keeper, 0.0).as_transform());
            board.ball.as_mut().unwrap().position = ball;
            assert!(!super::is_goalkeeper_kick_away_needed(&mut board));
        }
    }

    #[test]
    fn keeper_pressure_radius_fits_penalty_dimensions() {
        let mut board = clearance_board();
        let transform = board.world_state.robot.ground_to_field.unwrap();
        let ball = board.ball.as_ref().unwrap().position;
        for (length, width) in [(1.65, 4.0), (3.5, 7.0), (3.5, 5.0)] {
            board.field_dimensions.penalty_area_length = length;
            board.field_dimensions.penalty_area_width = width;
            let radius = 0.85 * length.min(width / 2.0);
            for direction in [vector![1.0, 0.0], vector![0.0, 1.0]] {
                for (offset, pressured) in [(-0.01, true), (0.01, false)] {
                    board.world_state.obstacles = vec![Obstacle::robot(
                        transform.inverse() * (ball + direction * (radius + 0.2 + offset)),
                        0.2,
                        0.2,
                    )];
                    assert_eq!(super::is_goalkeeper_under_pressure(&board), pressured);
                }
            }
        }
    }

    #[test]
    fn calm_keeper_announces_a_controlled_pass_and_clears_when_pressure_arrives() {
        let mut board = clearance_board();
        board.field_dimensions.length = 14.0;
        board.field_dimensions.width = 9.0;
        board.field_dimensions.penalty_area_length = 3.0;
        board.field_dimensions.penalty_area_width = 6.0;
        board.field_dimensions.goal_box_area_length = 1.0;
        let transform = Isometry2::from_parts(vector![-6.7, 0.0], 0.0);
        board.world_state.robot.ground_to_field = Some(transform);
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        let ball = point![-6.0 - 1e-6, 0.0];
        board.ball.as_mut().unwrap().position = ball;
        board.ball.as_mut().unwrap().age = board.world_state.now;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.strategy = KickStrategyParameters {
            maximum_shot_distance: 3.0,
            maximum_dribble_distance: 3.0,
            pressure_distance: 0.8,
            passing: PassingParameters {
                maximum_distance: 8.0,
                minimum_distance: 1.4,
                minimum_progress: 0.25,
                receiver_standoff: 0.5,
                target_half_width: 0.2,
                maximum_receiver_age: Duration::from_secs(3),
                ..Default::default()
            },
            ..Default::default()
        };
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            remaining_time_in_half: Duration::from_secs(600),
            remaining_number_of_messages: 1000,
            ..crate::test_support::filtered_game_controller_state()
        });
        board.world_state.player_states[PlayerNumber::Two] = Some(TimeWrapper {
            time: board.world_state.now,
            inner: PlayerState {
                pose: Pose2::new(point![-1.5, 0.0], 0.0),
                ..crate::test_support::player_state()
            },
        });
        board.world_state.obstacles = vec![Obstacle::robot(
            transform.inverse() * (ball + vector![4.0, 3.5]),
            0.2,
            0.2,
        )];
        assert!(!super::is_goalkeeper_under_pressure(&board));
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert_eq!(
            board.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Two
            })
        );
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert!(board.try_sending_state_message().is_some());
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                quick: false,
                strong: false,
                soft: false,
                ..
            })
        ));

        let mut beside_goal_line = board.clone();
        beside_goal_line.ball.as_mut().unwrap().position = point![-6.4, 0.8];
        assert_eq!(
            super::try_goalkeeper_pass(&mut beside_goal_line),
            types::behavior_tree::Status::Failure
        );
        goalkeeper_subtree().tick_with_trace(&mut beside_goal_line);
        assert!(matches!(
            beside_goal_line.body_motion,
            Some(BodyMotion::Kick {
                quick: true,
                strong: true,
                ..
            })
        ));

        board.last_motion_type = Some(MotionType::Kick);
        board.world_state.obstacles[0].position = transform.inverse() * (ball + vector![1.5, 1.5]);
        assert!(super::is_goalkeeper_under_pressure(&board));
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                quick: true,
                strong: true,
                soft: false,
                ..
            })
        ));
        assert!(board.ball_action.is_none());
        assert!((transform * board.kick_target.unwrap()).x() > ball.x());
    }

    #[test]
    fn keeper_retrieves_slow_post_balls_beyond_kick_range_without_stealing_possession() {
        for side in [-1.0, 1.0] {
            let mut board = clearance_board();
            board.field_dimensions.length = 14.0;
            board.field_dimensions.width = 9.0;
            board.field_dimensions.goal_inner_width = 2.4;
            board.field_dimensions.penalty_area_length = 3.0;
            board.field_dimensions.penalty_area_width = 6.0;
            let transform = Isometry2::from_parts(vector![-6.7, 0.0], 0.0);
            board.world_state.robot.ground_to_field = Some(transform);
            board.world_state.now = board.world_state.now + Duration::from_secs(10);
            board.parameters.goalkeeper.striker_distance = 3.5;
            board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
            board
                .parameters
                .ball
                .interception
                .minimum_ball_velocity_towards_robot = 0.7;
            board
                .parameters
                .kicking
                .strategy
                .passing
                .maximum_receiver_age = Duration::from_secs(3);
            let ball = point![-6.5, side * 1.7];
            board.ball.as_mut().unwrap().position = ball;
            board.ball.as_mut().unwrap().velocity = vector![0.1, -0.2 * side];
            let post = point![-7.025, side * 1.25];
            board.world_state.obstacles =
                vec![Obstacle::goal_post(transform.inverse() * post, 0.2)];
            assert!(super::is_goalkeeper_kick_away_needed(&mut board));

            let mut controlled = board.clone();
            controlled.world_state.player_states[PlayerNumber::Two] = Some(TimeWrapper {
                time: controlled.world_state.now,
                inner: PlayerState {
                    pose: Pose2::new(ball + vector![0.2, 0.0], 0.0),
                    ..crate::test_support::player_state()
                },
            });
            assert!(!super::is_goalkeeper_kick_away_needed(&mut controlled));
            controlled.world_state.player_states[PlayerNumber::Two]
                .as_mut()
                .unwrap()
                .time = controlled.world_state.now - Duration::from_secs(3);
            assert!(super::is_goalkeeper_kick_away_needed(&mut controlled));
            let mut distant = board.clone();
            distant.ball.as_mut().unwrap().position = point![-5.7, side * 1.7];
            assert!(!super::is_goalkeeper_kick_away_needed(&mut distant));
            let mut fast = board.clone();
            fast.ball.as_mut().unwrap().velocity = vector![-3.0, 0.0];
            assert!(!super::is_goalkeeper_kick_away_needed(&mut fast));

            goalkeeper_subtree().tick_with_trace(&mut board);
            assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
            let stance = transform * board.walk_position.unwrap();
            assert!(
                stance.y().abs() < ball.y().abs(),
                "approach must follow the rolling ball"
            );
            assert!(stance.x() > -board.field_dimensions.length / 2.0);

            let aligned = Isometry2::from_parts((ball - vector![0.3, 0.0]).coords(), 0.0);
            board.world_state.robot.ground_to_field = Some(aligned);
            board.world_state.obstacles[0].position = aligned.inverse() * post;
            goalkeeper_subtree().tick_with_trace(&mut board);
            assert!(
                matches!(
                    board.body_motion,
                    Some(BodyMotion::Kick { quick: true, .. })
                ),
                "post pickup must hand over to clearance once approach is safe"
            );
        }
    }

    #[test]
    fn keeper_approaches_rear_balls_and_turns_before_backward_clearances() {
        for angle in [0.0, std::f32::consts::PI] {
            let mut board = clearance_board();
            let transform = Isometry2::from_parts(vector![-3.4, 0.0], angle);
            let ball = point![-3.9, 0.0];
            board.world_state.robot.ground_to_field = Some(transform);
            board.ball.as_mut().unwrap().position = ball;
            board.last_ball = board.ball.clone();
            board.world_state.ball = Some(BallState {
                ball_in_ground: transform.inverse() * ball,
                ..types::world_state::BallState::new_at_center(Default::default())
            });
            if angle != 0.0 {
                assert!((transform.inverse() * ball).x() > 0.0);
            }
            let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
            assert_eq!(status, types::behavior_tree::Status::Success);
            let Some(BodyMotion::Walk {
                path,
                target_orientation,
                ..
            }) = &board.body_motion
            else {
                panic!(
                    "unsafe clearance must approach first: {:?}",
                    board.body_motion
                );
            };
            assert!(
                path.segments.len() > 1,
                "approach must detour around the ball"
            );
            let stance = transform * board.walk_position.unwrap();
            assert!(stance.x() < ball.x());
            assert!(stance.x() > -board.field_dimensions.length / 2.0);
            let heading = transform * target_orientation.as_unit_vector();
            assert!(heading.x() > 0.0, "approach must face away from own goal");
            assert!(board.kick_target.is_none());

            board.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
                (ball - vector![board.parameters.kicking.approach_ball_standoff, 0.0]).coords(),
                0.0,
            ));
            board.world_state.ball = None;
            goalkeeper_subtree().tick_with_trace(&mut board);
            assert!(matches!(
                board.body_motion,
                Some(BodyMotion::Kick { quick: true, .. })
            ));
        }
    }

    #[test]
    fn clearance_prefers_keeper_heading_and_avoids_blocked_lanes() {
        for angle in [-1.2, 0.0, 1.2] {
            let mut board = clearance_board();
            board.field_dimensions.length = 14.0;
            board.field_dimensions.width = 9.0;
            let transform = Isometry2::from_parts(vector![-6.7, 0.0], angle);
            let ball = point![-6.0, 0.0];
            board.world_state.robot.ground_to_field = Some(transform);
            board.ball.as_mut().unwrap().position = ball;
            board.parameters.kicking.strategy.obstacle_clearance = 0.1;
            let heading = transform * vector![1.0, 0.0];
            let target =
                super::goalkeeper_clearance_target_for_pose(&board, transform.as_pose()).unwrap();
            assert!((target - ball).angle(&heading) < 1e-3);

            let blocker = ball + heading;
            board.world_state.obstacles =
                vec![Obstacle::robot(transform.inverse() * blocker, 0.3, 0.3)];
            let target =
                super::goalkeeper_clearance_target_for_pose(&board, transform.as_pose()).unwrap();
            assert!(target.x() > ball.x());
            assert!(target.x().abs() < board.field_dimensions.length / 2.0);
            assert!(target.y().abs() < board.field_dimensions.width / 2.0);
            assert!(LineSegment(ball, target).distance_to(blocker) >= 0.45 - 1e-4);
        }
    }

    #[test]
    fn keeper_starts_walking_for_shot_from_halfway() {
        let transform = Isometry2::from_parts(vector![-4.2, 0.0], 0.0);
        let ball = LastBall {
            position: point![0.0, 1.5],
            velocity: vector![-3.0, -0.8],
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ball: Some(ball.clone()),
            last_ball: Some(ball),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(transform);
        board.parameters.goalkeeper.x_offset = 0.3;
        board.parameters.goalkeeper.passive_distance = 1.0;
        board.parameters.ball.interception.minimum_ball_velocity = 1.0;
        board
            .parameters
            .ball
            .interception
            .minimum_ball_velocity_towards_own_half = 0.7;
        let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
        assert_eq!(status, types::behavior_tree::Status::Success);
        assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
        let target = transform * board.walk_position.unwrap();
        approx::assert_relative_eq!(target.x(), -4.2);
        approx::assert_relative_eq!(target.y(), 0.38, epsilon = 1e-5);
    }

    #[test]
    fn keeper_leaves_ball_goal_line_for_fast_angled_shot() {
        for side in [-1.0, 1.0] {
            let goal_to_ball = vector![0.7_f32, side];
            let ready_position = point![-4.5, 0.0] + goal_to_ball.normalize() * 0.6;
            let transform = Isometry2::from_parts(ready_position.coords(), 0.0);
            let ball = LastBall {
                position: point![-3.8, side],
                velocity: vector![-0.5, -1.2 * side],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            };
            let mut board = Blackboard {
                field_dimensions: FieldDimensions::SPL_2025,
                ball: Some(ball.clone()),
                last_ball: Some(ball),
                ..Default::default()
            };
            board.world_state.robot.ground_to_field = Some(transform);
            board.last_motion_type = Some(types::motion_type::MotionType::Walk);
            board.parameters.goalkeeper.x_offset = 0.3;
            board.parameters.goalkeeper.distance_to_goalpost = 0.15;
            board.parameters.goalkeeper.passive_distance = 4.5;
            board
                .parameters
                .goalkeeper
                .active_defense_maximum_robot_distance = 2.0;
            board.parameters.ball.interception.minimum_ball_velocity = 1.0;
            board
                .parameters
                .ball
                .interception
                .minimum_ball_velocity_towards_own_half = 0.7;
            board.parameters.ball.interception.maximum_ball_distance = 3.5;
            board
                .parameters
                .ball
                .interception
                .maximum_intercept_distance = 1.0;
            board
                .parameters
                .kicking
                .minimum_interception_forward_distance = 0.3;
            let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
            assert_eq!(status, types::behavior_tree::Status::Success);
            let target = transform * board.walk_position.unwrap();
            approx::assert_relative_eq!(target.x(), -4.2, epsilon = 1e-5);
            approx::assert_relative_eq!(target.y(), 0.04 * side, epsilon = 1e-5);
            assert!(matches!(board.body_motion, Some(BodyMotion::Walk { .. })));
        }
    }

    #[test]
    fn keeper_commands_enough_speed_to_reach_fast_shot() {
        let transform = Isometry2::from_parts(vector![-6.7, 0.0], 0.0);
        let ball = LastBall {
            position: point![-4.5, -0.6],
            velocity: vector![-2.8, 1.6],
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                goal_inner_width: 2.6,
                ..FieldDimensions::SPL_2025
            },
            ball: Some(ball.clone()),
            last_ball: Some(ball),
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(transform);
        board.last_motion_type = Some(types::motion_type::MotionType::Walk);
        board.parameters.goalkeeper.x_offset = 0.3;
        board.parameters.goalkeeper.distance_to_goalpost = 0.15;
        board.parameters.ball.interception.minimum_ball_velocity = 1.0;
        board.parameters.ball.interception.maximum_ball_distance = 3.5;
        board
            .parameters
            .ball
            .interception
            .maximum_intercept_distance = 1.0;
        board
            .parameters
            .kicking
            .minimum_interception_forward_distance = 0.3;
        board.parameters.walking.speed.blocking = 0.8;
        board.parameters.walking.speed.minimum_speed = 0.2;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.walking_velocity_limits.forward_velocity_limits = [-0.5, 1.0];
        board.walking_velocity_limits.lateral_velocity_limit = 1.0;
        board.walking_velocity_limits.angular_velocity_limit = 2.0;
        let (status, _) = goalkeeper_subtree().tick_with_trace(&mut board);
        assert_eq!(status, types::behavior_tree::Status::Success);
        let target = board.walk_position.unwrap();
        let time_to_line = 2.2 / 2.8;
        let commanded_speed = match board.body_motion.as_ref().unwrap() {
            BodyMotion::Walk { speed, .. } => *speed,
            BodyMotion::WalkWithVelocity { velocity, .. } => velocity.norm(),
            motion => panic!("expected goalkeeper walk, got {motion:?}"),
        };
        assert!(
            commanded_speed * time_to_line >= target.coords().norm() - 1e-4,
            "commanded {commanded_speed}m/s cannot cover {}m before shot in {time_to_line}s",
            target.coords().norm()
        );
        board.ball.as_mut().unwrap().velocity = vector![-3.5, 2.4];
        board.last_ball = board.ball.clone();
        goalkeeper_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = &board.body_motion else {
            panic!("expected deadline walk for fast angled shot");
        };
        approx::assert_relative_eq!(velocity.y(), 1.0);
        board.walking_velocity_limits.lateral_velocity_limit = 1.5;
        goalkeeper_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = &board.body_motion else {
            panic!("expected faster deadline walk when lateral limit permits");
        };
        assert!(velocity.y() > 1.0);
        approx::assert_relative_eq!(
            velocity.y() * (2.2 / 3.5),
            board.walk_position.unwrap().y(),
            epsilon = 1e-5
        );
        board.ball.as_mut().unwrap().velocity = vector![-2.8, 1.6];
        board.last_ball = board.ball.clone();
        board.walking_velocity_limits.lateral_velocity_limit = 0.25;
        goalkeeper_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity {
            velocity,
            angular_velocity,
        }) = &board.body_motion
        else {
            panic!("expected policy-limited deadline walk");
        };
        approx::assert_relative_eq!(velocity.y(), 0.25);
        assert!(angular_velocity.abs() <= board.walking_velocity_limits.angular_velocity_limit);

        board.parameters.walking.path_planning.robot_radius = 0.15;
        board
            .world_state
            .obstacles
            .push(types::obstacles::Obstacle::robot(
                point![0.0, 0.35],
                0.1,
                0.1,
            ));
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::Walk { .. })),
            "planned detours must retain ordinary path following"
        );

        board.world_state.obstacles.clear();
        board
            .world_state
            .rule_obstacles
            .push(types::rule_obstacles::RuleObstacle::Rectangle(
                geometry::rectangle::Rectangle {
                    min: point![-6.9, 0.4],
                    max: point![-6.5, 0.9],
                },
            ));
        goalkeeper_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::Walk { path, .. }) = &board.body_motion else {
            panic!("blocked planner fallback must not use deadline velocity");
        };
        let [types::path::PathSegment::LineSegment(line)] = path.segments.as_slice() else {
            panic!("expected a hold when no safe planned route exists");
        };
        assert_eq!(line.0, line.1);
        board.world_state.rule_obstacles.clear();

        board.world_state.now = ros_z::time::Time::from_nanos(1_000_000_000);
        board.world_state.obstacles = vec![types::obstacles::Obstacle::robot(
            point![1.7, 0.0],
            0.1,
            0.1,
        )];
        board
            .parameters
            .goalkeeper
            .kick_away_ball_maximum_robot_distance = 1.5;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.strong_kick_min_target_distance = 1.0;
        board.walking_velocity_limits.lateral_velocity_limit = 1.0;
        board.ball.as_mut().unwrap().position = point![-6.0, 0.2571429];
        board.last_ball = board.ball.clone();
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::WalkWithVelocity { .. })),
            "nearby opponent must not turn an incoming goal shot into a clearance: {:?}",
            board.body_motion
        );

        board.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
        board.last_ball = board.ball.clone();
        board.parameters.control.allow_switch.walk = Duration::from_millis(500);
        board.last_motion_switch_time = board.world_state.now - Duration::from_millis(100);
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    strong: true,
                    quick: true,
                    soft: false,
                    ..
                })
            ),
            "settled ball under pressure should clear quickly without waiting for walking dwell"
        );

        board.world_state.obstacles.clear();
        board.ball.as_mut().unwrap().position = point![-6.45, 0.2571429];
        board.last_ball = board.ball.clone();
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    strong: true,
                    quick: true,
                    ..
                })
            ),
            "unpressured goal-mouth ball should clear without waiting for a striker election"
        );

        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.ball.as_mut().unwrap().position = point![-6.2, 0.05];
        board.ball.as_mut().unwrap().velocity = vector![-3.0, 0.0];
        board.last_ball = board.ball.clone();
        goalkeeper_subtree().tick_with_trace(&mut board);
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = board.body_motion else {
            panic!("expected small blocking steps for a close frontal shot");
        };
        approx::assert_relative_eq!(velocity.norm(), 0.2, epsilon = 1e-5);

        board.ball.as_mut().unwrap().position = point![-5.8, 0.2];
        board.ball.as_mut().unwrap().velocity = vector![-2.8, 1.6];
        board.last_ball = board.ball.clone();
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(
                board.body_motion,
                Some(BodyMotion::Kick {
                    strong: false,
                    soft: false,
                    ..
                })
            ),
            "close interceptable balls should still use the kick policy"
        );

        board.parameters.control.allow_switch.walk = Duration::from_millis(500);
        board.last_motion_switch_time = board.world_state.now - Duration::from_millis(100);
        goalkeeper_subtree().tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::Kick { .. })),
            "a close interception must not wait for the walking dwell"
        );
    }

    #[test]
    fn blocks_distant_shots_at_the_defense_line() {
        let target = shot_y_at_defense_line(point!(0.0, 1.5), vector!(-3.0, -0.8), -4.5, -4.0, 0.8);
        approx::assert_relative_eq!(target.unwrap(), 1.5 - 0.8 * 4.0 / 3.0);
    }

    #[test]
    fn ignores_shots_wide_away_or_already_past_defense_line() {
        for (ball, velocity) in [
            (point!(0.0, 1.5), vector!(-3.0, 0.0)),
            (point!(0.0, 0.0), vector!(3.0, 0.0)),
            (point!(0.0, 0.0), vector!(0.0, 0.0)),
            (point!(0.0, 0.0), vector!(f32::NEG_INFINITY, 0.0)),
            (point!(f32::NAN, 0.0), vector!(-3.0, 0.0)),
            (point!(-4.2, 0.0), vector!(-3.0, 0.0)),
        ] {
            assert!(shot_y_at_defense_line(ball, velocity, -4.5, -4.0, 0.8).is_none());
        }
    }
}
