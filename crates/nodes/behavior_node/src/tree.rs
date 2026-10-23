use hsl_network_messages::SubState;
use types::{controller_input::Button, motion_type::MotionType, primary_state::PrimaryState};

use crate::{
    action,
    actions::{damping, injected_motion_command, prepare, remote_control, stand, stand_up},
    behavior_tree::Node,
    condition,
    conditions::{
        has_ball_position, hulks_is_kicking_team, is_ball_free, is_closest_to_ball,
        is_controller_connected, is_fallen, is_finishing_striker_kick, is_goalkeeper, is_kickoff,
        is_last_hulk_standing, is_primary_state, is_ready_for_stand_up, is_remote_control_enabled,
        is_remote_kick_mode, is_simple, should_do_fall_protection,
        should_yield_goalkeeper_clearance,
    },
    goalkeeper::goalkeeper_subtree,
    head::{look_around, look_at_ball_subtree, look_straight_ahead, search_for_lost_ball_subtree},
    kick::{
        execution::{challenge_opponent_ball, execute_ball_action, finish_close_goal},
        kick,
        kickoff::kickoff_subtree,
        passing::{
            is_pass_ball_arriving, is_pass_receiver, prepare_for_restart_pass,
            prepare_to_receive_pass, yield_goalkeeper_clearance,
        },
        reception::receive_rolling_ball,
        set_kick_target_beyond_ball,
        strategy::select_ball_action,
        use_kick,
    },
    negation,
    node::Blackboard,
    penalty_shootout::{is_penalty_shootout, penalty_shootout_subtree},
    replacement_goalkeeper::{
        is_active_replacement_goalkeeper, refresh_replacement_goalkeeper_role,
        replacement_goalkeeper_subtree,
    },
    search::{leuchtturm, walk_towards_search_position},
    selection, sequence,
    substates::{is_in_sub_state, is_opponent_restart_striker, is_sub_state, sub_state_subtree},
    subtree,
    switch_motion_type::switch_motion_type,
    voronoi::{calculate_voronoi_grid, calculate_voronoi_grid_without_goalkeeper},
    walk::{
        block_opponent_with_ball, recover_goal_side, walk_alternatives_subtree,
        walk_to_kickoff_pose, walk_to_voronoi_position,
    },
};

pub fn create_tree() -> Node<Blackboard> {
    let behavior = selection!(
        sequence!(
            condition!(is_primary_state, PrimaryState::Damping),
            action!(damping)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Prepare),
            switch_motion_type(
                MotionType::Prepare,
                action!(prepare),
                sequence!(action!(look_straight_ahead), action!(stand))
            )
        ),
        sequence!(condition!(should_do_fall_protection), action!(damping)),
        sequence!(
            condition!(is_primary_state, PrimaryState::Stop),
            action!(stand)
        ),
        subtree!(remote_control_subtree),
        action!(injected_motion_command),
        sequence!(
            condition!(is_primary_state, PrimaryState::Finished),
            action!(stand)
        ),
        sequence!(
            selection!(
                condition!(is_primary_state, PrimaryState::Finished),
                condition!(is_primary_state, PrimaryState::Penalized),
            ),
            action!(stand)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Initial),
            action!(look_around),
            action!(stand)
        ),
        sequence!(condition!(is_ready_for_stand_up), action!(stand_up)),
        sequence!(condition!(is_fallen), action!(prepare)),
        sequence!(
            condition!(is_primary_state, PrimaryState::Set),
            sequence!(subtree!(look_at_ball_subtree), action!(stand))
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Ready),
            subtree!(ready_subtree)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Playing),
            subtree!(playing_subtree)
        ),
        Node::Failure
    );
    sequence!(action!(refresh_replacement_goalkeeper_role), behavior)
}

fn ready_subtree() -> Node<Blackboard> {
    sequence!(action!(walk_to_kickoff_pose), action!(look_around),)
}

fn playing_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(is_kickoff),
            negation!(condition!(is_ball_free)),
            negation!(condition!(hulks_is_kicking_team)),
            subtree!(look_at_ball_subtree),
            action!(stand)
        ),
        sequence!(
            condition!(is_penalty_shootout),
            subtree!(penalty_shootout_subtree)
        ),
        subtree!(kickoff_subtree),
        sequence!(
            condition!(is_active_replacement_goalkeeper),
            subtree!(replacement_goalkeeper_subtree),
        ),
        sequence!(
            condition!(should_yield_goalkeeper_clearance),
            subtree!(look_at_ball_subtree),
            switch_motion_type(
                MotionType::Walk,
                action!(yield_goalkeeper_clearance),
                action!(stand),
            ),
        ),
        sequence!(
            selection!(condition!(is_simple), condition!(is_last_hulk_standing)),
            selection!(
                sequence!(
                    negation!(condition!(has_ball_position)),
                    subtree!(search_subtree)
                ),
                subtree!(striker_subtree)
            )
        ),
        sequence!(condition!(is_goalkeeper), subtree!(goalkeeper_subtree)),
        sequence!(
            negation!(condition!(is_in_sub_state)),
            condition!(is_finishing_striker_kick),
            subtree!(striker_subtree),
        ),
        sequence!(
            selection!(
                negation!(condition!(is_in_sub_state)),
                sequence!(
                    condition!(hulks_is_kicking_team),
                    negation!(condition!(is_sub_state, SubState::PenaltyKick)),
                ),
            ),
            condition!(is_pass_receiver),
            subtree!(look_at_ball_subtree),
            selection!(
                sequence!(condition!(is_pass_ball_arriving), subtree!(striker_subtree)),
                switch_motion_type(
                    MotionType::Walk,
                    action!(prepare_to_receive_pass),
                    action!(stand)
                ),
            ),
        ),
        sequence!(
            negation!(condition!(has_ball_position)),
            subtree!(search_subtree)
        ),
        sequence!(
            condition!(is_in_sub_state),
            negation!(condition!(hulks_is_kicking_team)),
            action!(calculate_voronoi_grid_without_goalkeeper),
            selection!(
                sequence!(
                    condition!(is_opponent_restart_striker),
                    subtree!(striker_subtree),
                ),
                subtree!(supporter_subtree),
            ),
        ),
        sequence!(
            selection!(
                sequence!(
                    condition!(hulks_is_kicking_team),
                    condition!(is_in_sub_state),
                    negation!(condition!(is_sub_state, SubState::GoalKick)),
                    action!(calculate_voronoi_grid_without_goalkeeper),
                ),
                action!(calculate_voronoi_grid),
            ),
            condition!(is_closest_to_ball),
            subtree!(striker_subtree),
        ),
        subtree!(supporter_subtree),
    )
}

pub fn search_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(search_for_lost_ball_subtree),
        switch_motion_type(
            MotionType::Walk,
            selection!(action!(walk_towards_search_position), action!(leuchtturm)),
            subtree!(walk_alternatives_subtree),
        )
    )
}

pub(crate) fn striker_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(condition!(is_in_sub_state), subtree!(sub_state_subtree),),
            sequence!(
                negation!(condition!(is_in_sub_state)),
                selection!(
                    action!(recover_goal_side),
                    action!(finish_close_goal),
                    action!(challenge_opponent_ball),
                    action!(receive_rolling_ball),
                    action!(block_opponent_with_ball),
                    sequence!(action!(select_ball_action), action!(execute_ball_action)),
                    action!(stand),
                )
            )
        )
    )
}

fn supporter_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            action!(prepare_for_restart_pass),
            action!(walk_to_voronoi_position),
            action!(stand)
        ),
    )
}

fn remote_control_subtree() -> Node<Blackboard> {
    sequence!(
        condition!(is_remote_control_enabled),
        selection!(
            sequence!(condition!(is_ready_for_stand_up), action!(stand_up)),
            sequence!(condition!(is_fallen), action!(prepare)),
            sequence!(
                condition!(is_controller_connected),
                selection!(
                    subtree!(remote_kick_subtree, Button::LeftTrigger, false),
                    subtree!(remote_kick_subtree, Button::RightTrigger, true),
                    action!(remote_control)
                )
            ),
            action!(stand)
        )
    )
}

fn remote_kick_subtree(button: Button, strong: bool) -> Node<Blackboard> {
    sequence!(
        condition!(is_remote_kick_mode, button),
        subtree!(look_at_ball_subtree),
        action!(kick),
        action!(set_kick_target_beyond_ball),
        action!(use_kick, strong),
    )
}

#[cfg(test)]
mod clearance_tests {
    use std::time::Duration;

    use geometry::circle::Circle;
    use hsl_network_messages::{PassIntent, Penalty, PlayerNumber, Team};
    use linear_algebra::{Pose2, point, vector};
    use types::{
        behavior_tree::{NodeTrace, Status},
        fall_detection::{FallDetection, Posture},
        field_dimensions::{FieldDimensions, Side},
        filtered_game_controller_state::FilteredGameControllerState,
        filtered_game_state::FilteredGameState,
        motion_command::BodyMotion,
        obstacles::Obstacle,
        parameters::{KickStrategyParameters, PassingParameters},
        rule_obstacles::RuleObstacle,
        time_wrapper::TimeWrapper,
        walking_velocity_limits::WalkingVelocityLimits,
        world_state::{BallSource, PlayerState},
    };

    use super::*;
    use crate::{kick::strategy::BallAction, node::LastBall};

    fn three_players() -> Blackboard {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                width: 9.0,
                penalty_area_length: 3.0,
                penalty_area_width: 6.0,
                ball_radius: 0.105,
                ..FieldDimensions::SPL_2025
            },
            last_motion_type: Some(MotionType::Walk),
            last_closest_to_ball: true,
            walking_velocity_limits: WalkingVelocityLimits {
                forward_velocity_limits: [-1.0, 1.0],
                lateral_velocity_limit: 0.6,
                angular_velocity_limit: 2.0,
            },
            ..Default::default()
        };
        board.world_state.now = board.world_state.now + Duration::from_secs(10);
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.player_number = PlayerNumber::Two;
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-3.6, 0.0], 0.0).as_transform());
        board.world_state.filtered_game_controller_state = Some(FilteredGameControllerState {
            game_state: FilteredGameState::Playing {
                ball_is_free: true,
                kick_off: false,
            },
            kicking_team: Some(Team::Hulks),
            ..crate::test_support::filtered_game_controller_state()
        });
        board.world_state.fall_detection = Some(FallDetection {
            time: board.world_state.now,
            valid_until: board.world_state.now + Duration::from_secs(10),
            posture: Posture::Upright,
        });
        for (number, position) in [
            (PlayerNumber::One, point![-6.6, 0.0]),
            (PlayerNumber::Two, point![-3.6, 0.0]),
            (PlayerNumber::Three, point![-1.5, -2.0]),
        ] {
            board.world_state.player_states[number] = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: Pose2::new(position, 0.0),
                    ..crate::test_support::player_state()
                },
            });
        }
        board.ball = Some(LastBall {
            position: point![-5.5, 0.0],
            velocity: vector![3.4, 0.0],
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board
            .parameters
            .goalkeeper
            .kick_away_ball_maximum_robot_distance = 1.5;
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.pursuit_lookahead = Duration::from_millis(500);
        board.parameters.ball.interception.minimum_ball_velocity = 0.3;
        board
            .parameters
            .ball
            .interception
            .minimum_ball_velocity_towards_own_half = 0.7;
        board.parameters.ball.closest_to_ball.exit_duration = Duration::from_millis(150);
        board.parameters.voronoi.grid_resolution = 0.25;
        board.parameters.voronoi.padding = 0.5;
        board.parameters.walking.path_planning.robot_radius = 0.4;
        board.parameters.walking.speed.kicking = 1.8;
        board.parameters.walking.speed.blocking = 0.8;
        board.parameters.walking.speed.velocity_fade_distance = 3.0;
        board
            .parameters
            .walking
            .walk_and_stand
            .orientation_tolerance = 0.05;
        board.parameters.kicking.target_speed = 3.4;
        board.parameters.kicking.kick_activation_distance = 1.3;
        board.parameters.kicking.approach_ball_standoff = 0.3;
        board.parameters.kicking.allow_strong_kicks = true;
        board.parameters.kicking.strong_kick_min_target_distance = 3.0;
        board.parameters.kicking.strategy = KickStrategyParameters {
            maximum_shot_distance: 12.0,
            maximum_dribble_distance: 3.0,
            obstacle_clearance: 0.05,
            minimum_shot_angle: 0.02,
            passing: PassingParameters {
                maximum_distance: 8.0,
                minimum_progress: 0.25,
                receiver_standoff: 0.5,
                target_half_width: 0.2,
                maximum_receiver_age: Duration::from_secs(3),
                ..Default::default()
            },
            ..Default::default()
        };
        board
    }

    fn succeeded(trace: &NodeTrace, name: &str) -> bool {
        (trace.name == name && trace.status == Status::Success)
            || trace.children.iter().any(|child| succeeded(child, name))
    }

    #[test]
    fn striker_pokes_reachable_opponent_ball_instead_of_waiting_for_a_corridor() {
        let tree = striker_subtree();
        let mut board = three_players();
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.ball.as_mut().unwrap().position = point![0.6, 0.0];
        board.ball.as_mut().unwrap().velocity = vector![0.1, 0.0];
        board.world_state.obstacles = vec![Obstacle::robot(point![0.85, 0.0], 0.2, 0.2)];
        board.parameters.kicking.strategy.pressure_distance = 0.8;
        board.parameters.kicking.strategy.maximum_dribble_distance = 3.0;
        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "challenge_opponent_ball"));
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick { quick: true, strong: false, soft: false,
                ball_position, ball_velocity, kick_direction, .. })
                if ball_position == point![0.6, 0.0]
                    && ball_velocity == vector![0.1, 0.0]
                    && kick_direction.angle().abs() < 1e-6
        ));
        assert!(board.ball_action.is_none());
        let mut previous = board.body_motion.clone().unwrap();
        if let BodyMotion::Kick {
            target_speed,
            strong,
            ..
        } = &mut previous
        {
            *target_speed = 0.5;
            *strong = true;
        }
        board.last_motion_command =
            Some(types::motion_command::MotionCommand::from_partial_motions(
                previous,
                board.head_motion.unwrap(),
            ));
        board.last_motion_type = Some(MotionType::Kick);
        tree.tick_with_trace(&mut board);
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                target_speed: 3.4,
                strong: false,
                quick: true,
                ..
            })
        ));
        board.last_motion_type = Some(MotionType::Walk);
        for case in 0..5 {
            let mut protected = board.clone();
            match case {
                0 => protected.ball.as_mut().unwrap().position = point![1.5, 0.0],
                1 => {
                    protected.world_state.robot.ground_to_field =
                        Some(Pose2::new(point![1.2, 0.0], std::f32::consts::PI).as_transform())
                }
                2 => {
                    protected.last_motion_type = Some(MotionType::StandUp);
                    protected.last_motion_switch_time = protected.world_state.now;
                    protected.parameters.control.allow_switch.stand_up = Duration::from_secs(1);
                }
                3 => {
                    protected
                        .world_state
                        .player_states
                        .three
                        .as_mut()
                        .unwrap()
                        .inner
                        .pose = point![0.85, 0.0].into()
                }
                _ => {
                    protected
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .sub_state = Some(SubState::ThrowIn)
                }
            }
            let (_, trace) = tree.tick_with_trace(&mut protected);
            assert!(
                !succeeded(&trace, "challenge_opponent_ball"),
                "protected case {case}"
            );
        }
    }

    #[test]
    fn striker_finishes_at_goal_mouth_even_when_the_keeper_blocks_every_corridor() {
        let tree = striker_subtree();
        let mut board = three_players();
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(point![6.0, 0.0], 0.0).as_transform());
        board.ball.as_mut().unwrap().position = point![6.6, 0.0];
        board.ball.as_mut().unwrap().velocity = vector![0.1, 0.0];
        board.world_state.obstacles = vec![Obstacle::robot(point![0.85, 0.0], 0.2, 0.2)];
        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "finish_close_goal"));
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick { quick: true, strong: true, soft: false,
                ball_velocity, target_speed: 3.4, .. })
                if ball_velocity == vector![0.1, 0.0]
        ));
        assert_eq!(board.kick_target, Some(point![1.0, 0.0]));

        board.world_state.obstacles = vec![Obstacle::robot(point![1.0, -0.3], 0.1, 0.1)];
        tree.tick_with_trace(&mut board);
        assert!(
            board.kick_target.unwrap().y() > 0.0,
            "use the open goal side"
        );

        for case in 0..4 {
            let mut protected = board.clone();
            match case {
                0 => {
                    protected.ball.as_mut().unwrap().position = point![4.6, 0.0];
                    protected.world_state.robot.ground_to_field =
                        Some(Pose2::new(point![4.0, 0.0], 0.0).as_transform());
                }
                1 => protected.ball.as_mut().unwrap().position = point![6.6, 1.5],
                2 => {
                    protected.last_motion_type = Some(MotionType::StandUp);
                    protected.last_motion_switch_time = protected.world_state.now;
                    protected.parameters.control.allow_switch.stand_up = Duration::from_secs(1);
                }
                _ => {
                    protected
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .sub_state = Some(SubState::IndirectFreeKick);
                }
            }
            let (_, trace) = tree.tick_with_trace(&mut protected);
            assert!(
                !succeeded(&trace, "finish_close_goal"),
                "protected case {case}"
            );
        }
    }

    #[test]
    fn contested_ball_does_not_wait_for_a_pass_announcement() {
        let mut board = three_players();
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_number_of_messages = 1200;
        board.world_state.robot.ground_to_field = Some(Default::default());
        board.ball.as_mut().unwrap().position = point![0.6, 0.0];
        board.ball.as_mut().unwrap().velocity = vector![0.1, 0.0];
        board
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![3.0, -3.0], 0.0);
        board.world_state.obstacles = vec![
            Obstacle::robot(point![0.6, 0.75], 0.1, 0.1),
            Obstacle::robot(point![6.5, 0.0], 1.0, 1.0),
        ];
        board.parameters.kicking.strategy.pressure_distance = 0.8;
        assert_eq!(select_ball_action(&mut board), Status::Success);
        assert_eq!(
            board.ball_action,
            Some(BallAction::Pass {
                receiver: PlayerNumber::Three
            })
        );
        assert!(board.last_announced_pass.is_none());
        let (_, trace) = striker_subtree().tick_with_trace(&mut board);
        assert!(succeeded(&trace, "challenge_opponent_ball"));
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Kick {
                quick: true,
                soft: false,
                ..
            })
        ));
    }

    #[test]
    fn opponent_restarts_elect_one_fielder_even_when_keeper_owns_the_blocked_ball() {
        for state in [
            SubState::ThrowIn,
            SubState::IndirectFreeKick,
            SubState::DirectFreeKick,
            SubState::GoalKick,
            SubState::CornerKick,
            SubState::PenaltyKick,
        ] {
            let mut board = three_players();
            let ball = point![-5.5, 0.0];
            board.ball.as_mut().unwrap().position = ball;
            board.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
            board.last_ball = board.ball.clone();
            for (number, y) in [(PlayerNumber::Two, -1.0), (PlayerNumber::Three, 1.0)] {
                board.world_state.player_states[number]
                    .as_mut()
                    .unwrap()
                    .inner
                    .pose = point![-3.6, y].into();
            }
            let game = board
                .world_state
                .filtered_game_controller_state
                .as_mut()
                .unwrap();
            game.kicking_team = Some(Team::Opponent);
            game.sub_state = Some(state);
            let exclusion_radius = board.field_dimensions.center_circle_diameter / 2.0;
            board.world_state.rule_obstacles =
                vec![RuleObstacle::Circle(Circle::new(ball, exclusion_radius))];
            board.parameters.substates.blocking_distance_offset = 0.6;
            for number in [PlayerNumber::Two, PlayerNumber::Three] {
                let mut player = board.clone();
                player.player_number = number;
                let transform = player.world_state.player_states[number]
                    .as_ref()
                    .unwrap()
                    .inner
                    .pose
                    .as_transform();
                player.world_state.robot.ground_to_field = Some(transform);
                let (_, trace) = create_tree().tick_with_trace(&mut player);
                let elected = number == PlayerNumber::Two;
                assert_eq!(
                    succeeded(&trace, "is_opponent_restart_striker"),
                    elected,
                    "{state:?}"
                );
                assert_eq!(player.last_closest_to_ball, elected);
                assert!(!matches!(player.body_motion, Some(BodyMotion::Kick { .. })));
                if elected {
                    assert!(succeeded(&trace, "walk_to_block_position"), "{state:?}");
                    let target = transform * player.walk_position.unwrap();
                    assert!(
                        (target - ball).norm()
                            >= exclusion_radius
                                + player.parameters.walking.path_planning.robot_radius
                                - 1e-5
                    );
                    assert!(!player.path_obstacles_output.is_empty());
                }
                assert_eq!(player.world_state.rule_obstacles.len(), 1);
            }
        }
    }

    #[test]
    fn losing_team_packets_keeps_goalkeeper_defending_until_confirmed_alone() {
        let mut keeper = three_players();
        keeper.player_number = PlayerNumber::One;
        keeper.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-6.7, 0.0], 0.0).as_transform());
        keeper.world_state.player_states = types::players::Players::new(None);
        keeper.parameters.goalkeeper.x_offset = 0.3;
        keeper.parameters.goalkeeper.distance_to_goalpost = 0.15;
        keeper.ball.as_mut().unwrap().position = point![-5.5, 0.6];
        keeper.ball.as_mut().unwrap().velocity = vector![-3.0, -0.8];
        let (_, trace) = create_tree().tick_with_trace(&mut keeper);
        assert!(!succeeded(&trace, "is_last_hulk_standing"));
        assert!(succeeded(&trace, "set_goalkeeper_shot_block_position"));

        let game = keeper
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap();
        game.penalties = types::players::Players::new(Some(Penalty::PickUp {
            remaining: Duration::from_secs(30),
        }));
        game.penalties.one = None;
        assert!(is_last_hulk_standing(&mut keeper));
        keeper.world_state.filtered_game_controller_state = None;
        assert!(is_last_hulk_standing(&mut keeper));
    }

    #[test]
    fn replacement_covers_goal_instead_of_chasing_and_returns_to_field_on_recovery() {
        let tree = create_tree();
        let mut board = three_players();
        board.parameters.network.hsl_striker_message_receive_timeout = Duration::from_secs(3);
        board.parameters.network.half_duration = Duration::from_secs(600);
        board
            .parameters
            .network
            .remaining_amount_of_messages_to_stop_sending = 20;
        let game = board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap();
        game.remaining_number_of_messages = 100;
        game.remaining_time_in_half = Duration::from_secs(600);
        board.last_sent_hsl_message_time = Some(board.world_state.now);
        board.parameters.goalkeeper.x_offset = 0.3;
        board
            .world_state
            .player_states
            .one
            .as_mut()
            .unwrap()
            .inner
            .pose = point![-4.0, 1.0].into();
        board.ball.as_mut().unwrap().position = point![-3.5, 1.0];
        board.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
        tree.tick_with_trace(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.claims_role());
        assert!(board.try_sending_state_message().is_some());
        assert!(board.last_sent_replacement_goalkeeper_claim);
        board.world_state.now = board.world_state.now + Duration::from_millis(501);
        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "is_active_replacement_goalkeeper"));
        assert!(!succeeded(&trace, "is_closest_to_ball"));
        assert!(!matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
        let cover_pose = board.world_state.robot.ground_to_field.unwrap().as_pose();
        board.voronoi_inputs.clear();
        calculate_voronoi_grid(&mut board);
        assert!(!board.voronoi_inputs.contains(&cover_pose));

        let mut passer = board.clone();
        passer.player_number = PlayerNumber::Three;
        passer.world_state.robot.ground_to_field = Some(
            passer
                .world_state
                .player_states
                .three
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform(),
        );
        passer
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .replacement_goalkeeper_claim = true;
        refresh_replacement_goalkeeper_role(&mut passer);
        passer
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = Some(SubState::ThrowIn);
        passer.ball.as_mut().unwrap().position = point![-3.0, 4.5];
        assert!(crate::substates::restart_pass_receiver(&passer).is_none());
        passer
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .kicking_team = Some(Team::Opponent);
        assert!(is_opponent_restart_striker(&mut passer));
        passer
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .kicking_team = Some(Team::Hulks);
        passer.replacement_goalkeeper_coordinator = Default::default();
        assert!(crate::substates::restart_pass_receiver(&passer).is_some());

        // The covering player keeps defending even when a loose ball reaches it.
        board.ball.as_mut().unwrap().position = cover_pose.position() + vector![0.25, 0.0];
        board.body_motion = None;
        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "is_active_replacement_goalkeeper"));
        assert!(!matches!(board.body_motion, Some(BodyMotion::Kick { .. })));

        board
            .world_state
            .player_states
            .one
            .as_mut()
            .unwrap()
            .inner
            .pose = point![-6.7, 0.0].into();
        tree.tick_with_trace(&mut board);
        assert!(board.replacement_goalkeeper_coordinator.keeper().is_none());
        assert!(!board.replacement_goalkeeper_coordinator.claims_role());
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_number_of_messages = 20;
        assert!(board.try_sending_state_message().is_none());
        assert!(board.last_sent_replacement_goalkeeper_claim);
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .remaining_number_of_messages = 100;
        assert!(board.try_sending_state_message().is_some());
        assert!(!board.last_sent_replacement_goalkeeper_claim);
        board.voronoi_inputs.clear();
        calculate_voronoi_grid(&mut board);
        assert!(board.voronoi_inputs.contains(&cover_pose));
    }

    #[test]
    fn launched_pass_hands_off_striker_and_controlled_receiver_kicks_immediately() {
        let tree = create_tree();
        let mut sender = three_players();
        sender.world_state.robot.ground_to_field = Some(Default::default());
        sender
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pose = point![0.0, 0.0].into();
        sender
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pose = point![3.5, 0.0].into();
        sender.ball.as_mut().unwrap().position = point![0.5, 0.0];
        sender.ball.as_mut().unwrap().velocity = vector![2.0, 0.0];
        sender.last_motion_type = Some(MotionType::Kick);
        sender.last_motion_switch_time = sender.world_state.now - Duration::from_millis(500);
        sender.ball_action = Some(BallAction::Pass {
            receiver: PlayerNumber::Three,
        });
        sender.last_announced_pass = Some(TimeWrapper {
            time: sender.world_state.now - Duration::from_millis(200),
            inner: PassIntent {
                receiver: PlayerNumber::Three,
                target: point![3.0, 0.0],
            },
        });
        sender.last_sent_hsl_message_time = Some(sender.last_announced_pass.as_ref().unwrap().time);
        let (_, trace) = tree.tick_with_trace(&mut sender);
        assert!(succeeded(&trace, "walk_to_voronoi_position"));
        assert!(!succeeded(&trace, "is_finishing_striker_kick"));
        assert!(!sender.last_closest_to_ball);
        assert!(!matches!(sender.body_motion, Some(BodyMotion::Kick { .. })));

        let mut receiver = sender.clone();
        receiver.player_number = PlayerNumber::Three;
        receiver.world_state.robot.ground_to_field =
            Some(Pose2::new(point![3.5, 0.0], std::f32::consts::PI).as_transform());
        receiver
            .world_state
            .player_states
            .two
            .as_mut()
            .unwrap()
            .inner
            .pass = Some(sender.last_announced_pass.as_ref().unwrap().inner);
        receiver.last_announced_pass = None;
        receiver.last_motion_type = Some(MotionType::Walk);
        receiver.ball_action = None;
        receiver.ball.as_mut().unwrap().position = point![3.2, 0.0];
        receiver.ball.as_mut().unwrap().velocity = vector![0.45, 0.0];
        let (_, trace) = tree.tick_with_trace(&mut receiver);
        assert!(succeeded(&trace, "is_pass_ball_arriving"));
        assert!(receiver.last_closest_to_ball);
        assert!(!succeeded(&trace, "receive_rolling_ball"));
        assert!(succeeded(&trace, "execute_ball_action"));
        assert!(!matches!(
            receiver.body_motion,
            Some(BodyMotion::Kick { .. })
        ));

        let mut aligned = receiver.clone();
        aligned.world_state.robot.ground_to_field =
            Some(Pose2::new(point![3.1, 0.0], 0.0).as_transform());
        aligned.ball.as_mut().unwrap().position = point![3.4, 0.0];
        aligned.ball.as_mut().unwrap().velocity = vector![-0.45, 0.0];
        aligned.ball_action = None;
        tree.tick_with_trace(&mut aligned);
        assert!(matches!(
            aligned.ball_action,
            Some(BallAction::Shoot { .. })
        ));
        assert!(matches!(
            aligned.body_motion,
            Some(BodyMotion::Kick { quick: true, .. })
        ));

        // A receiver facing the sender approaches behind the stopped ball before kicking.
        receiver.ball.as_mut().unwrap().velocity = vector![0.2, 0.0];
        tree.tick_with_trace(&mut receiver);
        assert!(matches!(
            receiver.body_motion,
            Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
        ));
    }

    #[test]
    fn three_players_clear_then_yield_even_after_projected_possession_claim() {
        let tree = create_tree();
        let mut keeper = three_players();
        for y in [-1e-7, 1e-7] {
            assert_eq!(
                crate::goalkeeper::goalkeeper_clearance_target(
                    point![-6.0, y],
                    &keeper.field_dimensions
                )
                .y(),
                y
            );
        }
        assert!(
            crate::goalkeeper::goalkeeper_clearance_target(
                point![-6.0, -0.5],
                &keeper.field_dimensions
            )
            .y() < 0.0
        );
        keeper.player_number = PlayerNumber::One;
        let keeper_transform = keeper
            .world_state
            .player_states
            .one
            .as_ref()
            .unwrap()
            .inner
            .pose
            .as_transform();
        keeper.world_state.robot.ground_to_field = Some(keeper_transform);
        keeper.ball.as_mut().unwrap().position = point![-6.0, 0.0];
        keeper.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
        keeper.world_state.obstacles = vec![Obstacle::robot(
            keeper_transform.inverse() * point![-4.0, 1.0],
            0.2,
            0.2,
        )];
        tree.tick_with_trace(&mut keeper);
        assert!(matches!(
            keeper.body_motion,
            Some(BodyMotion::Kick {
                strong: true,
                soft: false,
                ..
            })
        ));

        // A previous trapping kick must not keep a subsequent long clearance weak.
        let mut previous = keeper.body_motion.clone().unwrap();
        if let BodyMotion::Kick { strong, .. } = &mut previous {
            *strong = false;
        }
        keeper.last_motion_type = Some(MotionType::Kick);
        keeper.last_motion_command =
            Some(types::motion_command::MotionCommand::from_partial_motions(
                previous,
                keeper.head_motion.unwrap(),
            ));
        tree.tick_with_trace(&mut keeper);
        assert!(matches!(
            keeper.body_motion,
            Some(BodyMotion::Kick { strong: true, .. })
        ));

        // Prepare before launch, while the keeper winds up and support could walk into its lane.
        for number in [PlayerNumber::Two, PlayerNumber::Three] {
            let mut preparing = three_players();
            preparing.ball.as_mut().unwrap().position = point![-6.0, 0.0];
            preparing.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
            preparing
                .world_state
                .player_states
                .two
                .as_mut()
                .unwrap()
                .inner
                .pose = Pose2::new(point![-3.8, 0.2], 0.0);
            let transform = preparing.world_state.player_states[number]
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform();
            preparing.player_number = number;
            preparing.world_state.robot.ground_to_field = Some(transform);
            preparing.world_state.obstacles = vec![Obstacle::robot(
                transform.inverse() * point![-2.5, 2.5],
                0.2,
                0.2,
            )];
            let (_, trace) = tree.tick_with_trace(&mut preparing);
            assert!(succeeded(&trace, "yield_goalkeeper_clearance"));
            assert!(preparing.ball_action.is_none());
            assert!(preparing.kick_target.is_none());
            let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = preparing.body_motion else {
                panic!("field players must prepare the clearance lane before launch");
            };
            if number == PlayerNumber::Two {
                assert!(velocity.norm() > 0.0);
                let direction =
                    crate::conditions::goalkeeper_clearance_direction(&preparing).unwrap();
                assert!((transform * velocity).dot(&direction).abs() < 1e-6);
                for case in 0..3 {
                    let mut ordinary = preparing.clone();
                    match case {
                        0 => ordinary.world_state.obstacles.clear(),
                        1 => {
                            ordinary
                                .world_state
                                .player_states
                                .three
                                .as_mut()
                                .unwrap()
                                .inner
                                .pose = Pose2::new(point![-5.7, 0.1], 0.0)
                        }
                        _ => {
                            ordinary.ball.as_mut().unwrap().position = point![-4.0, 1.0];
                            ordinary.ball.as_mut().unwrap().velocity = vector![0.5, 0.0];
                        }
                    }
                    let (_, trace) = tree.tick_with_trace(&mut ordinary);
                    assert_eq!(
                        succeeded(&trace, "yield_goalkeeper_clearance"),
                        case != 2,
                        "keeper retains clearance priority without pressure or despite closer fielders: {case}"
                    );
                }
            } else {
                assert_eq!(velocity.norm(), 0.0);
            }
        }

        let mut supporter = three_players();
        calculate_voronoi_grid(&mut supporter);
        assert!(is_closest_to_ball(&mut supporter));
        supporter.ball_action = Some(BallAction::Dribble);
        supporter.kick_target = Some(point![1.0, 0.0]);
        let (_, trace) = tree.tick_with_trace(&mut supporter);
        assert!(succeeded(&trace, "yield_goalkeeper_clearance"));
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = supporter.body_motion else {
            panic!("supporter must leave clearance lane");
        };
        assert!(velocity.y() >= 0.6);
        assert!(velocity.x().abs() < 1e-6);
        assert!(supporter.ball_action.is_none());
        assert!(supporter.kick_target.is_none());
        assert!(!supporter.last_closest_to_ball);

        // A real strong clearance slows below half its requested speed before passing support.
        for speed in [1.1, 0.5] {
            supporter.ball.as_mut().unwrap().velocity = vector![speed, 0.0];
            let (_, trace) = tree.tick_with_trace(&mut supporter);
            assert!(succeeded(&trace, "yield_goalkeeper_clearance"));
        }

        // The third robot is already clear and must not chase the projected ball.
        let mut third = three_players();
        third.player_number = PlayerNumber::Three;
        third.world_state.robot.ground_to_field = Some(
            third
                .world_state
                .player_states
                .three
                .as_ref()
                .unwrap()
                .inner
                .pose
                .as_transform(),
        );
        let (_, trace) = tree.tick_with_trace(&mut third);
        assert!(succeeded(&trace, "yield_goalkeeper_clearance"));
        assert!(
            matches!(third.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity.norm() == 0.0)
        );

        supporter.ball.as_mut().unwrap().position = point![-3.0, 0.0];
        let (_, trace) = tree.tick_with_trace(&mut supporter);
        assert!(!succeeded(&trace, "yield_goalkeeper_clearance"));
    }

    #[test]
    fn clearance_yield_preserves_pass_reception_and_incoming_shot_defense() {
        let tree = create_tree();
        let mut receiver = three_players();
        receiver
            .world_state
            .player_states
            .one
            .as_mut()
            .unwrap()
            .inner
            .pass = Some(PassIntent {
            receiver: PlayerNumber::Two,
            target: point![-4.1, 0.0],
        });
        let (_, trace) = tree.tick_with_trace(&mut receiver);
        assert!(!succeeded(&trace, "yield_goalkeeper_clearance"));
        assert!(succeeded(&trace, "is_pass_receiver"));
        assert!(succeeded(&trace, "receive_rolling_ball"));

        let mut field_pass = three_players();
        field_pass
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![-6.0, 0.0], 0.0);
        field_pass
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pass = Some(PassIntent {
            receiver: PlayerNumber::Two,
            target: point![-4.1, 0.0],
        });
        let (_, trace) = tree.tick_with_trace(&mut field_pass);
        assert!(!succeeded(&trace, "yield_goalkeeper_clearance"));
        assert!(succeeded(&trace, "is_pass_receiver"));
        assert!(succeeded(&trace, "receive_rolling_ball"));
        field_pass
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![-6.0, 1.5], 0.0);
        let (_, trace) = tree.tick_with_trace(&mut field_pass);
        assert!(!succeeded(&trace, "yield_goalkeeper_clearance"));
        assert!(succeeded(&trace, "receive_rolling_ball"));
        field_pass
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .time = field_pass.world_state.now - PassIntent::TIMEOUT;
        let (_, trace) = tree.tick_with_trace(&mut field_pass);
        assert!(succeeded(&trace, "yield_goalkeeper_clearance"));

        let mut defender = three_players();
        defender.ball.as_mut().unwrap().position = point![-1.5, 0.0];
        defender.ball.as_mut().unwrap().velocity = vector![-3.4, 0.0];
        let (_, trace) = tree.tick_with_trace(&mut defender);
        assert!(!succeeded(&trace, "yield_goalkeeper_clearance"));
        assert!(succeeded(&trace, "receive_rolling_ball"));
    }

    #[test]
    fn clearance_yield_releases_for_uncertain_origin_or_changed_ball() {
        let tree = create_tree();
        for case in 0..13 {
            let mut board = three_players();
            match case {
                0 => {
                    board.ball.as_mut().unwrap().position = point![-4.9, 0.0];
                    board.ball.as_mut().unwrap().velocity = vector![0.0, 0.0];
                }
                1 => board.ball.as_mut().unwrap().velocity = vector![0.0, 3.4],
                2 => board.ball.as_mut().unwrap().velocity = vector![-3.4, 0.0],
                3 => board.ball.as_mut().unwrap().age = Default::default(),
                4 => {
                    board.world_state.player_states.one.as_mut().unwrap().time = Default::default()
                }
                5 => {
                    board.world_state.player_states.one.as_mut().unwrap().time =
                        board.world_state.now + Duration::from_secs(1)
                }
                6 => {
                    board
                        .world_state
                        .player_states
                        .one
                        .as_mut()
                        .unwrap()
                        .inner
                        .pose = Pose2::new(point![-3.0, 0.0], 0.0)
                }
                7 => {
                    board
                        .world_state
                        .player_states
                        .one
                        .as_mut()
                        .unwrap()
                        .inner
                        .pose = Pose2::new(point![-6.6, 1.0], 0.0)
                }
                8 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .penalties
                        .one = Some(Penalty::PickUp {
                        remaining: Duration::from_secs(30),
                    })
                }
                9 => board.last_motion_type = Some(MotionType::Kick),
                10 => {
                    board
                        .world_state
                        .filtered_game_controller_state
                        .as_mut()
                        .unwrap()
                        .sub_state = Some(SubState::GoalKick)
                }
                11 => board.ball.as_mut().unwrap().position = point![0.1, 0.0],
                _ => {
                    board.last_motion_type = Some(MotionType::StandUp);
                    board.last_motion_switch_time = board.world_state.now;
                    board.parameters.control.allow_switch.stand_up = Duration::from_secs(1);
                }
            }
            let (_, trace) = tree.tick_with_trace(&mut board);
            assert!(
                !succeeded(&trace, "yield_goalkeeper_clearance"),
                "case {case}"
            );
            if case == 12 {
                assert!(succeeded(&trace, "should_yield_goalkeeper_clearance"));
                assert_eq!(board.body_motion, Some(BodyMotion::Stand));
            }
        }
    }

    #[test]
    fn clearance_step_respects_teammate_bodies_rules_and_current_side() {
        let tree = create_tree();
        let mut rotated = three_players();
        rotated.walking_velocity_limits.lateral_velocity_limit = 1.5;
        let transform = Pose2::new(point![-3.6, 0.0], std::f32::consts::FRAC_PI_4).as_transform();
        rotated.world_state.robot.ground_to_field = Some(transform);
        tree.tick_with_trace(&mut rotated);
        let Some(BodyMotion::WalkWithVelocity { velocity, .. }) = rotated.body_motion else {
            panic!("rotated supporter must leave clearance lane");
        };
        let field_velocity = transform * velocity;
        assert!(field_velocity.x().abs() < 1e-6);
        assert!(field_velocity.y() > 0.0);
        assert!(velocity.x() <= rotated.walking_velocity_limits.forward_velocity_limits[1]);
        assert!(velocity.y().abs() <= 1.0);

        let mut board = three_players();
        board
            .world_state
            .player_states
            .three
            .as_mut()
            .unwrap()
            .inner
            .pose = Pose2::new(point![-3.6, 0.9], 0.0);
        tree.tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity.y() < 0.0)
        );

        board.world_state.rule_obstacles =
            vec![RuleObstacle::Circle(Circle::new(point![-3.6, -0.8], 0.3))];
        tree.tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity.norm() == 0.0)
        );

        // Already above the line: a blocked exit must not send us through the ball lane.
        board.world_state.rule_obstacles.clear();
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-3.6, 0.1], 0.0).as_transform());
        tree.tick_with_trace(&mut board);
        assert!(
            matches!(board.body_motion, Some(BodyMotion::WalkWithVelocity { velocity, .. }) if velocity.norm() == 0.0)
        );
    }
}
