use types::{motion_type::MotionType, primary_state::PrimaryState};

use crate::{
    action,
    actions::{damping, injected_motion_command, prepare, remote_control, stand, stand_up},
    behavior_tree::Node,
    condition,
    conditions::{
        has_ball_position, is_ball_interception_candidate, is_close_to_ball, is_closest_to_ball,
        is_fallen, is_goalkeeper, is_primary_state, is_remote_controlled, is_remote_kick_mode,
    },
    head::{look_at_ball_subtree, look_straight_ahead, search_for_lost_ball_subtree},
    kick::{intercept, kick, kick_power_subtree, kick_subtree, set_kick_target_in_front},
    negation,
    node::Blackboard,
    search::{has_suggested_search_position, leuchtturm, walk_to_search_position},
    selection, sequence,
    substates::{is_in_sub_state, sub_state_subtree},
    subtree,
    switch_motion_type::switch_motion_type,
    voronoi::calculate_voronoi_grid,
    walk::{
        walk_alternatives_subtree, walk_to_ball_subtree, walk_to_centroid, walk_to_kickoff_pose,
    },
};

pub fn create_tree() -> Node<Blackboard> {
    selection!(
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
        sequence!(
            condition!(is_primary_state, PrimaryState::Stop),
            action!(stand)
        ),
        subtree!(remote_control_subtree),
        action!(injected_motion_command),
        sequence!(
            selection!(
                condition!(is_primary_state, PrimaryState::Initial),
                condition!(is_primary_state, PrimaryState::Penalized),
                condition!(is_primary_state, PrimaryState::Finished)
            ),
            action!(look_straight_ahead),
            action!(stand)
        ),
        sequence!(condition!(is_fallen), action!(stand_up)),
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
    )
}

fn ready_subtree() -> Node<Blackboard> {
    sequence!(action!(walk_to_kickoff_pose))
}

fn playing_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(condition!(is_goalkeeper), subtree!(goalkeeper_subtree)),
        sequence!(
            negation!(condition!(has_ball_position)),
            subtree!(search_subtree)
        ),
        sequence!(
            action!(calculate_voronoi_grid),
            condition!(is_closest_to_ball),
            subtree!(striker_subtree)
        ),
        subtree!(supporter_subtree),
    )
}

fn goalkeeper_subtree() -> Node<Blackboard> {
    sequence!(subtree!(look_at_ball_subtree), action!(stand))
}

pub fn search_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(search_for_lost_ball_subtree),
        switch_motion_type(
            MotionType::Walk,
            selection!(
                sequence!(
                    condition!(has_suggested_search_position),
                    action!(walk_to_search_position)
                ),
                action!(leuchtturm)
            ),
            subtree!(walk_alternatives_subtree),
        )
    )
}

fn striker_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(
            sequence!(condition!(is_in_sub_state), subtree!(sub_state_subtree),),
            sequence!(
                negation!(condition!(is_close_to_ball)),
                subtree!(walk_to_ball_subtree)
            ),
            sequence!(
                condition!(is_ball_interception_candidate),
                subtree!(kick_subtree),
                action!(intercept),
            ),
            subtree!(kick_subtree)
        )
    )
}

fn supporter_subtree() -> Node<Blackboard> {
    sequence!(
        subtree!(look_at_ball_subtree),
        selection!(action!(walk_to_centroid), action!(stand)),
    )
}

fn remote_control_subtree() -> Node<Blackboard> {
    sequence!(
        condition!(is_remote_controlled),
        selection!(
            sequence!(
                condition!(is_remote_kick_mode),
                subtree!(look_at_ball_subtree),
                sequence!(
                    action!(kick),
                    action!(set_kick_target_in_front),
                    subtree!(kick_power_subtree),
                )
            ),
            sequence!(action!(look_straight_ahead), action!(remote_control))
        )
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ros_z::time::Time;
    use types::{
        behavior_tree::Status,
        field_dimensions::FieldDimensions,
        motion_command::{BodyMotion, HeadMotion, ImageRegion, MotionCommand},
        parameters::BehaviorParameters,
        primary_state::PrimaryState,
        world_state::WorldState,
    };

    use super::create_tree;
    use crate::node::Blackboard;

    #[test]
    fn passive_primary_states_look_straight_ahead_and_stand() {
        for primary_state in [
            PrimaryState::Initial,
            PrimaryState::Penalized,
            PrimaryState::Finished,
        ] {
            let mut blackboard = blackboard_with_primary_state(primary_state);

            let (status, _) = create_tree().tick_with_trace(&mut blackboard);

            assert_eq!(status, Status::Success);
            assert_eq!(blackboard.body_motion, Some(BodyMotion::Stand));
            assert_eq!(
                blackboard.head_motion,
                Some(HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                })
            );
        }
    }

    fn blackboard_with_primary_state(primary_state: PrimaryState) -> Blackboard {
        Blackboard {
            field_dimensions: FieldDimensions::default(),
            parameters: BehaviorParameters::default(),
            world_state: WorldState {
                robot: types::world_state::RobotState {
                    primary_state,
                    ..Default::default()
                },
                ..Default::default()
            },
            path_obstacles_output: Vec::new(),
            time_since_last_switch: Duration::ZERO,
            direction_difference: 0.0,
            voronoi_inputs: Vec::new(),
            ball: None,
            last_ball: None,
            last_close_enough_to_kick: false,
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
}
