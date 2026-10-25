use hsl_network_messages::SubState;
use types::{controller_input::Button, motion_type::MotionType, primary_state::PrimaryState};

use crate::{
    action,
    actions::{
        damping, injected_motion_command, prepare, remote_control, stand, stand_up,
        walk_out_off_rule_obstacles,
    },
    behavior_tree::Node,
    condition,
    conditions::{
        has_ball_position, has_more_than_unpenalized_players, hulks_is_kicking_team,
        is_active_replacement_goalkeeper, is_ball_free, is_ball_interception_candidate,
        is_close_to_ball, is_closest_to_ball, is_controller_connected, is_fallen, is_goalkeeper,
        is_inside_rule_obstacles, is_kickoff, is_last_hulk_standing, is_primary_state,
        is_ready_for_stand_up, is_remote_control_enabled, is_remote_kick_mode, is_simple,
        is_stand_up_allowed, is_standing_up, should_do_fall_protection,
    },
    goalkeeper::goalkeeper_subtree,
    head::{look_around, look_at_ball_subtree, look_straight_ahead, search_for_lost_ball_subtree},
    kick::{
        execution::{execute_ball_action, finish_close_goal},
        intercept, kick, kick_alternatives_subtree, kick_subtree,
        kickoff::kickoff_subtree,
        kickoff_kick,
        passing::{
            is_finishing_restart_pass, is_pass_ball_arriving, is_pass_receiver,
            outgoing_pass_in_flight, prepare_for_restart_pass, prepare_to_receive_pass,
        },
        quick_kick_subtree,
        reception::receive_rolling_ball,
        set_kick_target_beyond_ball,
        strategy::{
            BallAction, QuickKickReason, begin_ball_action_cycle, is_quick_kick_selected,
            select_ball_action,
        },
        use_kick,
    },
    negation,
    node::Blackboard,
    penalty_shootout::{is_penalty_shootout, penalty_shootout_subtree},
    replacement_goalkeeper::{
        refresh_replacement_goalkeeper_election_status, refresh_replacement_goalkeeper_role,
        update_replacement_goalkeeper_election,
    },
    search::{leuchtturm, walk_towards_search_position},
    selection, sequence,
    substates::{is_in_sub_state, is_sub_state, sub_state_subtree},
    subtree,
    switch_motion_type::switch_motion_type,
    voronoi::{calculate_voronoi_grid, calculate_voronoi_grid_without_goalkeeper},
    walk::{
        walk_alternatives_subtree, walk_to_ball_subtree, walk_to_kickoff_pose,
        walk_to_voronoi_position,
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
        subtree!(fall_subtree),
        sequence!(
            condition!(is_primary_state, PrimaryState::Stop),
            action!(stand)
        ),
        subtree!(remote_control_subtree),
        action!(injected_motion_command),
        sequence!(
            selection!(
                condition!(is_primary_state, PrimaryState::Finished), // Why Finished again?!
                condition!(is_primary_state, PrimaryState::Penalized),
            ),
            action!(stand)
        ),
        sequence!(
            condition!(is_primary_state, PrimaryState::Initial),
            action!(look_around),
            action!(stand)
        ),
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

    sequence!(
        action!(begin_ball_action_cycle),
        action!(refresh_replacement_goalkeeper_role), // to refresh coordinator every loop
        behavior,
    )
}

fn fall_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(condition!(should_do_fall_protection), action!(damping)),
        sequence!(condition!(is_standing_up), action!(stand_up)),
        sequence!(
            condition!(is_fallen),
            selection!(
                sequence!(
                    condition!(is_stand_up_allowed),
                    selection!(
                        sequence!(condition!(is_ready_for_stand_up), action!(stand_up)),
                        action!(prepare)
                    )
                ),
                action!(damping)
            )
        )
    )
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
        sequence!(
            condition!(is_inside_rule_obstacles),
            action!(walk_out_off_rule_obstacles)
        ),
        subtree!(kickoff_subtree),
        sequence!(condition!(is_restart_pass_in_flight), action!(stand)),
        sequence!(
            condition!(is_finishing_restart_pass),
            action!(execute_ball_action),
        ),
        sequence!(
            negation!(condition!(is_goalkeeper)),
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
            condition!(is_active_replacement_goalkeeper),
            subtree!(goalkeeper_subtree)
        ),
        sequence!(
            negation!(condition!(has_ball_position)),
            selection!(
                subtree!(replacement_goalkeeper_decision_subtree),
                subtree!(search_subtree)
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
            action!(update_replacement_goalkeeper_election, false),
            subtree!(striker_subtree),
        ),
        subtree!(replacement_goalkeeper_decision_subtree),
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
                action!(select_ball_action),
                selection!(
                    sequence!(
                        condition!(is_close_to_ball),
                        condition!(is_kickoff),
                        condition!(hulks_is_kicking_team),
                        condition!(has_more_than_unpenalized_players, 2),
                        switch_motion_type(
                            MotionType::Kick,
                            action!(kickoff_kick),
                            subtree!(kick_alternatives_subtree),
                        )
                    ),
                    action!(finish_close_goal),
                    sequence!(
                        negation!(condition!(is_pass_receiver)),
                        condition!(is_urgent_quick_kick_selected),
                        subtree!(quick_kick_subtree)
                    ),
                    action!(receive_rolling_ball),
                    action!(execute_ball_action),
                    sequence!(
                        condition!(is_quick_kick_selected),
                        subtree!(quick_kick_subtree)
                    ),
                    sequence!(
                        condition!(is_close_to_ball),
                        selection!(
                            sequence!(
                                condition!(is_ball_interception_candidate),
                                subtree!(kick_subtree),
                                selection!(condition!(is_quick_kick_selected), action!(intercept)),
                            ),
                            subtree!(kick_subtree)
                        )
                    ),
                    subtree!(walk_to_ball_subtree)
                )
            ),
            action!(stand)
        )
    )
}

fn is_urgent_quick_kick_selected(blackboard: &mut Blackboard) -> bool {
    matches!(
        blackboard.ball_action,
        Some(BallAction::QuickKick {
            reason: QuickKickReason::NearOwnGoal
                | QuickKickReason::NearOpponentGoal
                | QuickKickReason::NearbyRobot,
        })
    )
}

fn is_restart_pass_in_flight(blackboard: &mut Blackboard) -> bool {
    outgoing_pass_in_flight(blackboard)
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

fn replacement_goalkeeper_decision_subtree() -> Node<Blackboard> {
    sequence!(
        condition!(refresh_replacement_goalkeeper_election_status),
        action!(update_replacement_goalkeeper_election, true),
        condition!(is_active_replacement_goalkeeper),
        subtree!(goalkeeper_subtree),
    )
}

#[cfg(test)]
mod extraction_tests {
    use std::time::Duration;

    use hsl_network_messages::{PlayerNumber, SubState, Team};
    use linear_algebra::{Pose2, point, vector};
    use ros_z::time::Time;
    use types::{
        behavior_tree::{NodeTrace, Status},
        fall_detection::{FallDetection, Posture},
        field_dimensions::{FieldDimensions, Side},
        filtered_game_state::FilteredGameState,
        motion_command::BodyMotion,
        time_wrapper::TimeWrapper,
        world_state::{BallSource, PlayerState},
    };

    use crate::{motion_assembler::assemble_motion_command, node::LastBall};

    use super::*;

    fn board() -> Blackboard {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions {
                length: 14.0,
                width: 9.0,
                ..FieldDimensions::SPL_2025
            },
            parameters: json5::from_str(include_str!(
                "../../../../etc/parameters/base/behavior_node.json5"
            ))
            .unwrap(),
            player_number: PlayerNumber::Two,
            last_motion_type: Some(MotionType::Walk),
            ..Default::default()
        };
        board.parameters.control.is_simple = true;
        board.parameters.control.allow_switch = Default::default();
        board.parameters.goalkeeper.player_number = PlayerNumber::One;
        board.world_state.now = Time::from_nanos(100_000_000_000);
        board.world_state.robot.primary_state = PrimaryState::Playing;
        board.world_state.robot.ground_to_field =
            Some(Pose2::new(point![-1.6, 0.0], 0.0).as_transform());
        board.world_state.fall_detection = Some(FallDetection {
            time: board.world_state.now,
            valid_until: board.world_state.now + Duration::from_secs(2),
            posture: Posture::Upright,
        });
        let mut game = crate::test_support::filtered_game_controller_state();
        game.game_state = FilteredGameState::Playing {
            ball_is_free: true,
            kick_off: false,
        };
        game.kicking_team = Some(Team::Hulks);
        game.sub_state = Some(SubState::DirectFreeKick);
        game.remaining_time_in_half = Duration::from_secs(600);
        game.remaining_number_of_messages = 1200;
        board.world_state.filtered_game_controller_state = Some(game);
        for (number, position) in [
            (PlayerNumber::One, point![-6.7, 0.0]),
            (PlayerNumber::Two, point![-1.6, 0.0]),
            (PlayerNumber::Three, point![4.0, 1.0]),
        ] {
            board.world_state.player_states[number] = Some(TimeWrapper {
                time: board.world_state.now,
                inner: PlayerState {
                    pose: position.into(),
                    ..crate::test_support::player_state()
                },
            });
        }
        set_ball(&mut board, point![-1.0, 0.0], vector![0.0, 0.0]);
        board
    }

    fn set_ball(
        board: &mut Blackboard,
        position: linear_algebra::Point2<coordinate_systems::Field>,
        velocity: linear_algebra::Vector2<coordinate_systems::Field>,
    ) {
        board.ball = Some(LastBall {
            position,
            velocity,
            age: board.world_state.now,
            field_side: Side::Left,
            source: BallSource::Own,
        });
        board.last_ball = board.ball.clone();
    }

    fn succeeded(trace: &NodeTrace, name: &str) -> bool {
        (trace.name == name && trace.status == Status::Success)
            || trace.children.iter().any(|child| succeeded(child, name))
    }

    #[test]
    fn rolling_interception_precedes_the_unplanned_quick_kick_fallback() {
        let mut board = board();
        board.world_state.robot.ground_to_field = Some(Default::default());
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = None;
        set_ball(&mut board, point![2.0, 0.4], vector![-1.0, 0.0]);

        let (_, trace) = create_tree().tick_with_trace(&mut board);

        assert!(succeeded(&trace, "receive_rolling_ball"));
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
        ));
    }

    #[test]
    fn addressed_receiver_traps_the_arrival_while_the_restart_is_still_active() {
        let mut board = board();
        board.player_number = PlayerNumber::Three;
        let target = point![3.5_f32, 1.0];
        let direction = (target - point![-1.0, 0.0]).normalize();
        board.world_state.robot.ground_to_field = Some(
            Pose2::new(point![4.0, 1.0], (-direction.y()).atan2(-direction.x())).as_transform(),
        );
        let sender = board.world_state.player_states.two.as_mut().unwrap();
        sender.inner.pass = Some(hsl_network_messages::PassIntent {
            receiver: PlayerNumber::Three,
            target,
        });
        set_ball(&mut board, target - direction, direction);

        let (_, trace) = create_tree().tick_with_trace(&mut board);

        assert!(succeeded(&trace, "is_pass_receiver"));
        assert!(succeeded(&trace, "receive_rolling_ball"));
        assert!(board.ball_action.is_none());
        assert!(matches!(
            board.body_motion,
            Some(BodyMotion::Walk { .. } | BodyMotion::WalkWithVelocity { .. })
        ));
    }

    #[test]
    fn announced_restart_pass_survives_restart_end_and_releases_the_passer_on_launch() {
        let tree = create_tree();
        let mut board = board();
        tree.tick_with_trace(&mut board);
        assert!(matches!(board.ball_action, Some(BallAction::Pass { .. })));
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert!(board.try_sending_state_message().is_some());

        board.world_state.now = board.world_state.now + Duration::from_millis(200);
        board.last_motion_type = Some(MotionType::Stand);
        tree.tick_with_trace(&mut board);
        assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));
        board.last_motion_command = Some(assemble_motion_command(&board, Status::Success).unwrap());
        board.last_motion_type = Some(MotionType::Kick);
        board
            .world_state
            .filtered_game_controller_state
            .as_mut()
            .unwrap()
            .sub_state = None;

        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "is_finishing_restart_pass"));
        assert!(matches!(board.body_motion, Some(BodyMotion::Kick { .. })));

        let origin = point![-1.0, 0.0];
        let direction =
            (board.last_announced_pass.as_ref().unwrap().inner.target - origin).normalize();
        set_ball(&mut board, origin + direction, direction * 2.0);
        let (_, trace) = tree.tick_with_trace(&mut board);
        assert!(succeeded(&trace, "is_restart_pass_in_flight"));
        assert_eq!(board.body_motion, Some(BodyMotion::Stand));
        assert!(!crate::conditions::is_closest_to_ball(&mut board));
    }
}
