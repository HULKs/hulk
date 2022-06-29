use crate::types::{BallState, HeadMotion};

pub fn look_for_ball(ball_state: Option<BallState>) -> HeadMotion {
    match ball_state {
        Some(ball) => HeadMotion::LookAt {
            target: ball.position,
        },
        None => HeadMotion::LookAround,
    }
}
