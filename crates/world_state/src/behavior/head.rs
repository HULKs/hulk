use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion},
};

use crate::{
    action,
    behavior::{behavior_tree::Node, condition::has_new_ball_position, node::Blackboard},
    condition, selection, sequence,
};

pub fn look_at_ball_subtree() -> Node<Blackboard> {
    selection!(
        sequence!(condition!(has_new_ball_position), action!(look_at_ball)),
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
