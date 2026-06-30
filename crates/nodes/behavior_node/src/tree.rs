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
    use types::behavior_tree::NodeTrace;

    use super::create_tree;

    #[test]
    fn passive_primary_states_look_straight_ahead_and_stand() {
        let tree_layout = create_tree().static_layout_trace();
        let passive_branch = find_passive_primary_state_branch(&tree_layout)
            .expect("passive primary-state branch exists");

        let child_names = passive_branch
            .children
            .iter()
            .map(|child| child.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            child_names,
            vec!["Selection", "look_straight_ahead", "stand"]
        );
    }

    fn find_passive_primary_state_branch(trace: &NodeTrace) -> Option<&NodeTrace> {
        if trace.name == "Sequence"
            && matches!(
                trace.children.first(),
                Some(first_child) if is_passive_primary_state_selection(first_child)
            )
        {
            return Some(trace);
        }

        trace
            .children
            .iter()
            .find_map(find_passive_primary_state_branch)
    }

    fn is_passive_primary_state_selection(trace: &NodeTrace) -> bool {
        if trace.name != "Selection" {
            return false;
        }

        let expected_conditions = ["Initial", "Penalized", "Finished"];

        trace.children.len() == expected_conditions.len()
            && expected_conditions.iter().all(|expected_state| {
                trace.children.iter().any(|child| {
                    child.name.contains("is_primary_state") && child.name.contains(expected_state)
                })
            })
    }
}
