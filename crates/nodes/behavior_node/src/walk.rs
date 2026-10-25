use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, Orientation2, Point, Point2, Pose2, Vector2, point};
use path_planner::path_planner::PathPlanner;
use types::{
    behavior_tree::Status,
    field_dimensions::FieldDimensions,
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
    motion_type::MotionType,
    obstacles::ObstacleKind,
    parameters::{KickoffParameters, VoronoiParameters},
    path::{Path, PathSegment, traits::Project},
};
use voronoi::{Ownership, VoronoiGrid};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::{has_more_than_unpenalized_players, opponent_near_ball},
    kick::{kick, select_fallback_kick_target, strategy::BallAction, use_last_kick_settings},
    node::{Blackboard, ball_timeout},
    selection, sequence, subtree,
    switch_motion_type::{is_last_motion_type, switch_motion_type},
};

pub(crate) fn adaptive_blocking_velocity(
    blackboard: &Blackboard,
    desired: Vector2<Ground>,
    ball: Point2<Ground>,
) -> Vector2<Ground> {
    let limits = blackboard.walking_velocity_limits;
    if limits.validate().is_err()
        || ![desired.x(), desired.y(), ball.x(), ball.y()]
            .into_iter()
            .all(f32::is_finite)
    {
        return Vector2::zeros();
    }
    let forward_limit = if desired.x() < 0.0 {
        -limits.forward_velocity_limits[0]
    } else {
        limits.forward_velocity_limits[1]
    };
    if forward_limit == 0.0 && desired.x() != 0.0 {
        return Vector2::zeros();
    }
    let scale = (desired.x().abs() / forward_limit.max(f32::EPSILON))
        .max(desired.y().abs() / limits.lateral_velocity_limit)
        .max(1.0);
    let mut velocity = desired / scale;
    let parameters = &blackboard.parameters;
    let body_half_width = parameters.walking.path_planning.robot_radius / 2.0;
    let near_front = ball.x() >= 0.0
        && ball.coords().norm() <= 2.0 * parameters.kicking.approach_ball_standoff
        && ball.y().abs() <= body_half_width + blackboard.field_dimensions.ball_radius;
    let speed = velocity.norm();
    let closing_speed = parameters.walking.speed.minimum_speed.max(0.0);
    if near_front && speed > closing_speed {
        // Small steps near the body reduce leg opening while blocking a frontal ball.
        velocity *= closing_speed / speed;
    }
    if velocity.x() * ball.x() > 0.0
        && let Some(toward_ball) = ball.coords().try_normalize(f32::EPSILON)
    {
        let clearance = parameters
            .kicking
            .approach_ball_standoff
            .max(body_half_width + blackboard.field_dimensions.ball_radius);
        let maximum_closing_speed = ((ball.coords().norm() - clearance) / 0.35).max(0.0);
        let closing_speed = velocity.dot(&toward_ball);
        if closing_speed > maximum_closing_speed {
            // Direct velocity commands bypass the path follower's arrival braking.
            velocity *= maximum_closing_speed / closing_speed;
        }
    }
    velocity
}

pub fn plan(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Option<Path> {
    plan_with_ball_obstacle(blackboard, target_in_ground, ground_to_field, true, false)
}

pub(crate) fn plan_interception(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Option<Path> {
    let contested = opponent_near_ball(
        blackboard,
        blackboard.parameters.kicking.strategy.pressure_distance,
    );
    plan_with_ball_obstacle(
        blackboard,
        target_in_ground,
        ground_to_field,
        contested,
        true,
    )
}

fn plan_with_ball_obstacle(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
    avoid_ball: bool,
    interception: bool,
) -> Option<Path> {
    let parameters: &types::parameters::PathPlanningParameters =
        &blackboard.parameters.walking.path_planning;
    let field_dimensions = blackboard.field_dimensions;

    let mut planner = PathPlanner {
        obstacle_escape_spline_segments: parameters.obstacle_escape_spline_segments,
        ..Default::default()
    };
    planner.with_last_motion(
        blackboard.last_motion_command.as_ref(),
        parameters.rotation_penalty_factor,
    );
    let target_in_field = ground_to_field * target_in_ground;
    let own_goal_x = -field_dimensions.length / 2.0;
    let keeper_defending = blackboard.player_number
        == blackboard.parameters.goalkeeper.player_number
        && crate::goalkeeper::goalkeeper_shot_block_position(blackboard).is_some()
        && (own_goal_x..=own_goal_x + field_dimensions.goal_box_area_length)
            .contains(&target_in_field.x())
        && target_in_field.y().abs() <= field_dimensions.goal_inner_width / 2.0;
    for mut obstacle in blackboard.world_state.obstacles.iter().copied() {
        let position = ground_to_field * obstacle.position;
        let defending_own_post = keeper_defending
            && obstacle.kind == ObstacleKind::GoalPost
            && (position.x() - own_goal_x).abs() <= field_dimensions.goal_post_diameter
            && (position.y().abs()
                - (field_dimensions.goal_inner_width + field_dimensions.goal_post_diameter) / 2.0)
                .abs()
                <= field_dimensions.goal_post_diameter;
        let robot_radius = if defending_own_post {
            // Preserve body/post clearance without the maneuvering margin around other obstacles.
            obstacle.radius_at_hip_height = field_dimensions.goal_post_diameter / 2.0;
            parameters.robot_radius * 0.75
        } else {
            parameters.robot_radius
        };
        planner.with_obstacles(&[obstacle], robot_radius);
    }
    planner.with_rule_obstacles(
        ground_to_field.inverse(),
        &blackboard.world_state.rule_obstacles,
        parameters.robot_radius,
    );
    planner.with_field_borders(
        ground_to_field,
        field_dimensions.length,
        field_dimensions.width,
        field_dimensions.border_strip_width,
        parameters.field_border_weight,
    );
    planner.with_goal_support_structures(ground_to_field.inverse(), &field_dimensions);
    let ball_obstacle = blackboard.world_state.ball.map(|ball| ball.ball_in_ground);
    let ball_obstacle = if interception {
        ball_obstacle
            .or_else(|| {
                blackboard
                    .ball
                    .as_ref()
                    .filter(|ball| {
                        ball.age <= blackboard.world_state.now
                            && blackboard.world_state.now.duration_since(ball.age)
                                < ball_timeout(&blackboard.parameters, ball.source)
                    })
                    .map(|ball| ground_to_field.inverse() * ball.position)
            })
            .filter(|position| position.inner.iter().all(|value| value.is_finite()))
    } else {
        ball_obstacle
    };

    let x_max = field_dimensions.length / 2.0 + field_dimensions.border_strip_width;
    let y_max = field_dimensions.width / 2.0 + field_dimensions.border_strip_width;
    let mut clamped_target_in_robot = ground_to_field.inverse()
        * point![
            target_in_field.x().clamp(-x_max, x_max),
            target_in_field.y().clamp(-y_max, y_max)
        ];

    if avoid_ball && let Some(ball_position) = ball_obstacle {
        let ball_radius = if interception {
            parameters
                .ball_obstacle_radius
                .max(field_dimensions.ball_radius)
        } else {
            parameters.ball_obstacle_radius
        };
        planner.with_ball(ball_position, ball_radius, parameters.robot_radius);
        // Match the planner's target margin without shrinking the ball obstacle at contact.
        let clearance = 1.1 * (parameters.robot_radius + ball_radius);
        let offset = clamped_target_in_robot - ball_position;
        if interception && offset.norm() < clearance {
            let side = offset
                .try_normalize(f32::EPSILON)
                .or_else(|| (-ball_position.coords()).try_normalize(f32::EPSILON))
                .unwrap_or_else(|| linear_algebra::vector![1.0, 0.0]);
            clamped_target_in_robot = ball_position + side * clearance;
        }
    }

    let path = planner.plan(Point::origin(), clamped_target_in_robot);
    blackboard.path_obstacles_output = planner.obstacles;
    path.unwrap_or_else(|error| {
        tracing::warn!(%error, "Walking path planning failed");
        None
    })
    .filter(|path| {
        !interception
            || !avoid_ball
            || ball_obstacle.is_none_or(|ball| {
                // Overlapping obstacles can shrink the planner's ball circle at the target.
                path_keeps_ball_clearance(
                    path,
                    ball,
                    (parameters.robot_radius + field_dimensions.ball_radius)
                        .min(ball.coords().norm()),
                )
            })
    })
}

fn path_keeps_ball_clearance(path: &Path, ball: Point2<Ground>, clearance: f32) -> bool {
    path.segments.iter().all(|segment| {
        let distance = match segment {
            // Projecting the center of an arc would normalize a zero vector.
            PathSegment::Arc(arc) if arc.circle.center == ball => arc.circle.radius,
            _ => (segment.project(ball) - ball).norm(),
        };
        distance + 1e-5 >= clearance
    })
}

pub fn walk_to(
    blackboard: &mut Blackboard,
    target_pose: Pose2<Ground>,
    maximal_walk_speed: f32,
    orientation_mode: OrientationMode,
    distance_to_be_aligned: f32,
    hysteresis: nalgebra::Vector2<f32>,
) -> Status {
    if let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field {
        let parameters = &blackboard.parameters.walking.walk_and_stand;
        let distance_to_walk = target_pose.position().coords().norm();
        let angle_to_walk = target_pose.orientation().angle();
        let was_standing_last_cycle = matches!(
            blackboard.last_motion_command,
            Some(MotionCommand::Stand { .. })
        );
        let is_reached = less_than_with_relative_hysteresis(
            was_standing_last_cycle,
            distance_to_walk,
            parameters.target_reached_thresholds.x,
            0.0..=hysteresis.x,
        ) && less_than_with_relative_hysteresis(
            was_standing_last_cycle,
            angle_to_walk.abs(),
            parameters.target_reached_thresholds.y,
            0.0..=hysteresis.y,
        );

        let minimal_walk_speed = blackboard.parameters.walking.speed.minimum_speed;
        let velocity_fade_distance = blackboard.parameters.walking.speed.velocity_fade_distance;

        // Desmos: https://www.desmos.com/calculator/ss94dje2ke
        let walk_speed = maximal_walk_speed
            - (maximal_walk_speed - minimal_walk_speed)
                * (-(2.0 * distance_to_walk / velocity_fade_distance).powf(2.0)).exp();

        if is_reached {
            blackboard.body_motion = Some(BodyMotion::Stand);
            Status::Success
        } else {
            let Some(path) = plan(blackboard, target_pose.position(), ground_to_field) else {
                return Status::Failure;
            };
            blackboard.body_motion = Some(BodyMotion::Walk {
                path,
                orientation_mode,
                target_orientation: target_pose.orientation(),
                distance_to_be_aligned,
                speed: walk_speed,
            });
            Status::Success
        }
    } else {
        Status::Failure
    }
}

pub fn walk_to_ball(blackboard: &mut Blackboard) -> Status {
    if let (Some(ball), Some(ground_to_field)) = (
        &blackboard.last_ball,
        &blackboard.world_state.robot.ground_to_field,
    ) {
        let field_to_ground = ground_to_field.inverse();
        let ball_in_ground = field_to_ground * ball.position;
        let target = match blackboard.ball_action {
            Some(BallAction::Shoot { opening }) => opening.center(),
            _ => point!(blackboard.field_dimensions.length / 2.0, 0.0),
        };
        let goal_position = field_to_ground * target;
        let orientation = Orientation2::from_vector(goal_position - ball_in_ground);
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let kicking_speed = blackboard.parameters.walking.speed.kicking;

        let target_position = ball_in_ground
            - (goal_position - ball_in_ground).normalize()
                * blackboard.parameters.kicking.approach_ball_standoff;
        walk_to(
            blackboard,
            Pose2::from_parts(target_position, orientation),
            kicking_speed,
            OrientationMode::AlignWithPath,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.hysteresis,
        )
    } else {
        Status::Failure
    }
}

pub fn walk_to_ball_subtree() -> Node<Blackboard> {
    switch_motion_type(
        MotionType::Walk,
        action!(walk_to_ball),
        subtree!(walk_alternatives_subtree),
    )
}

pub fn walk_alternatives_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_last_motion_type, MotionType::Kick),
            sequence!(
                action!(kick),
                action!(select_fallback_kick_target),
                action!(use_last_kick_settings),
            )
        ),
        action!(stand)
    )
}

pub fn walk_to_block_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(block_position), Some(ball), Some(ground_to_field)) = (
        &blackboard.walk_position,
        &blackboard.last_ball,
        blackboard.world_state.robot.ground_to_field,
    ) {
        let ball_position = ground_to_field.inverse() * ball.position;
        let orientation = Orientation2::from_vector(ball_position - *block_position);
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let blocking_speed = blackboard.parameters.walking.speed.blocking;

        walk_to(
            blackboard,
            Pose2::from_parts(*block_position, orientation),
            blocking_speed,
            OrientationMode::LookAt {
                target: ball_position,
                tolerance: walk_and_stand.orientation_tolerance,
            },
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.goalkeeper_hysteresis,
        )
    } else {
        Status::Failure
    }
}

pub fn walk_to_kickoff_pose(blackboard: &mut Blackboard) -> Status {
    let Some(ground_to_field) = blackboard.world_state.robot.ground_to_field else {
        return Status::Failure;
    };

    let Some(game_controller_state) = blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
    else {
        return Status::Failure;
    };

    let player_number = blackboard.player_number;

    if game_controller_state.penalties[player_number].is_some() {
        return Status::Failure;
    }

    let kickoff_pose = crate::kick::kickoff::kickoff_pose(blackboard, player_number)
        .unwrap_or_else(|| {
            select_kickoff_pose(
                blackboard,
                player_number,
                game_controller_state,
                blackboard.parameters.goalkeeper.player_number,
                &blackboard.parameters.kickoff,
                &blackboard.field_dimensions,
            )
        });

    let walk_and_stand_parameters = blackboard.parameters.walking.walk_and_stand;

    walk_to(
        blackboard,
        ground_to_field.inverse() * kickoff_pose,
        blackboard.parameters.walking.speed.walk_to_kickoff,
        OrientationMode::AlignWithPath,
        walk_and_stand_parameters.normal_distance_to_be_aligned,
        walk_and_stand_parameters.hysteresis,
    )
}

fn select_kickoff_pose(
    blackboard: &Blackboard,
    player_number: PlayerNumber,
    game_controller_state: &FilteredGameControllerState,
    goalkeeper_player_number: PlayerNumber,
    kickoff_parameters: &KickoffParameters,
    field_dimensions: &FieldDimensions,
) -> Pose2<Field> {
    let field_center = point![0.0, 0.0];
    if player_number == goalkeeper_player_number {
        let relative_position = kickoff_parameters.relative_goalkeeper_position;
        let position = point![
            relative_position.x() * field_dimensions.length / 2.0,
            relative_position.y() * field_dimensions.width / 2.0
        ];
        let orientation = Orientation2::from_vector(field_center - position);
        return Pose2::from_parts(position, orientation);
    }

    let field_player_rank = game_controller_state
        .penalties
        .iter()
        .filter(|(number, penalty)| {
            *number > player_number && *number != goalkeeper_player_number && penalty.is_none()
        })
        .count();

    let relative_position = if game_controller_state.kicking_team == Some(Team::Hulks) {
        match field_player_rank {
            0 if has_more_than_unpenalized_players(blackboard, 2) => {
                kickoff_parameters.relative_striker_side_kick_position
            }
            0 => kickoff_parameters.relative_striker_position,
            rank => kickoff_parameters.relative_aggressive_positions[rank - 1],
        }
    } else {
        kickoff_parameters.relative_defensive_positions[field_player_rank]
    };

    let position = point![
        relative_position.x() * field_dimensions.length / 2.0,
        relative_position.y() * field_dimensions.width / 2.0
    ];
    let orientation = Orientation2::from_vector(field_center - position);
    Pose2::from_parts(position, orientation)
}

pub fn walk_to_voronoi_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(ground_to_field), Some(map)) = (
        blackboard.world_state.robot.ground_to_field,
        &blackboard.voronoi_map,
    ) && let Some(target_position) = target_player_position(
        map,
        blackboard.player_number,
        blackboard.ball.as_ref().map(|ball| ball.position),
        &blackboard.field_dimensions,
        &blackboard.parameters.voronoi,
    ) {
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let kicking_speed = blackboard.parameters.walking.speed.kicking;
        let orientation_mode = if let Some(ball) = &blackboard.ball {
            OrientationMode::LookAt {
                target: ground_to_field.inverse() * ball.position,
                tolerance: walk_and_stand.orientation_tolerance,
            }
        } else {
            OrientationMode::AlignWithPath
        };

        let target_orientation =
            blackboard
                .ball
                .as_ref()
                .map_or_else(Orientation2::default, |ball| {
                    Orientation2::from_vector(
                        ground_to_field.inverse() * (ball.position - target_position),
                    )
                });
        walk_to(
            blackboard,
            Pose2::from_parts(
                ground_to_field.inverse() * target_position,
                target_orientation,
            ),
            kicking_speed,
            orientation_mode,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.hysteresis,
        )
    } else {
        Status::Failure
    }
}

fn target_player_position(
    map: &VoronoiGrid,
    player: PlayerNumber,
    ball_position: Option<Point2<Field>>,
    field_dimensions: &FieldDimensions,
    parameters: &VoronoiParameters,
) -> Option<Point2<Field>> {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut count = 0;
    let mut candidates = Vec::new();

    for (point, ownership) in map.cells() {
        if ownership != Ownership::Robot(player)
            || point.x().abs() > field_dimensions.length / 2.0
            || point.y().abs() > field_dimensions.width / 2.0
        {
            continue;
        }

        candidates.push(point);

        sum_x += point.x();
        sum_y += point.y();
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let inv_count = 1.0 / count as f32;
    let centroid: Point2<Field> = point![sum_x * inv_count, sum_y * inv_count];

    let Some(ball_position) = ball_position else {
        return Some(centroid);
    };

    let half_length = field_dimensions.length / 2.0 + parameters.padding;
    let ball_x = ball_position.x();
    let ball_y = ball_position.y();
    let side_factor = (ball_x / half_length).clamp(-1.0, 1.0);

    let resolution = map.resolution();

    let support_distance = (field_dimensions.center_circle_diameter / 2.0).max(resolution);
    let support_sigma =
        (parameters.relative_ball_support_sigma * field_dimensions.width).max(resolution);
    let inv_two_support_sigma_sq = 1.0 / (2.0 * support_sigma * support_sigma);

    let centroid_sigma =
        (parameters.relative_centroid_anchor_sigma * field_dimensions.width).max(resolution);

    let mut best_target = None;

    for point in candidates {
        let forward_norm = point.x() / half_length;
        let forward_term = parameters.forward_weight * side_factor * forward_norm;

        let dx_ball = point.x() - ball_x;
        let dy_ball = point.y() - ball_y;
        let ball_distance = (dx_ball * dx_ball + dy_ball * dy_ball).sqrt();
        let support_distance_error = ball_distance - support_distance;

        let ball_term = if dx_ball <= 0.0 {
            parameters.ball_weight
                * (-(support_distance_error * support_distance_error) * inv_two_support_sigma_sq)
                    .exp()
        } else {
            0.0
        };

        let dx_centroid = point.x() - centroid.x();
        let dy_centroid = point.y() - centroid.y();
        let centroid_penalty = parameters.centroid_anchor_weight
            * (dx_centroid * dx_centroid + dy_centroid * dy_centroid).sqrt()
            / centroid_sigma;

        let score = forward_term + ball_term - centroid_penalty;
        if best_target.is_none_or(|(best_score, _)| score > best_score) {
            best_target = Some((score, point));
        }
    }

    best_target.map(|(_, point)| point)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use types::{field_dimensions::Side, world_state::BallSource};

    use crate::node::LastBall;

    #[test]
    fn blocking_steps_are_small_in_front_and_fast_for_offset_or_distant_balls() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ..Default::default()
        };
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.speed.minimum_speed = 0.2;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.walking_velocity_limits.forward_velocity_limits = [-0.8, 1.8];
        board.walking_velocity_limits.lateral_velocity_limit = 1.0;
        board.walking_velocity_limits.angular_velocity_limit = 2.0;
        let desired = linear_algebra::vector![2.0, 4.0];
        let close = adaptive_blocking_velocity(&board, desired, point![0.5, 0.1]);
        approx::assert_relative_eq!(close.norm(), 0.2);
        approx::assert_relative_eq!(close.x() / close.y(), 0.5);
        for ball in [
            point![3.0, 0.1],
            point![1.0, 0.1],
            point![0.5, 0.8],
            point![-0.5, 0.1],
        ] {
            let fast = adaptive_blocking_velocity(&board, desired, ball);
            approx::assert_relative_eq!(fast, linear_algebra::vector![0.5, 1.0]);
        }
        let backward = adaptive_blocking_velocity(
            &board,
            linear_algebra::vector![-2.0, 4.0],
            point![3.0, 0.1],
        );
        approx::assert_relative_eq!(backward, linear_algebra::vector![-0.5, 1.0]);
        assert_eq!(
            adaptive_blocking_velocity(&board, Vector2::zeros(), point![0.5, 0.0]),
            Vector2::zeros()
        );
        assert_eq!(
            adaptive_blocking_velocity(&board, desired, point![f32::NAN, 0.0]),
            Vector2::zeros()
        );

        for distance in [0.31, 0.3, 0.2] {
            let ball = point![distance, 0.0];
            let velocity =
                adaptive_blocking_velocity(&board, linear_algebra::vector![1.8, 0.0], ball);
            assert!(velocity.x() * 0.35 <= (distance - 0.3).max(0.0) + 1e-5);
            let retreat =
                adaptive_blocking_velocity(&board, linear_algebra::vector![-0.4, 0.0], ball);
            assert_eq!(retreat.x(), -0.2);
        }
        // Keep the small sideways correction that puts an addressed pass on a foot.
        let correction = adaptive_blocking_velocity(
            &board,
            linear_algebra::vector![0.0, 0.1],
            point![0.2, 0.17],
        );
        assert!(correction.y() > 0.0);
    }

    #[test]
    fn incoming_shot_keeps_post_clearance_without_reducing_other_margins() {
        use crate::node::LastBall;
        use types::{
            field_dimensions::Side, obstacles::Obstacle, path::traits::EndPoints,
            world_state::BallSource,
        };
        for side in [-1.0, 1.0] {
            for post_radius in [0.05, 0.2] {
                for mapped_offset in [0.0, 0.025] {
                    let dimensions = FieldDimensions {
                        length: 14.0,
                        width: 9.0,
                        goal_inner_width: 2.4,
                        ..FieldDimensions::SPL_2025
                    };
                    let transform =
                        Isometry2::from_parts(linear_algebra::vector![-6.7, side * 0.6], 0.0);
                    let mut board = Blackboard {
                        field_dimensions: dimensions,
                        ball: Some(LastBall {
                            position: point![-4.0, side * 0.9],
                            velocity: linear_algebra::vector![-3.0, side * 0.3],
                            age: Default::default(),
                            field_side: Side::Left,
                            source: BallSource::Own,
                        }),
                        ..Default::default()
                    };
                    board.world_state.robot.ground_to_field = Some(transform);
                    board.parameters.goalkeeper.player_number = PlayerNumber::One;
                    board.player_number = PlayerNumber::One;
                    board.parameters.goalkeeper.distance_to_goalpost = 0.15;
                    board.parameters.goalkeeper.x_offset = 0.3;
                    board
                        .parameters
                        .goalkeeper
                        .active_defense_maximum_robot_distance = 2.0;
                    board.parameters.walking.path_planning.robot_radius = 0.4;
                    let post = point![-7.0 - mapped_offset, side * 1.25];
                    board.world_state.obstacles = vec![
                        Obstacle::goal_post(transform.inverse() * post, post_radius),
                        Obstacle::goal_post(
                            transform.inverse() * point![7.0, side * 1.25],
                            post_radius,
                        ),
                        Obstacle::robot(point![2.0, 2.0], 0.2, 0.2),
                    ];
                    let target = transform.inverse()
                        * crate::goalkeeper::goalkeeper_shot_block_position(&board).unwrap();
                    let path = plan(&mut board, target, transform).unwrap();
                    let endpoint = transform * path.end_point();
                    assert!((endpoint - post).norm() >= 0.35 - 1e-5);
                    assert!((endpoint - transform * target).norm() < 0.15);
                    approx::assert_relative_eq!(
                        board.path_obstacles_output[1]
                            .shape
                            .as_circle()
                            .unwrap()
                            .radius,
                        post_radius + 0.4
                    );
                    approx::assert_relative_eq!(
                        board.path_obstacles_output[2]
                            .shape
                            .as_circle()
                            .unwrap()
                            .radius,
                        0.6
                    );

                    let _ = plan(
                        &mut board,
                        transform.inverse() * point![-5.0, 0.5],
                        transform,
                    );
                    approx::assert_relative_eq!(
                        board.path_obstacles_output[0]
                            .shape
                            .as_circle()
                            .unwrap()
                            .radius,
                        post_radius + 0.4,
                        epsilon = 1e-5
                    );

                    board.ball.as_mut().unwrap().velocity = Vector2::zeros();
                    let _ = plan(&mut board, target, transform);
                    approx::assert_relative_eq!(
                        board.path_obstacles_output[0]
                            .shape
                            .as_circle()
                            .unwrap()
                            .radius,
                        post_radius + 0.4
                    );

                    board.player_number = PlayerNumber::Two;
                    board.ball.as_mut().unwrap().velocity =
                        linear_algebra::vector![-3.0, side * 0.3];
                    let fielder_path = plan(&mut board, target, transform).unwrap();
                    assert!(
                        (transform * fielder_path.end_point() - post).norm()
                            >= post_radius + 0.4 - 1e-5
                    );
                    approx::assert_relative_eq!(
                        board.path_obstacles_output[0]
                            .shape
                            .as_circle()
                            .unwrap()
                            .radius,
                        post_radius + 0.4
                    );
                }
            }
        }
    }

    #[test]
    fn contested_interception_avoids_ball_and_holds_if_route_is_blocked() {
        let mut board = Blackboard {
            player_number: PlayerNumber::Two,
            field_dimensions: FieldDimensions::SPL_2025,
            ..Default::default()
        };
        board.world_state.robot.ground_to_field = Some(Isometry2::identity());
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.path_planning.ball_obstacle_radius = 0.1;
        board.parameters.kicking.strategy.pressure_distance = 0.8;
        board.ball = Some(LastBall {
            position: point![2.0, 0.0],
            velocity: linear_algebra::vector![-1.0, 0.0],
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        let target = point![3.5, 0.0];
        let ordinary_path = plan(&mut board, target, Isometry2::identity()).unwrap();
        assert!((ordinary_path.project(point![2.0, 0.0]) - point![2.0, 0.0]).norm() < 1e-5);
        board.world_state.obstacles = vec![types::obstacles::Obstacle::robot(
            point![2.2, 0.0],
            0.2,
            0.2,
        )];
        let path = plan_interception(&mut board, target, Isometry2::identity()).unwrap();
        assert!((path.project(point![2.0, 0.0]) - point![2.0, 0.0]).norm() >= 0.5 - 1e-5);
        board
            .world_state
            .rule_obstacles
            .push(types::rule_obstacles::RuleObstacle::Rectangle(
                geometry::rectangle::Rectangle {
                    min: point![3.0, -1.0],
                    max: point![4.0, 1.0],
                },
            ));
        let blocked = plan_interception(&mut board, target, Isometry2::identity());
        assert!(blocked.is_none());
    }
}
