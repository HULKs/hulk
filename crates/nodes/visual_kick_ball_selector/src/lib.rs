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

    let mut held_ball = None;

    loop {
        let ball_percepts = ball_percepts_sub.recv_with_metadata().await?;
        let output_time = ball_percepts.source_time;
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        if let Some(position) = nearest_visual_kick_ball_position(&ball_percepts.message) {
            held_ball = Some(BallPosition {
                position,
                velocity: Vector2::zeros(),
                last_seen: output_time,
            });
        }

        let output = held_ball_if_fresh(held_ball, output_time, parameters.percept_timeout);
        if output.is_none() {
            held_ball = None;
        }
        ball_position_pub.publish(&output).await?;
    }
}

fn nearest_visual_kick_ball_position(ball_percepts: &[BallPercept]) -> Option<Point2<Ground>> {
    ball_percepts
        .iter()
        .min_by(|a, b| {
            a.percept_in_ground
                .mean
                .norm_squared()
                .total_cmp(&b.percept_in_ground.mean.norm_squared())
        })
        .map(|ball| ball.percept_in_ground.mean.framed().as_point())
}

fn held_ball_if_fresh(
    held_ball: Option<BallPosition<Ground>>,
    now: Time,
    sample_timeout: Duration,
) -> Option<BallPosition<Ground>> {
    held_ball.filter(|ball| ball.age_at(now).is_some_and(|age| age <= sample_timeout))
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
    fn held_ball_expires_after_timeout() {
        let ball = test_ball_percept(1.0, 0.0);

        assert!(
            held_ball_if_fresh(
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
            held_ball_if_fresh(
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
