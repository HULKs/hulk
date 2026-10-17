use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Orientation2, Point, Point2, Pose2, Vector2, point, vector};
use path_planner::path_planner::PathPlanner;
use types::{
    behavior_tree::Status,
    field_dimensions::FieldDimensions,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
    motion_type::MotionType,
    parameters::{VoronoiParameters, WalkToBallPredictionParameters},
    path::{Path, direct_path, traits::Length},
};
use voronoi::{Ownership, VoronoiGrid};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::hulks_is_kicking_team,
    kick::{kick, select_kick_target, use_last_kick_power},
    node::Blackboard,
    selection, sequence, subtree,
    switch_motion_type::{is_last_motion_type, switch_motion_type},
};

pub fn plan(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Path {
    try_plan(blackboard, target_in_ground, ground_to_field)
        .unwrap_or_else(|| direct_path(Point::origin(), target_in_ground))
}

fn try_plan(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Option<Path> {
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
    path
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
        let last_ball_position = ball.position;
        let ground_to_field = *ground_to_field;
        let prediction = blackboard.parameters.walking.ball_prediction;
        let field_dimensions = blackboard.field_dimensions;
        let standoff = blackboard.parameters.kicking.kick_position_ball_distance;
        let current_ball = blackboard.world_state.ball.filter(|current_ball| {
            blackboard
                .world_state
                .now
                .to_wallclock()
                .duration_since(current_ball.last_seen_ball)
                .is_ok_and(|age| age < blackboard.parameters.ball.last_ball_timeout)
        });
        let ball_in_field = if let Some(current_ball) = current_ball {
            let predicted = first_reachable_ball_position(
                current_ball.ball_in_field,
                ground_to_field * current_ball.ball_in_ground_velocity,
                ground_to_field,
                field_dimensions,
                standoff,
                prediction,
                |target| {
                    try_plan(blackboard, target, ground_to_field)
                        .map_or(f32::INFINITY, |path| path.length())
                },
            );
            if let Some(predicted) = predicted {
                smooth_ball_target(blackboard, predicted)
            } else {
                blackboard.last_walk_to_ball_target = None;
                current_ball.ball_in_field
            }
        } else {
            blackboard.last_walk_to_ball_target = None;
            last_ball_position
        };
        let target_pose =
            kick_approach_pose(ball_in_field, ground_to_field, field_dimensions, standoff);
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let kicking_speed = blackboard.parameters.walking.speed.kicking;

        walk_to(
            blackboard,
            target_pose,
            kicking_speed,
            OrientationMode::AlignWithPath,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.hysteresis,
        )
    } else {
        blackboard.last_walk_to_ball_target = None;
        Status::Failure
    }
}

fn kick_approach_pose(
    ball_in_field: Point2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
    field_dimensions: FieldDimensions,
    standoff: f32,
) -> Pose2<Ground> {
    let field_to_ground = ground_to_field.inverse();
    let goal_position = point!(field_dimensions.length / 2.0, 0.0);
    let direction_to_goal = field_to_ground
        * (goal_position - ball_in_field)
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(|| vector![1.0, 0.0]);
    let ball_in_ground = field_to_ground * ball_in_field;
    Pose2::from_parts(
        ball_in_ground - direction_to_goal * standoff,
        Orientation2::from_vector(direction_to_goal),
    )
}

fn first_reachable_ball_position(
    position: Point2<Field>,
    velocity: Vector2<Field>,
    ground_to_field: Isometry2<Ground, Field>,
    field_dimensions: FieldDimensions,
    standoff: f32,
    parameters: WalkToBallPredictionParameters,
    mut path_length: impl FnMut(Point2<Ground>) -> f32,
) -> Option<Point2<Field>> {
    if !velocity.x().is_finite() || !velocity.y().is_finite() {
        return None;
    }

    let horizon = parameters.horizon.as_secs_f32();
    let margin = parameters.arrival_time_margin.as_secs_f32();
    for sample in 1..=parameters.samples {
        let time = horizon * sample as f32 / parameters.samples as f32;
        if time <= margin {
            continue;
        }

        let predicted = predict_ball_position(
            position,
            velocity,
            time,
            parameters.velocity_decay_rate,
            field_dimensions,
        );
        let approach_pose =
            kick_approach_pose(predicted, ground_to_field, field_dimensions, standoff);
        let target = approach_pose.position();
        let turning_time = approach_pose.orientation().angle().abs() / parameters.turning_speed;
        let direct_time = target.coords().norm() / parameters.walking_speed;
        if direct_time.max(turning_time) + margin > time {
            continue;
        }
        let walking_time = path_length(target) / parameters.walking_speed;
        if walking_time.max(turning_time) + margin <= time {
            return Some(predicted);
        }
    }

    None
}

fn predict_ball_position(
    position: Point2<Field>,
    velocity: Vector2<Field>,
    time: f32,
    velocity_decay_rate: f32,
    field_dimensions: FieldDimensions,
) -> Point2<Field> {
    let travel_time = if velocity_decay_rate > 0.0 {
        -(-velocity_decay_rate * time).exp_m1() / velocity_decay_rate
    } else {
        time
    };
    let projected = position + velocity * travel_time;

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

fn smooth_ball_target(blackboard: &mut Blackboard, target: Point2<Field>) -> Point2<Field> {
    let now = blackboard.world_state.now;
    let smoothing_time = blackboard.parameters.walking.ball_prediction.smoothing_time;
    let previous = blackboard.last_walk_to_ball_target;
    let last_time = blackboard.last_walk_to_ball_target_time;
    let smoothed = if let Some(previous) = previous
        && !smoothing_time.is_zero()
        && blackboard.last_motion_type == Some(MotionType::Walk)
        && now >= last_time
        && now.duration_since(last_time) <= smoothing_time * 2
        && (target - previous).norm() <= 0.5
    {
        let elapsed = now.duration_since(last_time).as_secs_f32();
        let alpha = 1.0 - (-elapsed / smoothing_time.as_secs_f32()).exp();
        previous + (target - previous) * alpha
    } else {
        target
    };
    blackboard.last_walk_to_ball_target = Some(smoothed);
    blackboard.last_walk_to_ball_target_time = now;
    smoothed
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
                action!(use_last_kick_power),
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
    if let (Some(ground_to_field), player_number) = (
        blackboard.world_state.robot.ground_to_field,
        blackboard.world_state.robot.player_number,
    ) {
        let field_to_ground = ground_to_field.inverse();
        let kickoff = &blackboard.parameters.kickoff;
        let standard_pose = kickoff.standard_positions[player_number];
        let striker_position = kickoff.striker_position;
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let walk_to_kickoff_speed = blackboard.parameters.walking.speed.walk_to_kickoff;

        let mut target_position = standard_pose.position;

        if hulks_is_kicking_team(blackboard) && player_number == PlayerNumber::Three {
            target_position = striker_position;
        }

        let kickoff_pose_in_field =
            Pose2::from_parts(target_position, Orientation2::new(standard_pose.rotation));

        let kickoff_pose_in_ground = field_to_ground * kickoff_pose_in_field;

        walk_to(
            blackboard,
            kickoff_pose_in_ground,
            walk_to_kickoff_speed,
            OrientationMode::AlignWithPath,
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.hysteresis,
        );
        Status::Success
    } else {
        Status::Failure
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

    use coordinate_systems::{Field, Ground};
    use linear_algebra::{Isometry2, point, vector};
    use types::{field_dimensions::FieldDimensions, parameters::WalkToBallPredictionParameters};

    use super::{first_reachable_ball_position, predict_ball_position};

    fn parameters() -> WalkToBallPredictionParameters {
        WalkToBallPredictionParameters {
            horizon: Duration::from_secs(4),
            samples: 20,
            walking_speed: 0.7,
            turning_speed: 2.0,
            arrival_time_margin: Duration::from_millis(200),
            velocity_decay_rate: 1.0,
            smoothing_time: Duration::from_millis(150),
        }
    }

    #[test]
    fn ball_prediction_decays_and_stays_inside_field() {
        let projected = predict_ball_position(
            point![0.0, 0.0],
            vector![1.0, 0.0],
            2.0,
            1.0,
            FieldDimensions::SPL_2025,
        );
        assert!((projected.x() - (1.0 - (-2.0_f32).exp())).abs() < 1e-6);

        let projected = predict_ball_position(
            point![4.4, 2.9],
            vector![1.0, 1.0],
            2.0,
            1.0,
            FieldDimensions::SPL_2025,
        );
        assert!((projected - point![4.5, 3.0]).norm() < 1e-6);
    }

    #[test]
    fn chooses_reachable_future_kick_position_for_crossing_ball() {
        let predicted = first_reachable_ball_position(
            point![2.0, 1.0],
            vector![0.0, -1.0],
            Isometry2::<Ground, Field>::identity(),
            FieldDimensions::SPL_2025,
            0.3,
            parameters(),
            |target| target.coords().norm(),
        )
        .unwrap();

        assert!(predicted.y() > 0.0 && predicted.y() < 0.1);
    }

    #[test]
    fn rejects_unreachable_ball_and_accounts_for_path_detours() {
        let identity = Isometry2::<Ground, Field>::identity();
        let field = FieldDimensions::SPL_2025;
        let mut prediction = parameters();
        prediction.velocity_decay_rate = 0.0;

        let receding = first_reachable_ball_position(
            point![1.0, 0.0],
            vector![2.0, 0.0],
            identity,
            field,
            0.3,
            prediction,
            |target| target.coords().norm(),
        );
        assert!(receding.is_none());

        prediction.horizon = Duration::from_secs(2);
        let direct = first_reachable_ball_position(
            point![1.0, 0.0],
            vector![0.0, 0.0],
            identity,
            field,
            0.3,
            prediction,
            |target| target.coords().norm(),
        );
        let detour = first_reachable_ball_position(
            point![1.0, 0.0],
            vector![0.0, 0.0],
            identity,
            field,
            0.3,
            prediction,
            |target| target.coords().norm() + 1.0,
        );
        assert!(direct.is_some());
        assert!(detour.is_none());
    }

    #[test]
    fn accounts_for_turning_before_reaching_kick_pose() {
        let ground_to_field =
            Isometry2::<Ground, Field>::from_parts(vector![0.0, 0.0], std::f32::consts::PI);
        let mut prediction = parameters();
        prediction.horizon = Duration::from_millis(1500);

        let slow_turn = first_reachable_ball_position(
            point![1.0, 0.0],
            vector![0.0, 0.0],
            ground_to_field,
            FieldDimensions::SPL_2025,
            0.3,
            prediction,
            |target| target.coords().norm(),
        );
        prediction.turning_speed = 10.0;
        let fast_turn = first_reachable_ball_position(
            point![1.0, 0.0],
            vector![0.0, 0.0],
            ground_to_field,
            FieldDimensions::SPL_2025,
            0.3,
            prediction,
            |target| target.coords().norm(),
        );

        assert!(slow_turn.is_none());
        assert!(fast_turn.is_some());
    }
}
