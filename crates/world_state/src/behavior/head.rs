use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion},
};

use crate::{
    action,
    behavior::{
        behavior_tree::Node,
        condition::{has_hypothetical_ball_position, has_new_ball_position},
        node::Blackboard,
    },
    condition, selection, sequence, subtree,
};

pub fn look_at_ball_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(condition!(has_new_ball_position), action!(look_at_ball)),
        subtree!(search_for_lost_ball_subtree)
    )
}

pub fn search_for_lost_ball_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(
            condition!(has_hypothetical_ball_position),
            action!(look_at_hypothetical_ball_position)
        ),
        action!(search_for_lost_ball)
    )
}

pub fn look_at_ball(blackboard: &mut Blackboard) -> Status {
    if let Some(ball) = &blackboard.world_state.ball {
        blackboard.head_motion = Some(HeadMotion::LookAt {
            target: ball.ball_in_ground,
            image_region_target: ImageRegion::Center,
        });
        Status::Success
    } else {
        Status::Failure
    }
}

pub fn search_for_lost_ball(blackboard: &mut Blackboard) -> Status {
    blackboard.head_motion = Some(HeadMotion::SearchForLostBall);
    Status::Success
}

pub fn look_straight_ahead(blackboard: &mut Blackboard) -> Status {
    blackboard.head_motion = Some(HeadMotion::Center {
        image_region_target: ImageRegion::Center,
    });
    Status::Success
}

pub fn look_at_hypothetical_ball_position(blackboard: &mut Blackboard) -> Status {
    let best_hypothetical_ball_position = blackboard
        .world_state
        .hypothetical_ball_positions
        .iter()
        .max_by(|a, b| a.validity.total_cmp(&b.validity));
    if let Some(hypothesis) = best_hypothetical_ball_position {
        blackboard.head_motion = Some(HeadMotion::LookAt {
            target: hypothesis.position,
            image_region_target: ImageRegion::Center,
        });
        Status::Success
    } else {
        Status::Failure
    }
}
