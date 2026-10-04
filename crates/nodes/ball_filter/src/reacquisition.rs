//! Retain alternate hypotheses after a surprising detection of a quiet ball.
//! Rolling balls and balls within kicking reach must remain free to accelerate.
use crate::hypothesis::BallHypothesis;
use ros_z::time::Time;
use std::time::Duration;
use types::{
    obstacles::{Obstacle, ObstacleKind},
    parameters::BallFilterParameters,
};

pub fn protect_prior(
    hypothesis: &BallHypothesis,
    time: Time,
    obstacles: Option<&[Obstacle]>,
    parameters: &BallFilterParameters,
) -> bool {
    if time <= hypothesis.last_seen
        || time.duration_since(hypothesis.last_seen) <= Duration::from_millis(120)
    {
        return false;
    }
    let ball = hypothesis.position();
    // Robot itself can kick a near ball. Unknown obstacle geometry cannot
    // establish that no other player could have kicked it during the gap.
    if ball.position.coords().norm_squared() <= 0.6_f32.powi(2) {
        return false;
    }
    let Some(obstacles) = obstacles else {
        return false;
    };
    if obstacles.iter().any(|obstacle| {
        if matches!(obstacle.kind, ObstacleKind::Ball | ObstacleKind::GoalPost) {
            return false;
        }
        !obstacle
            .position
            .coords()
            .inner
            .iter()
            .all(|x| x.is_finite())
            || !obstacle.radius_at_foot_height.is_finite()
            || obstacle.radius_at_foot_height < 0.0
            || (obstacle.position - ball.position).norm() < obstacle.radius_at_foot_height + 0.5
    }) {
        return false;
    }
    let speed = ball.velocity.norm();
    let decay = parameters.velocity_decay_factor;
    if !speed.is_finite() || !decay.is_finite() || decay <= 0.0 || decay > 1.0 {
        return false;
    }
    // Undo deterministic damping to judge speed at the last observation. A
    // rolling prediction slowing during a long gap is not evidence of rest.
    let observed_log_speed =
        speed.ln() - time.duration_since(hypothesis.last_seen).as_secs_f32() / 0.002 * decay.ln();
    observed_log_speed <= 0.2_f32.ln()
}

#[cfg(test)]
mod tests {
    use super::*;
    use linear_algebra::point;
    use nalgebra::Matrix4;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;
    #[test]
    fn kick_opportunities_unknown_geometry_and_pre_gap_motion_are_exempt() {
        let make = |x, speed| {
            BallHypothesis::new(
                MultivariateNormalDistribution {
                    mean: nalgebra::vector![x, 0.0, speed, 0.0],
                    covariance: Matrix4::identity(),
                },
                Time::zero(),
            )
        };
        let parameters = BallFilterParameters {
            velocity_decay_factor: 0.998,
            ..Default::default()
        };
        let time = Time::from_nanos(2_000_000_000);
        assert!(protect_prior(&make(1.0, 0.0), time, Some(&[]), &parameters));
        assert!(!protect_prior(
            &make(0.3, 0.0),
            time,
            Some(&[]),
            &parameters
        ));
        assert!(!protect_prior(&make(1.0, 0.0), time, None, &parameters));
        assert!(!protect_prior(
            &make(1.0, 0.04),
            time,
            Some(&[]),
            &parameters
        ));
        assert!(!protect_prior(
            &make(1.0, 0.0),
            time,
            Some(&[Obstacle::robot(point![1.3, 0.0], 0.2, 0.3)]),
            &parameters
        ));
        assert!(!protect_prior(
            &make(1.0, 0.0),
            Time::from_nanos(40_000_000),
            Some(&[]),
            &parameters
        ));
    }
}
