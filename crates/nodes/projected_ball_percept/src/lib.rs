use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use coordinate_systems::Ground;
use linear_algebra::{Point2, Vector2, vector};
use projection::{Projection, camera_matrix::CameraMatrix};
use ros_z::{context::Context, prelude::*, qos::QosDurability, time::Time};
use ros_z_streams::CreateFutureMapBuilder;
use serde::{Deserialize, Serialize};
use types::{
    ball_position::BallPosition,
    field_dimensions::FieldDimensions,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub confidence_threshold: f32,
    pub sample_timeout: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("projected_ball_percept").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("projected_ball_percept")?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(10)
        .with_stamp(|wrapper: &TimeWrapper<CameraMatrix>| wrapper.time)
        .build()
        .await?;
    let mut detected_objects = node
        .create_future_map_builder()
        .create_future_subscriber::<Vec<Object<RobocupObjectLabel>>>(
            "detected_objects",
            Duration::from_millis(25),
        )
        .await?
        .build();
    let ball_position_pub = node
        .publisher::<Option<BallPosition<Ground>>>("projected_ball_percept/ball_position")
        .build()
        .await?;

    let mut held_ball = None;
    let mut timer = node.create_timer(Duration::from_millis(20));

    loop {
        tokio::select! {
            item = detected_objects.recv() => {
                let item = item?;
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();

                if let Some(field_dimensions) = field_dimensions_cache.get_latest() {
                    for (detection_time, (detected_objects,)) in item.persistent {
                        let Some(detected_objects) = detected_objects else {
                            continue;
                        };
                        let Some(camera_matrix) = camera_matrix_cache.get_nearest(detection_time) else {
                            continue;
                        };

                        if let Some(position) = nearest_projected_ball(
                            &detected_objects,
                            &camera_matrix.inner,
                            field_dimensions.ball_radius,
                            parameters.confidence_threshold,
                        ) {
                            held_ball = Some(BallPosition {
                                position,
                                velocity: Vector2::zeros(),
                                last_seen: detection_time,
                            });
                        }
                    }
                }

                let now = node.clock().now();
                let output = held_ball_if_fresh(held_ball, now, parameters.sample_timeout);
                if output.is_none() {
                    held_ball = None;
                }
                ball_position_pub.publish(&output).await?;
            }
            _ = timer.tick() => {
                let now = node.clock().now();
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();
                let output = held_ball_if_fresh(held_ball, now, parameters.sample_timeout);
                if output.is_none() {
                    held_ball = None;
                }
                ball_position_pub.publish(&output).await?;
            }
        }
    }
}

fn nearest_projected_ball(
    detections: &[Object<RobocupObjectLabel>],
    camera_matrix: &CameraMatrix,
    ball_radius: f32,
    confidence_threshold: f32,
) -> Option<Point2<Ground>> {
    detections
        .iter()
        .filter(|detection| {
            detection.label == RobocupObjectLabel::Ball
                && detection.bounding_box.confidence >= confidence_threshold
        })
        .filter_map(|detection| {
            camera_matrix
                .pixel_to_ground_with_z(detection.bounding_box.area.center(), ball_radius)
                .ok()
        })
        .min_by(|a, b| {
            a.coords()
                .norm_squared()
                .total_cmp(&b.coords().norm_squared())
        })
        .map(|ball| ball + vector!(0.05, 0.05))
}

fn held_ball_if_fresh(
    held_ball: Option<BallPosition<Ground>>,
    now: Time,
    sample_timeout: Duration,
) -> Option<BallPosition<Ground>> {
    held_ball.filter(|ball| ball.age_at(now).is_some_and(|age| age <= sample_timeout))
}

#[cfg(test)]
mod tests {
    use linear_algebra::point;

    use super::*;

    #[test]
    fn held_ball_expires_after_timeout() {
        let ball = BallPosition {
            position: point![1.0, 0.0],
            velocity: Vector2::zeros(),
            last_seen: Time::zero(),
        };

        assert!(
            held_ball_if_fresh(
                Some(ball),
                Time::zero() + Duration::from_millis(99),
                Duration::from_millis(100)
            )
            .is_some()
        );
        assert!(
            held_ball_if_fresh(
                Some(ball),
                Time::zero() + Duration::from_millis(101),
                Duration::from_millis(100)
            )
            .is_none()
        );
    }
}
