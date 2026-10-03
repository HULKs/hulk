use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::Ground;
use linear_algebra::{IntoFramed, Point2, Vector2};
use ros_z::{context::Context, prelude::*, time::Time};
use serde::{Deserialize, Serialize};
use types::{ball_detection::BallPercept, ball_position::BallPosition};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub percept_timeout: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("visual_kick_ball_selector").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("visual_kick_ball_selector")?;

    let ball_percepts_sub = node
        .subscriber::<Vec<BallPercept>>("ball_filter/ball_percepts")
        .build()
        .await?;
    let ball_position_pub = node
        .publisher::<Option<BallPosition<Ground>>>("visual_kick/ball_position")
        .build()
        .await?;

    let mut selector = VisualKickBallSelector::default();

    loop {
        let ball_percepts = ball_percepts_sub.recv_with_metadata().await?;
        let output_time = ball_percepts.source_time;
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        if let Some(output) = selector.process_frame(
            &ball_percepts.message,
            output_time,
            node.clock().now(),
            parameters.percept_timeout,
        ) {
            ball_position_pub
                .publish_with_source_time(&output, output_time)
                .await?;
        }
    }
}

#[derive(Default)]
struct VisualKickBallSelector {
    last_frame_time: Option<Time>,
}

impl VisualKickBallSelector {
    /// Each message is one actual camera frame, stamped at exposure. An empty
    /// frame revokes authorization immediately; there is no model/held fallback.
    /// Outer None ignores duplicate/backdated delivery without reviving an old hit.
    fn process_frame(
        &mut self,
        percepts: &[BallPercept],
        exposure: Time,
        now: Time,
        timeout: Duration,
    ) -> Option<Option<BallPosition<Ground>>> {
        if exposure > now
            || self
                .last_frame_time
                .is_some_and(|previous| exposure <= previous)
        {
            return None;
        }
        self.last_frame_time = Some(exposure);
        let observed = nearest_visual_kick_ball_position(percepts).map(|position| BallPosition {
            position,
            velocity: Vector2::zeros(),
            last_seen: exposure,
        });
        Some(observed_ball_if_fresh(observed, now, timeout))
    }
}

fn nearest_visual_kick_ball_position(ball_percepts: &[BallPercept]) -> Option<Point2<Ground>> {
    ball_percepts
        .iter()
        .filter(|ball| {
            ball.percept_in_ground
                .mean
                .iter()
                .all(|value| value.is_finite())
        })
        .min_by(|a, b| {
            a.percept_in_ground
                .mean
                .norm_squared()
                .total_cmp(&b.percept_in_ground.mean.norm_squared())
        })
        .map(|ball| ball.percept_in_ground.mean.framed().as_point())
}

fn observed_ball_if_fresh(
    observed_ball: Option<BallPosition<Ground>>,
    now: Time,
    sample_timeout: Duration,
) -> Option<BallPosition<Ground>> {
    observed_ball.filter(|ball| ball.age_at(now).is_some_and(|age| age <= sample_timeout))
}

#[cfg(test)]
fn test_ball_percept(x: f32, y: f32) -> BallPercept {
    use linear_algebra::nalgebra;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    BallPercept {
        percept_in_ground: MultivariateNormalDistribution {
            mean: nalgebra::vector![x, y],
            covariance: nalgebra::Matrix2::identity(),
        },
        image_location: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_empty_frame_revokes_immediately_and_old_hits_cannot_restore_it() {
        let mut selector = VisualKickBallSelector::default();
        let timeout = Duration::from_millis(100);
        let first = Time::from_nanos(1_000_000_000);
        let hit = [test_ball_percept(0.3, 0.0)];
        let output = selector
            .process_frame(&hit, first, first + Duration::from_millis(50), timeout)
            .unwrap()
            .unwrap();
        assert_eq!(
            output.last_seen, first,
            "publication delay must not reset observation age"
        );
        let missed = first + Duration::from_millis(40);
        assert!(
            selector
                .process_frame(&[], missed, missed + Duration::from_millis(50), timeout)
                .unwrap()
                .is_none()
        );
        assert!(
            selector
                .process_frame(&hit, first, missed + Duration::from_millis(50), timeout)
                .is_none()
        );
        assert!(
            selector
                .process_frame(&hit, missed, missed + Duration::from_millis(50), timeout)
                .is_none()
        );
    }

    #[test]
    fn delayed_nonfinite_and_future_observations_cannot_authorize_a_kick() {
        let mut selector = VisualKickBallSelector::default();
        let timeout = Duration::from_millis(100);
        let first = Time::from_nanos(1_000_000_000);
        assert!(
            selector
                .process_frame(
                    &[test_ball_percept(0.3, 0.0)],
                    first,
                    first + Duration::from_millis(101),
                    timeout
                )
                .unwrap()
                .is_none()
        );
        let next = first + Duration::from_millis(200);
        assert!(
            selector
                .process_frame(&[test_ball_percept(f32::NAN, 0.0)], next, next, timeout)
                .unwrap()
                .is_none()
        );
        assert!(
            selector
                .process_frame(
                    &[test_ball_percept(0.3, 0.0)],
                    next + Duration::from_secs(1),
                    next,
                    timeout
                )
                .is_none()
        );
        // A future timestamp must not poison the monotonic gate forever.
        assert!(
            selector
                .process_frame(
                    &[test_ball_percept(0.3, 0.0)],
                    next + Duration::from_millis(40),
                    next + Duration::from_millis(50),
                    timeout
                )
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn observation_expires_after_timeout() {
        let ball = test_ball_percept(1.0, 0.0);

        assert!(
            observed_ball_if_fresh(
                Some(BallPosition {
                    position: ball.percept_in_ground.mean.framed().as_point(),
                    velocity: Vector2::zeros(),
                    last_seen: Time::zero(),
                }),
                Time::zero() + Duration::from_millis(99),
                Duration::from_millis(100)
            )
            .is_some()
        );
        assert!(
            observed_ball_if_fresh(
                Some(BallPosition {
                    position: ball.percept_in_ground.mean.framed().as_point(),
                    velocity: Vector2::zeros(),
                    last_seen: Time::zero(),
                }),
                Time::zero() + Duration::from_millis(101),
                Duration::from_millis(100)
            )
            .is_none()
        );
    }

    #[test]
    fn nearest_visual_kick_ball_position_selects_nearest_percept() {
        let nearest = nearest_visual_kick_ball_position(&[
            test_ball_percept(2.0, 0.0),
            test_ball_percept(1.0, 0.0),
        ])
        .expect("nearest ball should exist");

        assert_eq!(nearest, linear_algebra::point![1.0, 0.0]);
    }
}
