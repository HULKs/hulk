use std::time::Duration;

use types::{
    behavior_tree::Status,
    motion_command::{HeadMotion, ImageRegion},
    world_state::BallSource,
};

use crate::{
    action,
    behavior_tree::Node,
    condition,
    conditions::has_hypothetical_ball_position,
    node::{Blackboard, ball_timeout},
    selection, sequence, subtree,
};

pub fn look_at_ball_subtree() -> Node<Blackboard> {
    selection!(
        action!(look_at_ball),
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
    let now = blackboard.world_state.now;
    let world_ball = blackboard.world_state.ball.as_ref().filter(|ball| {
        now.to_wallclock()
            .duration_since(ball.last_seen_ball)
            .is_ok_and(|age| age < ball_timeout(&blackboard.parameters, ball.source))
            && ball
                .ball_in_ground
                .inner
                .iter()
                .all(|value| value.is_finite())
    });
    let target = blackboard
        .visual_kick_ball_position
        .as_ref()
        .filter(|ball| {
            ball.age_at(now)
                .is_some_and(|age| age <= Duration::from_millis(100))
                && ball.position.inner.iter().all(|value| value.is_finite())
                && world_ball.is_none_or(|world_ball| {
                    world_ball.source != BallSource::Own
                        || ball.last_seen.to_wallclock() >= world_ball.last_seen_ball
                })
        })
        .map(|ball| ball.position)
        .or_else(|| world_ball.map(|ball| ball.ball_in_ground))
        .or_else(|| {
            // A rejected current sample must not re-enter through its refreshed cache.
            if blackboard.world_state.ball.is_some() {
                return None;
            }
            let ball = blackboard.ball.as_ref()?;
            if ball.age > now
                || now.duration_since(ball.age) >= ball_timeout(&blackboard.parameters, ball.source)
            {
                return None;
            }
            let ground_to_field = blackboard.world_state.robot.ground_to_field?;
            Some(ground_to_field.inverse() * ball.position)
        })
        .filter(|target| target.inner.iter().all(|value| value.is_finite()));
    if let Some(target) = target {
        blackboard.head_motion = Some(HeadMotion::LookAt {
            target,
            height_above_ground: blackboard.field_dimensions.ball_radius,
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

pub fn look_around(blackboard: &mut Blackboard) -> Status {
    blackboard.head_motion = Some(HeadMotion::LookAround);
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
            height_above_ground: blackboard.field_dimensions.ball_radius,
            image_region_target: ImageRegion::Center,
        });
        Status::Success
    } else {
        Status::Failure
    }
}

#[cfg(test)]
mod tests {
    use linear_algebra::{Pose2, point, vector};
    use ros_z::time::Time;
    use types::{
        ball_position::{BallPosition, HypotheticalBallPosition},
        field_dimensions::{FieldDimensions, Side},
        world_state::BallState,
    };

    use super::*;
    use crate::node::LastBall;

    #[test]
    fn head_tracks_actual_ball_during_predicted_approach_and_rejects_invalid_samples() {
        let mut board = Blackboard {
            field_dimensions: FieldDimensions::SPL_2025,
            ..Default::default()
        };
        let now = Time::zero() + Duration::from_secs(10);
        let transform = Pose2::new(point![1.0, -1.0], 0.0).as_transform();
        let actual = point![2.0, 1.0];
        let visual = BallPosition {
            position: point![2.0, 1.5],
            velocity: vector![0.0, 0.0],
            last_seen: now - Duration::from_millis(20),
        };
        let world_ball = BallState {
            ball_in_ground: actual,
            ball_in_field: transform * actual,
            ball_in_ground_velocity: vector![0.0, -1.0],
            last_seen_ball: (now - Duration::from_millis(50)).to_wallclock(),
            field_side: Side::Left,
            source: BallSource::Own,
        };
        board.world_state.now = now;
        board.world_state.robot.ground_to_field = Some(transform);
        board.parameters.ball.last_ball_timeout = Duration::from_secs(1);
        board.parameters.ball.team_ball_timeout = Duration::from_secs(2);
        board.parameters.ball.pursuit_lookahead = Duration::from_secs(1);
        board.ball = Some(LastBall {
            position: world_ball.ball_in_field,
            velocity: transform * world_ball.ball_in_ground_velocity,
            age: now - Duration::from_millis(50),
            field_side: Side::Left,
            source: BallSource::Own,
        });
        let ball = board.ball.as_ref().unwrap();
        let predicted =
            transform.inverse() * board.projected_ball_position(ball.position, ball.velocity);
        assert_ne!(predicted, actual);
        assert_ne!(predicted, visual.position);
        board.walk_position = Some(predicted);
        let tree = look_at_ball_subtree();
        let ball_radius = board.field_dimensions.ball_radius;
        let look_at = |target| HeadMotion::LookAt {
            target,
            height_above_ground: ball_radius,
            image_region_target: ImageRegion::Center,
        };
        let assert_head = |board: &mut Blackboard, expected| {
            board.head_motion = None;
            assert_eq!(tree.tick_with_trace(board).0, Status::Success);
            assert_eq!(board.head_motion, Some(expected));
            assert_eq!(board.walk_position, Some(predicted));
        };

        board.world_state.ball = Some(world_ball);
        board.visual_kick_ball_position = Some(visual);
        assert_head(&mut board, look_at(visual.position));
        for last_seen in [
            now - Duration::from_millis(60),
            now - Duration::from_millis(101),
            now + Duration::from_millis(1),
        ] {
            board.visual_kick_ball_position.as_mut().unwrap().last_seen = last_seen;
            assert_head(&mut board, look_at(actual));
        }
        board.visual_kick_ball_position = Some(BallPosition {
            position: point![f32::NAN, 0.0],
            ..visual
        });
        assert_head(&mut board, look_at(actual));

        board.world_state.ball = Some(BallState {
            source: BallSource::Team,
            last_seen_ball: now.to_wallclock(),
            ..world_ball
        });
        board.visual_kick_ball_position = Some(visual);
        assert_head(&mut board, look_at(visual.position));
        board.world_state.ball = None;
        board.world_state.robot.ground_to_field = None;
        assert_head(&mut board, look_at(visual.position));

        board.world_state.robot.ground_to_field = Some(transform);
        board.visual_kick_ball_position.as_mut().unwrap().last_seen =
            now - Duration::from_millis(101);
        assert_head(&mut board, look_at(actual));
        board.visual_kick_ball_position = None;
        assert_head(&mut board, look_at(actual));
        for age in [now - Duration::from_secs(1), now + Duration::from_millis(1)] {
            board.ball.as_mut().unwrap().age = age;
            assert_head(&mut board, HeadMotion::SearchForLostBall);
        }
        board.ball.as_mut().unwrap().age = now;
        for last_seen in [now - Duration::from_secs(1), now + Duration::from_millis(1)] {
            board.world_state.ball = Some(BallState {
                last_seen_ball: last_seen.to_wallclock(),
                ..world_ball
            });
            assert_head(&mut board, HeadMotion::SearchForLostBall);
        }
        let hypothetical = point![1.0, -1.0];
        board
            .world_state
            .hypothetical_ball_positions
            .push(HypotheticalBallPosition {
                position: hypothetical,
                validity: 1.0,
            });
        assert_head(&mut board, look_at(hypothetical));
    }
}
