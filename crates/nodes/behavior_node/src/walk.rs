use coordinate_systems::{Field, Ground};
use filtering::hysteresis::less_than_with_relative_hysteresis;
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, Orientation2, Point, Point2, Pose2, Vector2, point};
use path_planner::path_planner::PathPlanner;
use types::{
    behavior_tree::Status,
    field_dimensions::{FieldDimensions, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    motion_command::{BodyMotion, MotionCommand, OrientationMode},
    motion_type::MotionType,
    obstacles::ObstacleKind,
    parameters::{KickoffParameters, VoronoiParameters},
    path::{Path, PathSegment, direct_path, traits::Project},
};
use voronoi::{Ownership, VoronoiGrid};

use crate::{
    action,
    actions::stand,
    behavior_tree::Node,
    condition,
    conditions::{
        enemy_obstacles, goalkeeper_clearance_direction, hulks_is_kicking_team, opponent_near_ball,
        opponent_shot_direction,
    },
    kick::{
        execution::execute_ball_action,
        kick_ball_in_ground, kick_ball_position_in_ground,
        kickoff::{kickoff_receiver, kickoff_receiver_position, kickoff_support_position},
    },
    node::{Blackboard, ball_timeout},
    selection, sequence, subtree,
    switch_motion_type::{is_allowed_to_switch, is_last_motion_type, switch_motion_type},
    voronoi::collect_sites,
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
) -> Path {
    plan_with_ball_obstacle(blackboard, target_in_ground, ground_to_field, true)
}

pub(crate) fn plan_interception(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
) -> Path {
    let contested = opponent_near_ball(
        blackboard,
        blackboard.parameters.kicking.strategy.pressure_distance,
    );
    plan_with_ball_obstacle(blackboard, target_in_ground, ground_to_field, contested)
}

fn plan_with_ball_obstacle(
    blackboard: &mut Blackboard,
    target_in_ground: Point2<Ground>,
    ground_to_field: Isometry2<Ground, Field>,
    avoid_ball: bool,
) -> Path {
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
    let ball_obstacle = blackboard
        .world_state
        .ball
        .map(|ball| ball.ball_in_ground)
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
        .filter(|position| position.inner.iter().all(|value| value.is_finite()));

    let x_max = field_dimensions.length / 2.0 + field_dimensions.border_strip_width;
    let y_max = field_dimensions.width / 2.0 + field_dimensions.border_strip_width;
    let mut clamped_target_in_robot = ground_to_field.inverse()
        * point![
            target_in_field.x().clamp(-x_max, x_max),
            target_in_field.y().clamp(-y_max, y_max)
        ];

    if avoid_ball && let Some(ball_position) = ball_obstacle {
        let ball_radius = parameters
            .ball_obstacle_radius
            .max(field_dimensions.ball_radius);
        planner.with_ball(ball_position, ball_radius, parameters.robot_radius);
        // Match the planner's target margin without shrinking the ball obstacle at contact.
        let clearance = 1.1 * (parameters.robot_radius + ball_radius);
        let offset = clamped_target_in_robot - ball_position;
        if offset.norm() < clearance {
            let side = offset
                .try_normalize(f32::EPSILON)
                .or_else(|| (-ball_position.coords()).try_normalize(f32::EPSILON))
                .unwrap_or_else(|| linear_algebra::vector![1.0, 0.0]);
            clamped_target_in_robot = ball_position + side * clearance;
        }
    }

    let path = planner
        .plan(Point::origin(), clamped_target_in_robot)
        .unwrap();
    blackboard.path_obstacles_output = planner.obstacles;
    path.filter(|path| {
        !avoid_ball
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
    .unwrap_or_else(|| {
        // A failed avoidance plan must not fall back to walking through the ball.
        direct_path(
            Point::origin(),
            if avoid_ball {
                Point::origin()
            } else {
                target_in_ground
            },
        )
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
        let observed_ball = kick_ball_position_in_ground(blackboard, ground_to_field)
            .unwrap_or(field_to_ground * ball.position);
        let ball_in_ground =
            field_to_ground * blackboard.projected_ball_position(ball.position, ball.velocity);
        let goal_position = field_to_ground * point!(blackboard.field_dimensions.length / 2.0, 0.0);
        let orientation = Orientation2::from_vector(observed_ball.coords());
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let kicking_speed = blackboard.parameters.walking.speed.kicking;

        let target_position = ball_in_ground
            - (goal_position - ball_in_ground).normalize()
                * blackboard.parameters.kicking.approach_ball_standoff;
        walk_to(
            blackboard,
            Pose2::from_parts(target_position, orientation),
            kicking_speed,
            OrientationMode::LookAt {
                target: observed_ball,
                tolerance: walk_and_stand.orientation_tolerance,
            },
            walk_and_stand.normal_distance_to_be_aligned,
            walk_and_stand.hysteresis,
        )
    } else {
        Status::Failure
    }
}

pub(crate) fn recover_goal_side(blackboard: &mut Blackboard) -> Status {
    if blackboard.player_number == blackboard.parameters.goalkeeper.player_number
        || blackboard.last_motion_type == Some(MotionType::Kick)
        || (blackboard.last_motion_type != Some(MotionType::Walk)
            && !is_allowed_to_switch(blackboard))
        || goalkeeper_clearance_direction(blackboard).is_some()
        || crate::kick::passing::is_pass_receiver(blackboard)
        || crate::kick::passing::outgoing_pass_in_flight(blackboard)
    {
        blackboard.ball_recovery_side = None;
        blackboard.ball_recovery_lane_cleared = false;
        return Status::Failure;
    }
    let sample = blackboard
        .ball
        .as_ref()
        .filter(|ball| {
            ball.age <= blackboard.world_state.now
                && blackboard.world_state.now.duration_since(ball.age)
                    < ball_timeout(&blackboard.parameters, ball.source)
        })
        .and_then(|_| {
            let transform = blackboard.world_state.robot.ground_to_field?;
            let (ball, velocity) = kick_ball_in_ground(blackboard, &transform)?;
            Some((transform, ball, velocity))
        });
    let Some((transform, ball, velocity)) = sample else {
        blackboard.ball_recovery_side = None;
        blackboard.ball_recovery_lane_cleared = false;
        return Status::Failure;
    };
    let position = transform.as_pose().position();
    let ball_in_field = transform * ball;
    let velocity_in_field = transform * velocity;
    let parameters = &blackboard.parameters;
    let walking = parameters.walking.walk_and_stand;
    let speed = parameters.walking.speed.kicking;
    let field = blackboard.field_dimensions;
    let robot_radius = parameters.walking.path_planning.robot_radius;
    let standoff = parameters.kicking.approach_ball_standoff;
    let clearance = 1.1
        * (robot_radius
            + field
                .ball_radius
                .max(parameters.walking.path_planning.ball_obstacle_radius))
        + standoff;
    if !clearance.is_finite()
        || clearance <= 0.0
        || !field.length.is_finite()
        || !field.width.is_finite()
        || field.length <= 2.0 * clearance
        || field.width <= 2.0 * clearance
        || (velocity.norm() > parameters.ball.interception.minimum_ball_velocity
            && velocity_in_field.x() >= -parameters.ball.interception.minimum_ball_velocity)
        || position.x() <= ball_in_field.x() - robot_radius - field.ball_radius
        || (blackboard.ball_recovery_side.is_none()
            && position.x() <= ball_in_field.x() + field.ball_radius)
    {
        blackboard.ball_recovery_side = None;
        blackboard.ball_recovery_lane_cleared = false;
        return Status::Failure;
    }
    let predicted = blackboard.ball_pursuit_position(ball_in_field, velocity_in_field);
    let side = blackboard.ball_recovery_side.unwrap_or_else(|| {
        if (position.y() - predicted.y()).abs() > standoff {
            if position.y() > predicted.y() {
                Side::Left
            } else {
                Side::Right
            }
        } else if predicted.y() > 0.0 {
            Side::Right
        } else {
            Side::Left
        }
    });
    let mut lateral = match side {
        Side::Left => clearance,
        Side::Right => -clearance,
    };
    if (predicted.y() + lateral).abs() > field.width / 2.0 - robot_radius {
        lateral = -lateral;
    }
    let side = if lateral > 0.0 {
        Side::Left
    } else {
        Side::Right
    };
    let lateral_distance = (position.y() - predicted.y()) * lateral.signum();
    let ball_clearance = robot_radius
        + field
            .ball_radius
            .max(parameters.walking.path_planning.ball_obstacle_radius);
    if blackboard.ball_recovery_side != Some(side) || lateral_distance < ball_clearance {
        blackboard.ball_recovery_lane_cleared = false;
    }
    if lateral_distance >= ball_clearance + standoff / 2.0 {
        blackboard.ball_recovery_lane_cleared = true;
    }
    blackboard.ball_recovery_side = Some(side);
    let beside_ball = blackboard.ball_recovery_lane_cleared;
    // Leave the ball's lane before walking past it; keep facing the observed ball throughout.
    let target = point![
        (if beside_ball {
            predicted.x() - clearance
        } else {
            position.x().max(ball_in_field.x() + clearance)
        })
        .clamp(
            -field.length / 2.0 + robot_radius,
            field.length / 2.0 - robot_radius
        ),
        predicted.y() + lateral
    ];
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    let target_in_ground = transform.inverse() * target;
    let path = plan(blackboard, target_in_ground, transform);
    blackboard.body_motion = Some(BodyMotion::Walk {
        path,
        orientation_mode: OrientationMode::LookAt {
            target: ball,
            tolerance: walking.orientation_tolerance,
        },
        target_orientation: Orientation2::from_vector(ball.coords()),
        distance_to_be_aligned: walking.normal_distance_to_be_aligned,
        speed: speed.min(target_in_ground.coords().norm() / 0.35),
    });
    Status::Success
}

pub fn block_opponent_with_ball(blackboard: &mut Blackboard) -> Status {
    if blackboard.player_number == blackboard.parameters.goalkeeper.player_number
        || blackboard.last_motion_type == Some(MotionType::Kick)
        || (blackboard.last_motion_type != Some(MotionType::Walk)
            && !is_allowed_to_switch(blackboard))
        || crate::kick::passing::is_pass_receiver(blackboard)
        || goalkeeper_clearance_direction(blackboard).is_some()
        || !opponent_near_ball(
            blackboard,
            blackboard.parameters.kicking.strategy.pressure_distance,
        )
    {
        return Status::Failure;
    }
    let (Some(ball), Some(transform)) = (
        blackboard.ball.as_ref(),
        blackboard.world_state.robot.ground_to_field,
    ) else {
        return Status::Failure;
    };
    let Some(toward_goal) = opponent_shot_direction(blackboard).or_else(|| {
        (point![-blackboard.field_dimensions.length / 2.0, 0.0] - ball.position)
            .try_normalize(f32::EPSILON)
    }) else {
        return Status::Failure;
    };
    let target = transform.inverse()
        * (ball.position + toward_goal * blackboard.parameters.kicking.approach_ball_standoff);
    let ball = transform.inverse() * ball.position;
    if !ball
        .inner
        .iter()
        .chain(target.inner.iter())
        .all(|value| value.is_finite())
    {
        return Status::Failure;
    }
    // Get goal-side within tackle range so the next touch can face away from our goal.
    let walking = blackboard.parameters.walking.walk_and_stand;
    blackboard.ball_action = None;
    blackboard.kick_target = None;
    if ball.x() < 0.0 {
        blackboard.body_motion = Some(BodyMotion::WalkWithVelocity {
            velocity: Vector2::zeros(),
            angular_velocity: blackboard
                .walking_velocity_limits
                .scale_angular_axis(Orientation2::from_vector(ball.coords()).angle()),
        });
        return Status::Success;
    }
    let path = plan(blackboard, target, transform);
    blackboard.body_motion = Some(BodyMotion::Walk {
        path,
        orientation_mode: OrientationMode::LookAt {
            target: ball,
            tolerance: walking.orientation_tolerance,
        },
        target_orientation: Orientation2::from_vector(ball.coords()),
        distance_to_be_aligned: walking.normal_distance_to_be_aligned,
        speed: blackboard.parameters.walking.speed.kicking,
    });
    Status::Success
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
            action!(execute_ball_action)
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
        let orientation = Orientation2::from_vector(ball_position.coords());
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
    if blackboard
        .world_state
        .filtered_game_controller_state
        .as_ref()
        .is_some_and(|game| game.penalties[blackboard.player_number].is_some())
    {
        return Status::Failure;
    }
    if let (Some(ground_to_field), player_number) = (
        blackboard.world_state.robot.ground_to_field,
        blackboard.player_number,
    ) {
        let field_to_ground = ground_to_field.inverse();
        let own_kickoff = hulks_is_kicking_team(blackboard);
        let kicker = crate::kick::kickoff::kickoff_kicker(blackboard);
        let own_field_player =
            own_kickoff && player_number != blackboard.parameters.goalkeeper.player_number;
        let receiving_pose = crate::kick::kickoff::kickoff_receiving_pose(blackboard)
            .filter(|_| Some(player_number) == kickoff_receiver(blackboard));
        let kickoff = &blackboard.parameters.kickoff;
        let standard_player = if own_field_player {
            match player_number {
                PlayerNumber::Three => PlayerNumber::Two,
                PlayerNumber::Four => PlayerNumber::Five,
                PlayerNumber::Five => PlayerNumber::Four,
                other => other,
            }
        } else {
            player_number
        };
        let standard_pose = kickoff.standard_positions[standard_player];
        let striker_position = kickoff.striker_position;
        let walk_and_stand = blackboard.parameters.walking.walk_and_stand;
        let walk_to_kickoff_speed = blackboard.parameters.walking.speed.walk_to_kickoff;

        let mut target_position = standard_pose.position;
        if own_field_player && player_number == PlayerNumber::Two {
            target_position = point![target_position.x(), target_position.y().abs()];
        }
        if own_field_player && matches!(player_number, PlayerNumber::Four | PlayerNumber::Five) {
            target_position = point![
                target_position.x(),
                target_position.y().signum()
                    * target_position
                        .y()
                        .abs()
                        .max(kickoff_receiver_position(blackboard).y().abs())
            ];
        }

        let mut target_orientation = Orientation2::new(standard_pose.rotation);
        if !own_field_player
            && let Some(game) = &blackboard.world_state.filtered_game_controller_state
        {
            let pose = select_kickoff_pose(
                player_number,
                game,
                blackboard.parameters.goalkeeper.player_number,
                kickoff,
                &blackboard.field_dimensions,
            );
            target_position = pose.position();
            target_orientation = pose.orientation();
        }
        if own_field_player {
            if player_number == kicker {
                target_position = striker_position;
            } else if Some(player_number) == kickoff_receiver(blackboard) {
                if let Some(pose) = receiving_pose {
                    target_position = pose.position();
                    target_orientation = pose.orientation();
                } else {
                    target_position = kickoff_receiver_position(blackboard);
                    // Receive with an open stance so the next shot needs only a small turn.
                    target_orientation = Orientation2::from_vector(
                        (-target_position.coords()).normalize()
                            + (point![blackboard.field_dimensions.length / 2.0, 0.0]
                                - target_position)
                                .normalize(),
                    );
                }
            }
        }
        if player_number != blackboard.parameters.goalkeeper.player_number
            && !(own_kickoff && player_number == kicker)
            && receiving_pose.is_none()
        {
            target_position = kickoff_support_position(blackboard, target_position);
        }

        let kickoff_pose_in_field = Pose2::from_parts(target_position, target_orientation);

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

fn select_kickoff_pose(
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
        field_player_positioning(blackboard, map).unwrap_or_default(),
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum PlayerPositioning {
    #[default]
    Normal,
    ForwardSupport {
        anchor: Point2<Field>,
        minimum_distance: f32,
    },
    Lane(Point2<Field>),
}

fn field_player_positioning(
    blackboard: &Blackboard,
    map: &VoronoiGrid,
) -> Option<PlayerPositioning> {
    let world = &blackboard.world_state;
    let ball = blackboard.ball.as_ref()?;
    if blackboard.kickoff_started.is_some()
        || world
            .filtered_game_controller_state
            .as_ref()
            .is_some_and(|game| game.sub_state.is_some())
        || ball.age > world.now
        || world.now.duration_since(ball.age) >= ball_timeout(&blackboard.parameters, ball.source)
        || !ball.position.inner.iter().all(|value| value.is_finite())
    {
        return None;
    }
    let projected = blackboard.ball_pursuit_position(ball.position, ball.velocity);
    let striker = match map.ownership_at(projected) {
        Some(Ownership::Robot(number)) => number,
        Some(Ownership::Blocked) => match map.nearest_non_blocked_ownership(projected)? {
            Ownership::Robot(number) => number,
            _ => return None,
        },
        _ => return None,
    };
    let fielders: Vec<_> = collect_sites(blackboard, world.robot.ground_to_field?.as_pose())
        .into_iter()
        .filter(|(_, number)| {
            *number != blackboard.parameters.goalkeeper.player_number
                && map
                    .cells()
                    .any(|(_, ownership)| ownership == Ownership::Robot(*number))
        })
        .collect();
    if fielders.len() < 3 {
        return None;
    }
    let back = fielders
        .iter()
        .filter(|(_, number)| *number != striker)
        .min_by(|(left, left_number), (right, right_number)| {
            left.position()
                .x()
                .total_cmp(&right.position().x())
                .then_with(|| left_number.cmp(right_number))
        })
        .map(|(_, number)| *number)?;
    let field = blackboard.field_dimensions;
    let half_length = field.length / 2.0;
    let passing = &blackboard.parameters.kicking.strategy.passing;
    let minimum_distance = passing.minimum_distance + passing.receiver_standoff;
    let spacing = passing
        .minimum_distance
        .max(field.center_circle_diameter / 2.0)
        + passing.receiver_standoff
        + map.resolution();
    let margin = crate::kick::geometry::pass_reception_margin(
        &field,
        passing,
        blackboard.parameters.kicking.strategy.obstacle_clearance,
    );
    let lateral_spacing = spacing * (3.0_f32).sqrt() / 2.0;
    let support_anchor = |pose: Pose2<Field>| {
        let mut side = if pose.position().y() < ball.position.y() {
            -1.0
        } else {
            1.0
        };
        if (ball.position.y() + side * lateral_spacing).abs() > field.width / 2.0 - margin {
            side = -side;
        }
        point![
            (ball.position.x() + spacing / 2.0).clamp(-half_length + margin, half_length - margin),
            (ball.position.y() + side * lateral_spacing)
                .clamp(-field.width / 2.0 + margin, field.width / 2.0 - margin)
        ]
    };
    let supporter = fielders
        .iter()
        .filter(|(_, number)| *number != striker && *number != back)
        .min_by(|(left_pose, left_number), (right_pose, right_number)| {
            (left_pose.position() - support_anchor(*left_pose))
                .norm_squared()
                .total_cmp(&(right_pose.position() - support_anchor(*right_pose)).norm_squared())
                .then_with(|| left_number.cmp(right_number))
        })
        .copied()?;
    let support_anchor = support_anchor(supporter.0);
    let player = blackboard.player_number;
    if player == supporter.1 {
        return Some(PlayerPositioning::ForwardSupport {
            anchor: support_anchor,
            minimum_distance,
        });
    }
    if player == striker {
        return None;
    }
    let anchor = if player == back {
        let x =
            ((ball.position.x() - half_length) / 2.0).max(-half_length + field.penalty_area_length);
        let goal_to_ball = (x + half_length) / (ball.position.x() + half_length).max(f32::EPSILON);
        point![x, ball.position.y() * goal_to_ball.clamp(0.0, 1.0)]
    } else if let Some(screen) = opponent_pass_screen(blackboard, map, ball.position) {
        screen
    } else {
        let forward_spacing = blackboard
            .parameters
            .kicking
            .strategy
            .passing
            .minimum_distance;
        point![
            (ball.position.x() + forward_spacing / 2.0)
                .clamp(0.0, half_length - field.goal_box_area_length),
            -(support_anchor.y() - ball.position.y()).signum() * field.width / 4.0
        ]
    };
    Some(PlayerPositioning::Lane(anchor))
}

fn opponent_pass_screen(
    blackboard: &Blackboard,
    map: &VoronoiGrid,
    ball: Point2<Field>,
) -> Option<Point2<Field>> {
    let transform = blackboard.world_state.robot.ground_to_field?;
    let field = blackboard.field_dimensions;
    let pressure_distance = blackboard.parameters.kicking.strategy.pressure_distance;
    let opponents = || {
        enemy_obstacles(blackboard)
            .map(move |obstacle| {
                (
                    transform * obstacle.position,
                    obstacle.radius_at_foot_height,
                )
            })
            .filter(move |(position, _)| {
                position.inner.iter().all(|value| value.is_finite())
                    && position.x().abs() <= field.length / 2.0
                    && position.y().abs() <= field.width / 2.0
            })
    };
    let (holder, _) = opponents()
        .filter(|(position, radius)| (*position - ball).norm() - *radius <= pressure_distance)
        .min_by(|(left, _), (right, _)| {
            (*left - ball)
                .norm_squared()
                .total_cmp(&(*right - ball).norm_squared())
                .then_with(|| left.x().total_cmp(&right.x()))
                .then_with(|| left.y().total_cmp(&right.y()))
        })?;
    let goal = point![-field.length / 2.0, 0.0];
    let (receiver, _) = opponents()
        .filter(|(position, _)| {
            (*position - holder).norm() > 2.0 * pressure_distance
                && (*position - goal).norm_squared() < (holder - goal).norm_squared()
                && map.ownership_at(holder + (*position - holder) * 0.5)
                    == Some(Ownership::Robot(blackboard.player_number))
        })
        .min_by(|(left, _), (right, _)| {
            (*left - goal)
                .norm_squared()
                .total_cmp(&(*right - goal).norm_squared())
                .then_with(|| left.x().total_cmp(&right.x()))
                .then_with(|| left.y().total_cmp(&right.y()))
        })?;
    Some(holder + (receiver - holder) * 0.5)
}

fn target_player_position(
    map: &VoronoiGrid,
    player: PlayerNumber,
    ball_position: Option<Point2<Field>>,
    field_dimensions: &FieldDimensions,
    parameters: &VoronoiParameters,
    positioning: PlayerPositioning,
) -> Option<Point2<Field>> {
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut count = 0;
    let mut candidates = Vec::new();

    for (point, ownership) in map.cells() {
        // The outlet must be able to move ahead of a striker whose cell covers the front lane.
        let eligible = match positioning {
            PlayerPositioning::ForwardSupport {
                minimum_distance, ..
            } => {
                matches!(ownership, Ownership::Robot(_))
                    && ball_position.is_some_and(|ball| (point - ball).norm() >= minimum_distance)
            }
            _ => ownership == Ownership::Robot(player),
        };
        if !eligible
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
        let positioning_priority = match positioning {
            PlayerPositioning::Normal => 0.0,
            PlayerPositioning::ForwardSupport { anchor, .. } | PlayerPositioning::Lane(anchor) => {
                -(point - anchor).norm_squared()
            }
        };
        if best_target.is_none_or(|(best_priority, best_score, _)| {
            positioning_priority > best_priority
                || (positioning_priority == best_priority && score > best_score)
        }) {
            best_target = Some((positioning_priority, score, point));
        }
    }

    best_target.map(|(_, _, point)| point)
}

#[cfg(test)]
mod ball_support_tests {
    use std::time::Duration;

    use crate::node::LastBall;
    use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
    use types::{
        field_dimensions::Side,
        path::traits::{EndPoints, PathProgress, Project},
        world_state::BallSource,
    };

    use super::*;

    fn recovery_board() -> Blackboard {
        let mut board = Blackboard {
            player_number: PlayerNumber::Two,
            field_dimensions: FieldDimensions {
                length: 22.0,
                width: 14.0,
                ball_radius: 0.11,
                ..FieldDimensions::SPL_2025
            },
            last_motion_type: Some(MotionType::Walk),
            ball: Some(LastBall {
                position: point![1.3, 0.0],
                velocity: linear_algebra::vector![-0.8, 0.0],
                age: Default::default(),
                field_side: Side::Left,
                source: BallSource::Own,
            }),
            ..Default::default()
        };
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.path_planning.ball_obstacle_radius = 0.05;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board
            .parameters
            .walking
            .walk_and_stand
            .target_reached_thresholds
            .x = 0.05;
        board.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
            linear_algebra::vector![3.0, 0.0],
            std::f32::consts::PI,
        ));
        board
    }

    #[test]
    fn own_half_recovery_leaves_lane_before_circling_and_keeps_ball_facing() {
        let mut board = recovery_board();
        let tree = crate::tree::striker_subtree();
        for (robot_x, robot_y, circling) in [
            (3.0, 0.0, false),
            (3.0, 0.9, true),
            (3.0, 0.6, true),
            (3.0, 0.4, false),
            (3.0, 0.7, true),
            (1.2, 0.7, true),
        ] {
            let transform = Isometry2::from_parts(
                linear_algebra::vector![robot_x, robot_y],
                std::f32::consts::PI,
            );
            board.world_state.robot.ground_to_field = Some(transform);
            assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
            let Some(BodyMotion::Walk {
                ref path,
                orientation_mode: OrientationMode::LookAt { target, .. },
                target_orientation,
                speed,
                ..
            }) = board.body_motion
            else {
                panic!("recovery must approach around the ball instead of kicking or chasing it")
            };
            let end = transform * path.end_point();
            assert!(end.y() > 0.8);
            if !circling {
                assert!(end.x() >= robot_x);
                assert!(speed > 0.5);
            } else {
                assert!(end.x() < 1.3 - 0.8);
            }
            assert!(path_keeps_ball_clearance(
                path,
                transform.inverse() * point![1.3, 0.0],
                0.51
            ));
            assert!((target - transform.inverse() * point![1.3, 0.0]).norm() < 1e-5);
            assert_eq!(
                target_orientation,
                Orientation2::from_vector(target.coords())
            );
            assert_eq!(board.ball_recovery_side, Some(Side::Left));
        }
        board.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
            linear_algebra::vector![0.7, 0.9],
            0.0,
        ));
        assert_eq!(recover_goal_side(&mut board), Status::Failure);
        assert!(board.ball_recovery_side.is_none());
        assert!(!board.ball_recovery_lane_cleared);
        board.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
            linear_algebra::vector![3.0, 0.0],
            0.0,
        ));
        assert_eq!(tree.tick_with_trace(&mut board).0, Status::Success);
        let Some(BodyMotion::Walk { ref path, .. }) = board.body_motion else {
            panic!("expected recovery for a ball behind the body")
        };
        let end = board.world_state.robot.ground_to_field.unwrap() * path.end_point();
        assert!(end.x() >= 3.0 && end.y() > 0.8);
    }

    #[test]
    fn goal_side_recovery_stays_infield_and_releases_invalid_or_incoming_balls() {
        for y in [-6.5, 6.5] {
            let mut board = recovery_board();
            board.ball.as_mut().unwrap().position = point![1.3, y];
            board.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
                linear_algebra::vector![3.0, y],
                std::f32::consts::PI,
            ));
            assert_eq!(recover_goal_side(&mut board), Status::Success);
            let Some(BodyMotion::Walk { ref path, .. }) = board.body_motion else {
                panic!("expected recovery")
            };
            let target = board.world_state.robot.ground_to_field.unwrap() * path.end_point();
            assert!(target.y().abs() < y.abs());
        }
        for case in 0..4 {
            let mut board = recovery_board();
            board.ball_recovery_side = Some(Side::Left);
            match case {
                0 => board.player_number = PlayerNumber::One,
                1 => board.ball.as_mut().unwrap().velocity = linear_algebra::vector![2.0, 0.0],
                2 => board.ball.as_mut().unwrap().velocity = linear_algebra::vector![0.0, 2.0],
                _ => board.world_state.now = board.world_state.now + Duration::from_secs(2),
            }
            assert_eq!(recover_goal_side(&mut board), Status::Failure);
            assert!(board.ball_recovery_side.is_none());
            assert!(!board.ball_recovery_lane_cleared);
        }
    }

    #[test]
    fn ball_centered_arc_keeps_clearance_without_accepting_unsafe_segments() {
        let ball = point![0.3, 0.9];
        let mut path = Path {
            segments: vec![PathSegment::Arc(Arc {
                circle: Circle::new(ball, 0.45),
                start: Orientation2::new(-std::f32::consts::FRAC_PI_4),
                end: Orientation2::new(std::f32::consts::FRAC_PI_2),
                direction: Direction::Counterclockwise,
            })],
        };
        assert!(path_keeps_ball_clearance(&path, ball, 0.45));
        assert!(!path_keeps_ball_clearance(&path, ball, 0.46));
        path.segments.push(PathSegment::LineSegment(LineSegment(
            Point2::origin(),
            ball,
        )));
        assert!(!path_keeps_ball_clearance(&path, ball, 0.45));
    }

    #[test]
    fn predicted_approach_faces_observed_ball_before_and_after_overshoot() {
        for robot_x in [0.0, 3.0] {
            let transform = Isometry2::from_parts(linear_algebra::vector![robot_x, 0.0], 0.0);
            let mut board = Blackboard {
                field_dimensions: FieldDimensions::SPL_2025,
                ..Default::default()
            };
            board.world_state.robot.ground_to_field = Some(transform);
            board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
            board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
            board.parameters.kicking.approach_ball_standoff = 0.3;
            board.parameters.walking.path_planning.robot_radius = 0.4;
            board.parameters.walking.path_planning.ball_obstacle_radius = 0.1;
            board.ball = Some(LastBall {
                position: point![1.0, 0.4],
                velocity: linear_algebra::vector![-3.0, 0.0],
                age: board.world_state.now,
                field_side: Side::Left,
                source: BallSource::Own,
            });
            board.last_ball = board.ball.clone();
            assert_eq!(walk_to_ball(&mut board), Status::Success);
            let Some(BodyMotion::Walk {
                path,
                orientation_mode: OrientationMode::LookAt { target, .. },
                target_orientation,
                ..
            }) = &board.body_motion
            else {
                panic!("expected approach facing the observed ball")
            };
            let observed = transform.inverse() * point![1.0, 0.4];
            approx::assert_relative_eq!(*target, observed);
            approx::assert_relative_eq!(
                target_orientation.as_unit_vector(),
                observed.coords().normalize()
            );
            assert!(path.forward(Point2::origin()).x() < 0.0);
            assert!((transform * path.end_point()).x() < 0.0);
        }
    }

    #[test]
    fn far_stationary_ball_approach_walks_to_clearance_on_requested_side() {
        for obstacle_radius in [0.0, 0.05, 0.5] {
            let transform = Isometry2::identity();
            let ball = point![3.0, 0.0];
            let mut board = Blackboard {
                player_number: PlayerNumber::Two,
                field_dimensions: FieldDimensions::SPL_2025,
                last_motion_type: Some(MotionType::Walk),
                ball_action: Some(crate::kick::strategy::BallAction::ShootAtGoalCenter),
                ..Default::default()
            };
            board.world_state.robot.ground_to_field = Some(transform);
            board.parameters.goalkeeper.player_number = PlayerNumber::One;
            board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
            board.parameters.kicking.kick_activation_distance = 1.3;
            board.parameters.kicking.approach_ball_standoff = 0.3;
            board.parameters.kicking.strategy.maximum_shot_distance = 12.0;
            board.parameters.walking.path_planning.robot_radius = 0.4;
            board.parameters.walking.path_planning.ball_obstacle_radius = obstacle_radius;
            board.parameters.walking.speed.kicking = 1.8;
            board.ball = Some(LastBall {
                position: transform * ball,
                velocity: Vector2::zeros(),
                age: board.world_state.now,
                field_side: Side::Left,
                source: BallSource::Own,
            });
            let clearance = 0.4 + obstacle_radius.max(board.field_dimensions.ball_radius);
            assert_eq!(execute_ball_action(&mut board), Status::Success);
            let Some(BodyMotion::Walk { path, .. }) = &board.body_motion else {
                panic!("far stationary ball must retain a walking approach")
            };
            approx::assert_relative_eq!(
                path.end_point(),
                point![3.0 - 1.1 * clearance, 0.0],
                epsilon = 1e-5
            );
            assert!(path.forward(Point2::origin()).x() > 0.0);
            assert!((path.project(ball) - ball).norm() >= clearance - 1e-5);
            assert!((path.end_point() - ball).norm() < 1.3);

            let far_side = plan(&mut board, point![3.3, 0.0], transform);
            approx::assert_relative_eq!(
                far_side.end_point(),
                point![3.0 + 1.1 * clearance, 0.0],
                epsilon = 1e-5
            );
            assert!((far_side.project(ball) - ball).norm() >= clearance - 1e-5);
        }
    }

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
    fn defending_keeper_keeps_post_clearance_without_reducing_other_margins() {
        use crate::{goalkeeper::set_goalkeeper_active_defense_position, node::LastBall};
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
                            position: point![-6.8, side * 2.8],
                            velocity: Default::default(),
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
                    assert_eq!(
                        set_goalkeeper_active_defense_position(&mut board),
                        Status::Success
                    );
                    let target = board.walk_position.unwrap();
                    let path = plan(&mut board, target, transform);
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

                    plan(
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

                    board.player_number = PlayerNumber::Two;
                    let fielder_path = plan(&mut board, target, transform);
                    assert!((fielder_path.end_point() - target).norm() > 0.15);
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
    fn approaches_contested_ball_from_either_side_at_tackle_distance() {
        use crate::node::LastBall;
        use types::{
            field_dimensions::Side,
            obstacles::{Obstacle, ObstacleKind},
            path::traits::EndPoints,
            world_state::BallSource,
        };
        let mut blackboard = Blackboard {
            player_number: PlayerNumber::Two,
            field_dimensions: FieldDimensions::SPL_2025,
            ..Default::default()
        };
        blackboard.parameters.goalkeeper.player_number = PlayerNumber::One;
        blackboard.world_state.robot.ground_to_field = Some(Default::default());
        blackboard.ball = Some(LastBall {
            position: point![2.0, 0.0],
            velocity: Default::default(),
            age: Default::default(),
            field_side: Side::Left,
            source: BallSource::Own,
        });
        blackboard.parameters.kicking.strategy.pressure_distance = 0.8;
        blackboard.parameters.ball.last_ball_timeout = std::time::Duration::from_secs(1);
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        blackboard.parameters.kicking.kick_activation_distance = 1.3;
        blackboard.parameters.walking.path_planning.robot_radius = 0.4;
        blackboard
            .parameters
            .walking
            .path_planning
            .ball_obstacle_radius = 0.5;
        blackboard.parameters.walking.speed.kicking = 1.8;
        let opponent_in_field: Point2<Field> = point![2.2, 0.0];
        for transform in [
            Isometry2::identity(),
            Isometry2::from_parts(linear_algebra::vector![3.5, 0.0], std::f32::consts::PI),
        ] {
            blackboard.world_state.robot.ground_to_field = Some(transform);
            blackboard.world_state.obstacles = vec![Obstacle {
                kind: ObstacleKind::Robot,
                position: transform.inverse() * opponent_in_field,
                radius_at_foot_height: 0.2,
                radius_at_hip_height: 0.2,
            }];
            assert_eq!(block_opponent_with_ball(&mut blackboard), Status::Success);
            let Some(BodyMotion::Walk {
                path,
                orientation_mode: OrientationMode::LookAt { target, .. },
                target_orientation,
                speed,
                ..
            }) = &blackboard.body_motion
            else {
                panic!("expected direct pressure on holder")
            };
            let ball = transform.inverse() * point![2.0, 0.0];
            let endpoint = path.end_point();
            assert!((transform * endpoint).x() < 2.0);
            assert!((endpoint - ball).norm() < 1.3);
            assert!((endpoint - blackboard.world_state.obstacles[0].position).norm() >= 0.6);
            approx::assert_relative_eq!(
                target_orientation.as_unit_vector(),
                ball.coords().normalize()
            );
            assert!(
                (path.project(ball) - ball).norm()
                    >= blackboard.parameters.walking.path_planning.robot_radius
                        + blackboard.field_dimensions.ball_radius
                        - 1e-5
            );
            approx::assert_relative_eq!(*target, ball);
            assert_eq!(*speed, 1.8);
        }
        let opponent = blackboard.world_state.obstacles.pop().unwrap();
        assert_eq!(block_opponent_with_ball(&mut blackboard), Status::Failure);
        blackboard.world_state.obstacles.push(opponent);
        blackboard.last_motion_type = Some(MotionType::Kick);
        assert_eq!(block_opponent_with_ball(&mut blackboard), Status::Failure);
        blackboard.last_motion_type = Some(MotionType::Walk);
        blackboard.world_state.robot.ground_to_field = Some(Isometry2::from_parts(
            linear_algebra::vector![3.5, 0.0],
            0.0,
        ));
        blackboard.world_state.obstacles[0].position = point![-1.3, 0.0];
        assert_eq!(block_opponent_with_ball(&mut blackboard), Status::Success);
        assert!(matches!(blackboard.body_motion,
            Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity == Vector2::zeros()));
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
        board.world_state.obstacles = vec![types::obstacles::Obstacle::robot(
            point![2.2, 0.0],
            0.2,
            0.2,
        )];
        let target = point![3.5, 0.0];
        let path = plan_interception(&mut board, target, Isometry2::identity());
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
        assert_eq!(blocked, direct_path(Point2::origin(), Point2::origin()));
    }

    #[test]
    fn more_ball_weight_moves_support_toward_ball_ring() {
        let mut map = VoronoiGrid::new(point![-4.5, -3.0], point![4.5, 3.0], 0.2);
        map.multi_source_dijkstra(&[(Pose2::new(point![-2.0, 0.0], 0.0), PlayerNumber::Two)]);
        let ball = point![3.8, 0.0];
        let mut parameters = VoronoiParameters {
            ball_weight: 1.0,
            relative_ball_support_sigma: 0.5 / FieldDimensions::SPL_2025.width,
            centroid_anchor_weight: 0.6,
            relative_centroid_anchor_sigma: 1.0 / FieldDimensions::SPL_2025.width,
            ..Default::default()
        };
        let before = target_player_position(
            &map,
            PlayerNumber::Two,
            Some(ball),
            &FieldDimensions::SPL_2025,
            &parameters,
            PlayerPositioning::Normal,
        )
        .unwrap();
        parameters.ball_weight = 3.0;
        let after = target_player_position(
            &map,
            PlayerNumber::Two,
            Some(ball),
            &FieldDimensions::SPL_2025,
            &parameters,
            PlayerPositioning::Normal,
        )
        .unwrap();
        assert!((after - ball).norm() < (before - ball).norm());
        assert!(
            ((after - ball).norm() - FieldDimensions::SPL_2025.center_circle_diameter / 2.0).abs()
                < 0.5
        );
    }

    #[test]
    fn four_and_five_players_keep_forward_support_and_defensive_cover() {
        use std::time::Duration;

        use crate::{
            conditions::is_closest_to_ball, node::LastBall, voronoi::calculate_voronoi_grid,
        };
        use hsl_network_messages::SubState;
        use types::{
            field_dimensions::Side,
            obstacles::Obstacle,
            path::traits::EndPoints,
            time_wrapper::TimeWrapper,
            world_state::{BallSource, PlayerState},
        };

        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                width: 9.0,
                ..FieldDimensions::SPL_2025
            },
            ..Default::default()
        };
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        board.world_state.filtered_game_controller_state =
            Some(crate::test_support::filtered_game_controller_state());
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(5);
        board
            .parameters
            .kicking
            .strategy
            .passing
            .maximum_receiver_age = Duration::from_secs(3);
        board.parameters.voronoi = VoronoiParameters {
            grid_resolution: 0.2,
            padding: 0.5,
            forward_weight: 0.2,
            ball_weight: 2.5,
            relative_ball_support_sigma: 0.5 / board.field_dimensions.width,
            centroid_anchor_weight: 0.6,
            relative_centroid_anchor_sigma: 1.0 / board.field_dimensions.width,
        };
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.kicking.strategy.passing.minimum_distance = 4.0;
        board.parameters.walking.speed.kicking = 1.8;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        let ball = point![6.0, 0.0];
        board.ball = Some(LastBall {
            position: ball,
            velocity: Vector2::zeros(),
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        for (number, position) in [
            (PlayerNumber::One, point![-6.7, 0.0]),
            (PlayerNumber::Two, point![1.0, 0.0]),
            (PlayerNumber::Three, point![5.8, 0.0]),
            (PlayerNumber::Four, point![-5.0, -3.0]),
            (PlayerNumber::Five, point![-4.0, 3.0]),
        ] {
            board.world_state.player_states[number] = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: position.into(),
                    ..crate::test_support::player_state()
                },
            });
        }
        let mut targets = Vec::new();
        for number in [PlayerNumber::Two, PlayerNumber::Four, PlayerNumber::Five] {
            let mut supporter = board.clone();
            let transform = supporter.world_state.player_states[number]
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform();
            supporter.player_number = number;
            supporter.world_state.robot.ground_to_field = Some(transform);
            assert_eq!(calculate_voronoi_grid(&mut supporter), Status::Success);
            let map = supporter.voronoi_map.as_ref().unwrap();
            let positioning = field_player_positioning(&supporter, map).unwrap();
            if number == PlayerNumber::Two {
                assert!(matches!(
                    positioning,
                    PlayerPositioning::ForwardSupport { .. }
                ));
            } else {
                assert!(matches!(positioning, PlayerPositioning::Lane(_)));
            }
            let normal = target_player_position(
                map,
                number,
                Some(ball),
                &supporter.field_dimensions,
                &supporter.parameters.voronoi,
                PlayerPositioning::Normal,
            )
            .unwrap();
            assert_eq!(walk_to_voronoi_position(&mut supporter), Status::Success);
            assert!(!is_closest_to_ball(&mut supporter));
            let Some(BodyMotion::Walk { path, .. }) = &supporter.body_motion else {
                panic!("support must walk without kicking")
            };
            let target = transform * path.end_point();
            if number == PlayerNumber::Two {
                assert!((normal - ball).norm() > 4.0);
                assert!((target - ball).norm() > 3.0);
                assert!(target.x() >= ball.x());
                assert!(target.y().abs() > 3.0);
            } else {
                assert_eq!(
                    supporter.voronoi_map.as_ref().unwrap().ownership_at(target),
                    Some(Ownership::Robot(number))
                );
                assert!((target - ball).norm() > 3.0);
            }
            targets.push(target);

            if number != PlayerNumber::Four {
                continue;
            }
            supporter.kickoff_started = Some(supporter.world_state.now);
            assert_eq!(
                field_player_positioning(&supporter, supporter.voronoi_map.as_ref().unwrap()),
                None
            );
            supporter.kickoff_started = None;
            for state in [SubState::CornerKick, SubState::ThrowIn, SubState::GoalKick] {
                supporter
                    .world_state
                    .filtered_game_controller_state
                    .as_mut()
                    .unwrap()
                    .sub_state = Some(state);
                assert_eq!(
                    field_player_positioning(&supporter, supporter.voronoi_map.as_ref().unwrap()),
                    None
                );
            }
        }
        assert!(targets[1].x() < targets[2].x());
        assert!(targets[1].x() < 0.0);
        assert!(targets[2].y() > 1.0);
        for (index, target) in targets.iter().enumerate() {
            for other in &targets[index + 1..] {
                assert!(
                    (*target - *other).norm()
                        > 2.0 * board.parameters.walking.path_planning.robot_radius
                );
            }
        }
        let mut four = board.clone();
        four.field_dimensions.length = 22.0;
        four.field_dimensions.width = 14.0;
        four.field_dimensions.ball_radius = 0.11;
        four.world_state.player_states.five = None;
        for (number, offensive) in [(PlayerNumber::Two, true), (PlayerNumber::Four, false)] {
            let transform = four.world_state.player_states[number]
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform();
            four.player_number = number;
            four.world_state.robot.ground_to_field = Some(transform);
            calculate_voronoi_grid(&mut four);
            let map = four.voronoi_map.as_ref().unwrap();
            let positioning = field_player_positioning(&four, map).unwrap();
            assert_eq!(
                matches!(positioning, PlayerPositioning::ForwardSupport { .. }),
                offensive
            );
            let target = target_player_position(
                map,
                number,
                Some(ball),
                &four.field_dimensions,
                &four.parameters.voronoi,
                positioning,
            )
            .unwrap();
            if offensive {
                assert!(target.x() > ball.x() + 1.0);
                assert!((target - ball).norm() >= 4.0);
                assert!(target.y().abs() > 3.0);
            } else {
                assert!(target.x() < 0.0);
                assert_eq!(map.ownership_at(target), Some(Ownership::Robot(number)));
            }
        }
        let mut defense = board.clone();
        defense
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = point![5.0, 0.5].into();
        defense
            .world_state
            .player_states
            .five
            .as_mut()
            .unwrap()
            .inner
            .pose = point![1.0, 3.0].into();
        defense.parameters.kicking.strategy.pressure_distance = 0.8;
        let holder = point![6.7, 0.2];
        let receiver = point![-2.0, 4.0];
        let screen = holder + (receiver - holder) * 0.5;
        for number in [PlayerNumber::Two, PlayerNumber::Four, PlayerNumber::Five] {
            let mut peer = defense.clone();
            let transform = peer.world_state.player_states[number]
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform();
            peer.player_number = number;
            peer.world_state.robot.ground_to_field = Some(transform);
            assert_eq!(calculate_voronoi_grid(&mut peer), Status::Success);
            let baseline = field_player_positioning(&peer, peer.voronoi_map.as_ref().unwrap());
            peer.world_state.obstacles = [holder, receiver, point![-0.5, 4.0], point![5.8, 0.0]]
                .into_iter()
                .map(|position| Obstacle::robot(transform.inverse() * position, 0.2, 0.2))
                .collect();
            assert_eq!(calculate_voronoi_grid(&mut peer), Status::Success);
            let map = peer.voronoi_map.as_ref().unwrap();
            let positioning = field_player_positioning(&peer, map);
            if number == PlayerNumber::Five {
                let Some(PlayerPositioning::Lane(anchor)) = positioning else {
                    panic!("advanced fielder must screen a reachable opponent pass")
                };
                approx::assert_relative_eq!(anchor, screen);
                assert_ne!(positioning, baseline);
                let target = target_player_position(
                    map,
                    number,
                    Some(ball),
                    &peer.field_dimensions,
                    &peer.parameters.voronoi,
                    positioning.unwrap(),
                )
                .unwrap();
                assert_eq!(map.ownership_at(target), Some(Ownership::Robot(number)));
                assert!((target - screen).norm() <= map.resolution());
                assert!((target - ball).norm() > 3.0);
            } else {
                assert_eq!(positioning, baseline);
            }
            peer.world_state.obstacles.reverse();
            assert_eq!(
                field_player_positioning(&peer, peer.voronoi_map.as_ref().unwrap()),
                positioning
            );
            peer.world_state
                .obstacles
                .retain(|obstacle| (transform * obstacle.position - holder).norm() > 0.01);
            assert_eq!(
                field_player_positioning(&peer, peer.voronoi_map.as_ref().unwrap()),
                baseline
            );
        }
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.ground_to_field = Some(
            board
                .world_state
                .player_states
                .two
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform(),
        );
        board.world_state.player_states.four = None;
        board.world_state.player_states.five = None;
        calculate_voronoi_grid(&mut board);
        assert_eq!(
            field_player_positioning(&board, board.voronoi_map.as_ref().unwrap()),
            None
        );
        let normal = target_player_position(
            board.voronoi_map.as_ref().unwrap(),
            PlayerNumber::Two,
            Some(ball),
            &board.field_dimensions,
            &board.parameters.voronoi,
            PlayerPositioning::Normal,
        )
        .unwrap();
        assert_eq!(walk_to_voronoi_position(&mut board), Status::Success);
        let Some(BodyMotion::Walk { path, .. }) = &board.body_motion else {
            panic!("three-player supporter must retain normal formation")
        };
        let target = board.world_state.robot.ground_to_field.unwrap() * path.end_point();
        approx::assert_relative_eq!(target, normal);
    }
}
