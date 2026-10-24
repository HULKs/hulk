use std::time::{Duration, UNIX_EPOCH};

use ::geometry::rectangle::Rectangle;
use approx::assert_relative_eq;
use hsl_network_messages::{GamePhase, PlayerNumber, SubState, Team};
use linear_algebra::{IntoTransform, Pose2, vector};
use ros_z::time::Time;
use types::{
    ball_position::BallPosition,
    behavior_tree::NodeTrace,
    fall_detection::{FallDetection, Posture},
    field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
    filtered_game_controller_state::FilteredGameControllerState,
    filtered_game_state::FilteredGameState,
    obstacles::{Obstacle, ObstacleKind},
    parameters::KickingParameters,
    primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
    world_state::{BallState, PlayerState},
};

use crate::{
    conditions::is_ball_interception_candidate,
    motion_assembler::assemble_motion_command,
    node::LastBall,
    tree::{create_tree, striker_subtree},
    walk::{plan, walk_alternatives_subtree, walk_to_ball},
};

use super::strategy::{BallAction, QuickKickReason, select_ball_action, urgent_kick_reason};
use super::*;

fn blackboard(robot_x: f32, robot_y: f32, yaw: f32) -> Blackboard {
    let mut blackboard = Blackboard {
        field_dimensions: FieldDimensions::SPL_2025,
        player_number: PlayerNumber::Three,
        head_yaw: 0.0,
        parameters: json5::from_str(include_str!(
            "../../../../../etc/parameters/base/behavior_node.json5"
        ))
        .unwrap(),
        walking_velocity_limits: Default::default(),
        search_cell_size: None,
        world_state: Default::default(),
        controller_input: None,
        remote_control_enabled: false,
        path_obstacles_output: Vec::new(),
        time_since_last_switch: Duration::ZERO,
        direction_difference: 0.0,
        voronoi_inputs: Vec::new(),
        ball: None,
        visual_kick_ball_position: None,
        last_ball: None,
        last_close_enough_to_kick: false,
        ball_action: None,
        last_ball_action: None,
        kick_target: None,
        last_kick_target: None,
        last_motion_command: None,
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
    };
    blackboard.parameters.control.is_simple = true;
    blackboard.world_state.robot.primary_state = PrimaryState::Playing;
    blackboard.world_state.robot.ground_to_field =
        Some(nalgebra::Isometry2::new(nalgebra::vector![robot_x, robot_y], yaw).framed_transform());
    blackboard.world_state.now = Time::from_nanos(10_000_000_000);
    blackboard.world_state.fall_detection = Some(FallDetection {
        time: blackboard.world_state.now,
        valid_until: blackboard.world_state.now + Duration::from_secs(1),
        posture: Posture::Upright,
    });
    set_ball(&mut blackboard, point![0.8, 0.0]);
    blackboard
}

fn is_urgent_kick(blackboard: &Blackboard) -> bool {
    urgent_kick_reason(blackboard).is_some()
}

fn set_ball(blackboard: &mut Blackboard, position: Point2<Ground>) {
    let ground_to_field = blackboard.world_state.robot.ground_to_field.unwrap();
    let ball = LastBall {
        position: ground_to_field * position,
        velocity: ground_to_field * vector![-1.0, 0.0],
        age: Time::zero(),
        field_side: Side::Left,
        source: Default::default(),
    };
    blackboard.ball = Some(ball.clone());
    blackboard.last_ball = Some(ball.clone());
    blackboard.visual_kick_ball_position = Some(BallPosition {
        position,
        velocity: vector![-1.0, 0.0],
        last_seen: blackboard.world_state.now,
    });
    blackboard.world_state.ball = Some(BallState {
        ball_in_ground: position,
        ball_in_field: ball.position,
        ball_in_ground_velocity: vector![-1.0, 0.0],
        last_seen_ball: UNIX_EPOCH,
        field_side: Side::Left,
        source: Default::default(),
    });
}

fn trace_contains(trace: &NodeTrace, name: &str) -> bool {
    trace.name == name
        || trace
            .children
            .iter()
            .any(|child| trace_contains(child, name))
}

fn game_state() -> FilteredGameControllerState {
    FilteredGameControllerState {
        game_state: FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        },
        opponent_game_state: FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        },
        half: hsl_network_messages::Half::First,
        remaining_time_in_half: Duration::ZERO,
        game_phase: GamePhase::Normal,
        kicking_team: Some(Team::Opponent),
        penalties: Default::default(),
        remaining_number_of_messages: 0,
        sub_state: None,
        global_field_side: GlobalFieldSide::Home,
        new_own_penalties_last_cycle: Default::default(),
        new_opponent_penalties_last_cycle: Default::default(),
    }
}

#[test]
fn urgent_goal_distance_uses_robot_to_goal_mouth_including_three_metre_boundary() {
    for sign in [-1.0, 1.0] {
        for (x, y, urgent) in [
            (1.499, 0.0, false),
            (1.5, 0.0, true),
            (1.501, 0.0, true),
            (4.5, 3.75, true), // 3 m from the end of the mouth, not its centre.
            (4.5, 3.751, false),
        ] {
            let mut blackboard = blackboard(sign * x, y, 1.1);
            assert_eq!(
                is_urgent_kick(&mut blackboard),
                urgent,
                "robot {sign} * {x}, {y}"
            );
        }
    }
    let mut blackboard = blackboard(0.0, 0.0, 0.0);
    set_ball(&mut blackboard, point![4.0, 0.0]);
    assert!(!is_urgent_kick(&mut blackboard));
}

#[test]
fn urgent_robot_pressure_uses_centres_around_us_at_one_point_five_metres() {
    for (position, urgent) in [
        (point![1.499, 0.0], true),
        (point![1.5, 0.0], true),
        (point![1.501, 0.0], false),
        (point![0.0, 1.5], true),
        (point![-1.5, 0.0], true), // Far from the ball, still close to us.
        (point![2.0, 0.0], false), // Close to the ball, too far from us.
    ] {
        let mut blackboard = blackboard(0.0, 0.0, 0.9);
        blackboard.world_state.obstacles = vec![Obstacle::robot(position, 0.5, 0.5)];
        assert_eq!(is_urgent_kick(&mut blackboard), urgent, "{position:?}");
    }
    for (kind, urgent) in [
        (ObstacleKind::Robot, true),
        (ObstacleKind::Person, true),
        (ObstacleKind::Unknown, true),
        (ObstacleKind::Ball, false),
        (ObstacleKind::GoalPost, false),
    ] {
        let mut blackboard = blackboard(0.0, 0.0, 0.0);
        blackboard.world_state.obstacles = vec![Obstacle {
            kind,
            ..Obstacle::robot(point![1.0, 0.0], 0.2, 0.2)
        }];
        assert_eq!(is_urgent_kick(&mut blackboard), urgent);
    }
}

#[test]
fn teammate_pressure_is_transformed_and_excludes_our_own_broadcast() {
    for (distance, urgent) in [(1.499, true), (1.5, true), (1.501, false)] {
        let mut blackboard = blackboard(0.0, 1.0, 0.8);
        let teammate = PlayerState {
            pose: Pose2::from_parts(point![-distance, 1.0], Orientation2::new(0.0)),
            head_yaw: 0.0,
            ball_position: None,
        };
        blackboard.world_state.player_states[PlayerNumber::Two] = Some(teammate);
        assert_eq!(is_urgent_kick(&mut blackboard), urgent);
        blackboard.world_state.player_states[PlayerNumber::Two] = None;
        blackboard.world_state.player_states[PlayerNumber::Three] = Some(teammate);
        assert!(!is_urgent_kick(&mut blackboard));
    }
}

#[test]
fn each_urgent_trigger_precedes_target_planning_and_interception_in_the_live_tree() {
    for trigger in 0..4 {
        let mut blackboard = match trigger {
            0 => blackboard(-2.0, 0.0, 0.0),
            1 => blackboard(2.0, 0.0, 0.0),
            _ => blackboard(0.0, 0.0, 0.0),
        };
        match trigger {
            2 => {
                blackboard.world_state.obstacles =
                    vec![Obstacle::robot(point![-1.5, 0.0], 0.2, 0.2)]
            }
            3 => {
                blackboard.world_state.player_states[PlayerNumber::Two] = Some(PlayerState {
                    pose: Pose2::from_parts(point![0.0, 1.5], Orientation2::new(0.0)),
                    head_yaw: 0.0,
                    ball_position: None,
                })
            }
            _ => (),
        }
        blackboard.parameters.kicking.quick = false;
        blackboard.ball.as_mut().unwrap().velocity = vector![-2.0, 0.0];
        assert!(is_ball_interception_candidate(&mut blackboard));
        let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
        assert_eq!(status, Status::Success);
        assert!(matches!(
            blackboard.body_motion,
            Some(BodyMotion::Kick { quick: true, .. })
        ));
        assert!(trace_contains(&trace, "subtree_quick_kick_subtree"));
        assert!(trace_contains(&trace, "select_ball_action"));
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::QuickKick {
                reason: match trigger {
                    0 => QuickKickReason::NearOwnGoal,
                    1 => QuickKickReason::NearOpponentGoal,
                    _ => QuickKickReason::NearbyRobot,
                }
            })
        );
        assert!(!trace_contains(&trace, "select_kick_target"));
        assert!(!trace_contains(&trace, "is_ball_interception_candidate"));
    }
}

#[test]
fn ordinary_and_fallback_kicks_default_to_quick() {
    assert!(KickingParameters::default().quick);
    assert!(KickingParameters::default().allow_strong_kicks);
    assert_eq!(
        KickingParameters::default().strong_kick_min_target_distance,
        2.0
    );
    let mut blackboard = blackboard(1.0, 0.0, 0.0);
    assert!(blackboard.parameters.kicking.quick);
    assert!(blackboard.parameters.kicking.allow_strong_kicks);
    blackboard.ball.as_mut().unwrap().velocity = Vector2::zeros();
    let (status, trace) = striker_subtree().tick_with_trace(&mut blackboard);
    assert_eq!(status, Status::Success);
    assert!(matches!(
        blackboard.body_motion,
        Some(BodyMotion::Kick { quick: true, .. })
    ));
    assert!(trace_contains(&trace, "select_kick_target"));
    assert!(!trace_contains(&trace, "select_fallback_kick_target"));
    assert!(matches!(
        blackboard.ball_action,
        Some(BallAction::Shoot { .. })
    ));

    blackboard.world_state.obstacles = vec![Obstacle::robot(point![2.5, 0.0], 1.0, 1.0)];
    blackboard.parameters.kicking.quick = false;
    assert!(!is_urgent_kick(&mut blackboard));
    let (status, trace) = striker_subtree().tick_with_trace(&mut blackboard);
    assert_eq!(status, Status::Success);
    assert!(trace_contains(&trace, "select_fallback_kick_target"));
    assert_eq!(
        blackboard.ball_action,
        Some(BallAction::QuickKick {
            reason: QuickKickReason::NoUsablePlan
        })
    );
    assert!(matches!(
        blackboard.body_motion,
        Some(BodyMotion::Kick { quick: true, .. })
    ));
}

#[test]
fn ordinary_target_uses_an_open_goal_segment() {
    let mut blackboard = blackboard(1.0, 0.0, 0.0);
    blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.0, -0.1], 0.15, 0.15)];
    assert_eq!(select_ball_action(&mut blackboard), Status::Success);
    assert!(matches!(
        blackboard.ball_action,
        Some(BallAction::Shoot { .. })
    ));
    assert_eq!(kick(&mut blackboard), Status::Success);
    assert_eq!(select_kick_target(&mut blackboard), Status::Success);
    let target = blackboard.kick_target.unwrap();
    assert!(target.y() > 0.0);
    blackboard.last_motion_type = Some(MotionType::Kick);
    blackboard.world_state.obstacles.clear();
    kick(&mut blackboard);
    assert_eq!(select_kick_target(&mut blackboard), Status::Success);
    assert_eq!(blackboard.kick_target, Some(target));
}

#[test]
fn urgent_kicks_keep_reachability_and_localization_checks() {
    for robot_x in [-2.0, 2.0, 0.0] {
        let mut blackboard = blackboard(robot_x, 0.0, 0.0);
        blackboard.world_state.obstacles = vec![Obstacle::robot(point![-1.0, 0.0], 0.2, 0.2)];
        set_ball(&mut blackboard, point![2.0, 0.0]);
        let (status, _) = quick_kick_subtree().tick_with_trace(&mut blackboard);
        assert_eq!(status, Status::Failure);
        let (status, _) = create_tree().tick_with_trace(&mut blackboard);
        assert!(!matches!(
            assemble_motion_command(&blackboard, status).unwrap(),
            MotionCommand::Kick { .. }
        ));
    }
    let mut blackboard = blackboard(2.0, 0.0, 0.0);
    let ball = blackboard.ball.clone();
    blackboard.ball = None;
    assert_eq!(
        quick_kick_subtree().tick_with_trace(&mut blackboard).0,
        Status::Failure
    );
    blackboard.ball = ball;
    blackboard.world_state.robot.ground_to_field = None;
    assert_eq!(
        quick_kick_subtree().tick_with_trace(&mut blackboard).0,
        Status::Failure
    );
}

#[test]
fn blocked_approaches_do_not_delay_reachable_quick_kicks_or_kick_unreachable_balls() {
    for (ball_x, sub_state, fallback) in [
        (1.0, None, true),
        (2.0, None, false),
        (1.0, Some(SubState::CornerKick), false),
    ] {
        let mut blackboard = blackboard(0.0, 0.0, 0.0);
        set_ball(&mut blackboard, point![ball_x, 0.0]);
        let mut state = game_state();
        state.sub_state = sub_state;
        blackboard.world_state.filtered_game_controller_state = Some(state);
        blackboard.world_state.rule_obstacles = vec![RuleObstacle::Rectangle(Rectangle {
            min: point![0.5, -2.0],
            max: point![4.0, 2.0],
        })];
        let transform = blackboard.world_state.robot.ground_to_field.unwrap();
        assert!(plan(&mut blackboard, point![ball_x - 0.3, 0.0], transform).is_none());
        assert_eq!(walk_to_ball(&mut blackboard), Status::Failure);
        let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
        let command = assemble_motion_command(&blackboard, status).unwrap();
        assert_eq!(
            matches!(command, MotionCommand::Kick { quick: true, .. }),
            fallback
        );
        if fallback {
            assert_eq!(
                blackboard.ball_action,
                Some(BallAction::QuickKick {
                    reason: QuickKickReason::NoUsablePlan
                })
            );
            assert!(trace_contains(&trace, "select_ball_action"));
            assert!(!trace_contains(&trace, "walk_to_ball"));
        }
    }
}

#[test]
fn approach_and_execution_follow_the_same_selected_opening() {
    let mut blackboard = blackboard(0.0, 0.0, 0.0);
    set_ball(&mut blackboard, point![2.0, 0.0]);
    blackboard.world_state.obstacles = vec![Obstacle::robot(point![3.5, -0.02], 0.1, 0.1)];
    let (status, _) = create_tree().tick_with_trace(&mut blackboard);
    assert_eq!(status, Status::Success);
    let Some(BallAction::Shoot { opening }) = blackboard.ball_action else {
        panic!("expected selected shot")
    };
    assert!(opening.center().y() > 0.0);
    let Some(BodyMotion::Walk {
        target_orientation, ..
    }) = blackboard.body_motion
    else {
        panic!("expected approach")
    };
    let direction =
        Orientation2::from_vector(opening.center() - blackboard.ball.as_ref().unwrap().position);
    assert_relative_eq!(target_orientation.angle(), direction.angle());

    blackboard.world_state.robot.ground_to_field =
        Some(nalgebra::Isometry2::new(nalgebra::vector![1.2, 0.0], 0.0).framed_transform());
    set_ball(&mut blackboard, point![0.8, 0.0]);
    blackboard.world_state.obstacles[0].position = point![2.3, -0.02];
    let (status, _) = create_tree().tick_with_trace(&mut blackboard);
    assert_eq!(status, Status::Success);
    let Some(BodyMotion::Kick { kick_direction, .. }) = blackboard.body_motion else {
        panic!("expected selected shot to execute")
    };
    assert_relative_eq!(kick_direction.angle(), direction.angle());
    assert_eq!(blackboard.last_kick_target, Some(opening.center()));
}

#[test]
fn fresh_ball_refinement_failure_is_an_explicit_quick_fallback_before_interception() {
    let mut blackboard = blackboard(0.0, 0.0, 0.0);
    set_ball(&mut blackboard, point![2.0, 0.0]);
    blackboard.ball.as_mut().unwrap().velocity = vector![-2.0, 0.0];
    blackboard
        .visual_kick_ball_position
        .as_mut()
        .unwrap()
        .position = point![0.8, 0.0];
    blackboard.parameters.kicking.quick = false;
    let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
    assert_eq!(status, Status::Success);
    assert_eq!(
        blackboard.ball_action,
        Some(BallAction::QuickKick {
            reason: QuickKickReason::NoUsablePlan
        })
    );
    assert!(
        matches!(blackboard.body_motion, Some(BodyMotion::Kick { quick: true, ball_position, .. }) if ball_position == point![0.8, 0.0])
    );
    assert!(trace_contains(&trace, "select_kick_target"));
    assert!(!trace_contains(&trace, "intercept"));
}

#[test]
fn a_close_remembered_ball_cannot_authorize_kicking_a_far_visual_ball() {
    let mut blackboard = blackboard(2.0, 0.0, 0.0);
    blackboard
        .visual_kick_ball_position
        .as_mut()
        .unwrap()
        .position = point![2.0, 0.0];
    let (status, _) = create_tree().tick_with_trace(&mut blackboard);
    assert!(!matches!(
        assemble_motion_command(&blackboard, status).unwrap(),
        MotionCommand::Kick { .. }
    ));
}

#[test]
fn urgent_selection_precedes_planning_bounds_but_still_requires_visual_reach() {
    for (visual_x, kicks) in [(0.3, true), (2.0, false)] {
        let mut blackboard = blackboard(4.0, 0.0, 0.0);
        set_ball(&mut blackboard, point![0.6, 0.0]); // Planning ball is beyond the goal line.
        blackboard
            .visual_kick_ball_position
            .as_mut()
            .unwrap()
            .position = point![visual_x, 0.0];
        let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
        assert_eq!(
            blackboard.ball_action,
            Some(BallAction::QuickKick {
                reason: QuickKickReason::NearOpponentGoal
            })
        );
        assert!(trace_contains(&trace, "select_ball_action"));
        assert_eq!(
            matches!(
                assemble_motion_command(&blackboard, status).unwrap(),
                MotionCommand::Kick { quick: true, .. }
            ),
            kicks
        );
    }
}

#[test]
fn selector_intent_is_cleared_when_leaving_ordinary_striker_play() {
    let mut blackboard = blackboard(1.0, 0.0, 0.0);
    let tree = create_tree();
    tree.tick_with_trace(&mut blackboard);
    assert!(matches!(
        blackboard.ball_action,
        Some(BallAction::Shoot { .. })
    ));
    blackboard.world_state.robot.primary_state = PrimaryState::Set;
    tree.tick_with_trace(&mut blackboard);
    assert_eq!(blackboard.ball_action, None);
    tree.tick_with_trace(&mut blackboard);
    assert_eq!(blackboard.last_ball_action, None);

    blackboard.world_state.robot.primary_state = PrimaryState::Playing;
    tree.tick_with_trace(&mut blackboard);
    assert!(blackboard.ball_action.is_some());
    blackboard.ball = None;
    blackboard.world_state.ball = None;
    tree.tick_with_trace(&mut blackboard);
    assert_eq!(blackboard.ball_action, None);
}

#[test]
fn goal_shot_strength_uses_ball_to_mouth_distance_at_two_metres() {
    for target_x in [4.5, 4.9, 6.0] {
        for (position, expected) in [
            (point![2.499, 0.0], true),
            (point![2.5, 0.0], false),
            (point![2.501, 0.0], false),
            (point![4.5, 2.751], true),
            (point![4.5, 2.75], false),
        ] {
            let mut blackboard = blackboard(0.0, 0.0, 0.0);
            set_ball(&mut blackboard, position);
            blackboard.parameters.kicking.target_speed = 1.2;
            assert_eq!(kick(&mut blackboard), Status::Success);
            assert_eq!(
                apply_kick_target(&mut blackboard, point![target_x, 0.0]),
                Status::Success
            );
            assert_eq!(
                kick_strength_subtree().tick_with_trace(&mut blackboard).0,
                Status::Success
            );
            assert!(
                matches!(blackboard.body_motion, Some(BodyMotion::Kick { strong, target_speed: 1.2, .. }) if strong == expected),
                "ball {position:?}, target {target_x}"
            );
        }
    }
}

#[test]
fn goal_strength_transforms_the_requested_ball_and_target() {
    for (distance, expected) in [(2.01, true), (1.99, false)] {
        let mut blackboard = blackboard(2.0, -0.3, 0.7);
        let transform = blackboard.world_state.robot.ground_to_field.unwrap();
        set_ball(
            &mut blackboard,
            transform.inverse() * point![4.5 - distance, 0.0],
        );
        kick(&mut blackboard);
        apply_kick_target(&mut blackboard, point![4.9, 0.2]);
        kick_strength_subtree().tick_with_trace(&mut blackboard);
        assert!(
            matches!(blackboard.body_motion, Some(BodyMotion::Kick { strong, .. }) if strong == expected)
        );
    }
}

#[test]
fn goal_line_roundoff_cannot_enable_strong_for_a_close_lateral_ball() {
    // Mouth distance is ~1.95 m, but distance to goal centre is 2.5 m.
    for yaw in [-2.3, -1.1, -0.7, 0.1, 0.7, 1.1, 2.3] {
        let mut blackboard = blackboard(0.1, -0.3, yaw);
        let transform = blackboard.world_state.robot.ground_to_field.unwrap();
        set_ball(&mut blackboard, transform.inverse() * point![3.0, 2.0]);
        kick(&mut blackboard);
        apply_kick_target(&mut blackboard, point![4.5, 0.0]);
        kick_strength_subtree().tick_with_trace(&mut blackboard);
        assert!(
            matches!(
                blackboard.body_motion,
                Some(BodyMotion::Kick { strong: false, .. })
            ),
            "yaw {yaw}"
        );
    }
}

#[test]
fn ongoing_kicks_keep_all_settings_in_urgent_normal_and_walk_alternative_paths() {
    for (path, settings) in (0..3).flat_map(|path| {
        [(false, true, false, 0.7), (true, false, true, 1.2)].map(|settings| (path, settings))
    }) {
        let mut blackboard = blackboard(if settings.0 { 2.0 } else { 0.0 }, 0.0, 0.0);
        kick(&mut blackboard);
        if let Some(BodyMotion::Kick {
            strong,
            soft,
            quick,
            target_speed,
            ..
        }) = blackboard.body_motion.as_mut()
        {
            (*strong, *soft, *quick, *target_speed) = settings;
        }
        blackboard.last_motion_command =
            Some(assemble_motion_command(&blackboard, Status::Success).unwrap());
        blackboard.last_motion_type = Some(MotionType::Kick);
        blackboard.last_motion_switch_time = blackboard.world_state.now;
        blackboard.parameters.kicking.target_speed = 4.0;
        let tree = match path {
            0 => quick_kick_subtree(),
            1 => kick_subtree(),
            _ => walk_alternatives_subtree(),
        };
        assert_eq!(tree.tick_with_trace(&mut blackboard).0, Status::Success);
        let Some(BodyMotion::Kick {
            strong,
            soft,
            quick,
            target_speed,
            ..
        }) = blackboard.body_motion
        else {
            panic!("expected ongoing kick")
        };
        assert_eq!((strong, soft, quick, target_speed), settings);
    }
}

#[test]
fn urgent_kicks_wait_for_motion_switch_safety() {
    for motion in [MotionType::Walk, MotionType::Stand, MotionType::Prepare] {
        let mut blackboard = blackboard(2.0, 0.0, 0.0);
        blackboard.last_motion_type = Some(motion);
        blackboard.last_motion_switch_time = blackboard.world_state.now;
        let (status, _) = create_tree().tick_with_trace(&mut blackboard);
        assert!(!matches!(
            assemble_motion_command(&blackboard, status).unwrap(),
            MotionCommand::Kick { .. }
        ));
    }
}

#[test]
fn urgency_does_not_override_primary_states_fall_protection_or_opponent_restarts() {
    for primary_state in [
        PrimaryState::Damping,
        PrimaryState::Stop,
        PrimaryState::Initial,
        PrimaryState::Prepare,
        PrimaryState::Ready,
        PrimaryState::Set,
        PrimaryState::Penalized,
        PrimaryState::Finished,
    ] {
        let mut blackboard = blackboard(2.0, 0.0, 0.0);
        blackboard.world_state.robot.primary_state = primary_state;
        let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
        assert!(!matches!(
            assemble_motion_command(&blackboard, status).unwrap(),
            MotionCommand::Kick { .. }
        ));
        assert!(!trace_contains(&trace, "select_ball_action"));
    }
    for posture in [
        None,
        Some(Posture::Falling),
        Some(Posture::Fallen {
            is_ready_for_standup: false,
        }),
    ] {
        let mut blackboard = blackboard(2.0, 0.0, 0.0);
        blackboard.world_state.fall_detection = posture.map(|posture| FallDetection {
            posture,
            ..blackboard.world_state.fall_detection.unwrap()
        });
        let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
        assert!(!matches!(
            assemble_motion_command(&blackboard, status).unwrap(),
            MotionCommand::Kick { .. }
        ));
        assert!(!trace_contains(&trace, "select_ball_action"));
    }
    for sub_state in [
        None,
        Some(SubState::CornerKick),
        Some(SubState::DirectFreeKick),
        Some(SubState::IndirectFreeKick),
        Some(SubState::GoalKick),
        Some(SubState::ThrowIn),
        Some(SubState::PenaltyKick),
    ] {
        let mut blackboard = blackboard(2.0, 0.0, 0.0);
        let mut state = game_state();
        state.sub_state = sub_state;
        state.game_state = FilteredGameState::Playing {
            ball_is_free: false,
            kick_off: sub_state.is_none(),
        };
        blackboard.world_state.filtered_game_controller_state = Some(state);
        let (status, trace) = create_tree().tick_with_trace(&mut blackboard);
        assert!(
            !matches!(
                assemble_motion_command(&blackboard, status).unwrap(),
                MotionCommand::Kick { .. }
            ),
            "{sub_state:?}"
        );
        assert!(!trace_contains(&trace, "select_ball_action"));
    }
}

#[test]
fn fallback_direction_remains_goalward() {
    let mut blackboard = blackboard(-2.0, 0.4, 0.6);
    let transform = blackboard.world_state.robot.ground_to_field.unwrap();
    quick_kick_subtree().tick_with_trace(&mut blackboard);
    let Some(BodyMotion::Kick {
        ball_position,
        kick_direction,
        ..
    }) = blackboard.body_motion
    else {
        panic!("expected quick kick")
    };
    let target = transform * blackboard.kick_target.unwrap();
    assert_relative_eq!(target.x(), 4.5, epsilon = 1e-5);
    assert_relative_eq!(target.y(), 0.0, epsilon = 1e-5);
    assert_relative_eq!(
        kick_direction.angle(),
        Orientation2::from_vector(transform.inverse() * point![4.5, 0.0] - ball_position).angle()
    );
}
