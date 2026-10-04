use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, Orientation2, Point, Point2, Pose2, Vector2, point, vector};
use path_planner::path_planner::PathPlanner;
use types::{
    behavior_tree::Status,
    field_dimensions::FieldDimensions,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
    motion_type::MotionType,
    parameters::WalkToBallPredictionParameters,
    parameters::{KickOffPose, KickoffParameters, VoronoiParameters},
    path::{Path, direct_path},
    world_state::WorldState,
};
use voronoi::{Ownership, VoronoiGrid};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    kick::{kick, select_kick_target, use_last_kick_settings},
    node::Blackboard,
    selection, sequence, subtree,
    switch_motion_type::{is_last_motion_type, switch_motion_type},
};

pub fn plan(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Path {
    let parameters: &types::parameters::PathPlanningParameters =
        &blackboard.parameters.walking.path_planning;
    let field_dimensions = blackboard.field_dimensions;

    let mut planner = PathPlanner {
        obstacle_escape_spline_segments: parameters.obstacle_escape_spline_segments,
        ..Default::default()
    };
    planner.with_last_motion(
        &blackboard.last_motion_command,
        parameters.rotation_penalty_factor,
    );
    planner.with_obstacles(&blackboard.world_state.obstacles, parameters.robot_radius);
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

    if let Some(ball_position) = ball_obstacle {
        planner.with_ball(
            ball_position,
            parameters.ball_obstacle_radius,
            parameters.robot_radius,
        );
    }

    let target_in_field = ground_to_field * target_in_ground;
    let x_max = field_dimensions.length / 2.0 + field_dimensions.border_strip_width;
    let y_max = field_dimensions.width / 2.0 + field_dimensions.border_strip_width;
    let clamped_target_in_robot = ground_to_field.inverse()
        * point![
            target_in_field.x().clamp(-x_max, x_max),
            target_in_field.y().clamp(-y_max, y_max)
        ];

    let path = planner
        .plan(Point::origin(), clamped_target_in_robot)
        .unwrap();
    blackboard.path_obstacles_output = planner.obstacles;
    path.unwrap_or_else(|| direct_path(Point::origin(), target_in_ground))
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
        let was_standing_last_cycle =
            matches!(blackboard.last_motion_command, MotionCommand::Stand { .. });
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
            let path = plan(blackboard, target_pose.position(), ground_to_field);
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
        let ball_in_field = blackboard
            .world_state
            .ball
            .filter(|current_ball| {
                blackboard
                    .world_state
                    .now
                    .to_wallclock()
                    .duration_since(current_ball.last_seen_ball)
                    .is_ok_and(|age| age < blackboard.parameters.ball.last_ball_timeout)
            })
            .map(|current_ball| {
                predict_ball_position(
                    current_ball.ball_in_field,
                    *ground_to_field * current_ball.ball_in_ground_velocity,
                    blackboard.parameters.walking.ball_prediction,
                    blackboard.field_dimensions,
                )
            })
            .unwrap_or(ball.position);
        let ball_in_ground = field_to_ground * ball_in_field;
        let goal_position_in_field = point!(blackboard.field_dimensions.length / 2.0, 0.0);
        let direction_to_goal = field_to_ground
            * (goal_position_in_field - ball_in_field)
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(|| vector![1.0, 0.0]);
        let orientation = Orientation2::from_vector(direction_to_goal);
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let kicking_speed = blackboard.parameters.walking.speed.kicking;

        let target_position = ball_in_ground
            - direction_to_goal * blackboard.parameters.kicking.approach_ball_standoff;
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

fn predict_ball_position(
    position: Point2<Field>,
    velocity: Vector2<Field>,
    parameters: WalkToBallPredictionParameters,
    field_dimensions: FieldDimensions,
) -> Point2<Field> {
    if !velocity.x().is_finite()
        || !velocity.y().is_finite()
        || velocity.x() > parameters.maximum_forward_velocity_for_prediction
    {
        return position;
    }

    let time = parameters.time.as_secs_f32();
    let maximum_displacement = parameters.maximum_displacement;
    let projected = position
        + vector![
            (velocity.x() * time).clamp(-maximum_displacement.x(), maximum_displacement.x()),
            (velocity.y() * time).clamp(-maximum_displacement.y(), maximum_displacement.y())
        ];

    point![
        projected.x().clamp(
            -field_dimensions.length / 2.0,
            field_dimensions.length / 2.0
        ),
        projected
            .y()
            .clamp(-field_dimensions.width / 2.0, field_dimensions.width / 2.0)
    ]
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
                action!(select_kick_target),
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
    let Some(kickoff_pose) = select_kickoff_pose(
        &blackboard.world_state,
        blackboard.parameters.goalkeeper.player_number,
        &blackboard.parameters.kickoff,
    ) else {
        return Status::Failure;
    };

    let kickoff_pose_in_field = Pose2::from_parts(
        kickoff_pose.position,
        Orientation2::new(kickoff_pose.rotation),
    );
    let walk_and_stand = blackboard.parameters.walking.walk_and_stand;

    walk_to(
        blackboard,
        ground_to_field.inverse() * kickoff_pose_in_field,
        blackboard.parameters.walking.speed.walk_to_kickoff,
        OrientationMode::AlignWithPath,
        walk_and_stand.normal_distance_to_be_aligned,
        walk_and_stand.hysteresis,
    )
}

fn select_kickoff_pose(
    world_state: &WorldState,
    goalkeeper_player_number: PlayerNumber,
    parameters: &KickoffParameters,
) -> Option<KickOffPose> {
    let player_number = world_state.robot.player_number;
    let game_controller_state = world_state.filtered_game_controller_state.as_ref()?;
    if game_controller_state.penalties[player_number].is_some() {
        return None;
    }

    if player_number == goalkeeper_player_number {
        return Some(parameters.goalkeeper_pose);
    }

    let field_player_rank = game_controller_state
        .penalties
        .iter()
        .rev()
        .filter(|(number, penalty)| *number != goalkeeper_player_number && penalty.is_none())
        .position(|(number, _)| number == player_number)?;

    if game_controller_state.kicking_team == Some(Team::Hulks) {
        match field_player_rank {
            0 => Some(parameters.striker_pose),
            rank => parameters.aggressive_positions.get(rank - 1).copied(),
        }
    } else {
        parameters
            .defensive_positions
            .get(field_player_rank)
            .copied()
    }
}

pub fn walk_to_voronoi_position(blackboard: &mut Blackboard) -> Status {
    if let (Some(ground_to_field), Some(map)) = (
        blackboard.world_state.robot.ground_to_field,
        &blackboard.voronoi_map,
    ) && let Some(target_position) = target_player_position(
        map,
        blackboard.world_state.robot.player_number,
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

        walk_to(
            blackboard,
            Pose2::from(ground_to_field.inverse() * target_position),
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

    let support_distance = parameters.ball_support_distance.max(resolution);
    let support_sigma = parameters.ball_support_sigma.max(resolution);
    let inv_two_support_sigma_sq = 1.0 / (2.0 * support_sigma * support_sigma);

    let centroid_sigma = parameters.centroid_anchor_sigma.max(resolution);

    let mut best_target = None;

    for point in candidates {
        let forward_norm = point.x() / half_length;
        let forward_term = parameters.forward_weight * side_factor * forward_norm;

        let dx_ball = point.x() - ball_x;
        let dy_ball = point.y() - ball_y;
        let ball_distance = (dx_ball * dx_ball + dy_ball * dy_ball).sqrt();
        let support_distance_error = ball_distance - support_distance;
        let ball_term = parameters.ball_weight
            * (-(support_distance_error * support_distance_error) * inv_two_support_sigma_sq).exp();

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

    use linear_algebra::{point, vector};
    use types::{field_dimensions::FieldDimensions, parameters::WalkToBallPredictionParameters};

    use super::predict_ball_position;

    #[test]
    fn ball_prediction_limits_displacement_and_stays_inside_field() {
        let parameters = WalkToBallPredictionParameters {
            time: Duration::from_secs(1),
            maximum_displacement: vector![0.5, 0.25],
            maximum_forward_velocity_for_prediction: 0.2,
        };
        let field = FieldDimensions::SPL_2025;

        let projected =
            predict_ball_position(point![1.0, 1.0], vector![-2.0, -1.0], parameters, field);
        assert!((projected - point![0.5, 0.75]).norm() < 1e-6);

        let projected =
            predict_ball_position(point![4.4, 2.9], vector![0.1, 1.0], parameters, field);
        assert!((projected - point![4.5, 3.0]).norm() < 1e-6);
    }

    #[test]
    fn forward_ball_uses_current_position_on_both_axes() {
        let parameters = WalkToBallPredictionParameters {
            time: Duration::from_secs(1),
            maximum_displacement: vector![0.5, 0.25],
            maximum_forward_velocity_for_prediction: 0.2,
        };

        let projected = predict_ball_position(
            point![1.0, 1.0],
            vector![0.3, 1.0],
            parameters,
            FieldDimensions::SPL_2025,
        );
        assert!((projected - point![1.0, 1.0]).norm() < 1e-6);

        let projected = predict_ball_position(
            point![1.0, 1.0],
            vector![0.2, 0.0],
            parameters,
            FieldDimensions::SPL_2025,
        );
        assert!((projected - point![1.2, 1.0]).norm() < 1e-6);
    }
}
