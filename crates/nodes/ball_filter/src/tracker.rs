//! Shared production update path for live execution and MCAP replay.
use super::*;
use serde::{Deserialize, Serialize};

/// Records fusion boundaries and the selected camera timestamp, without copying inputs.
/// This diagnostic is identical on the robot and simulator. It preserves batching
/// and camera selection when replaying the original ros-z input messages.
#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct UpdateSchedule {
    pub sequence: u64,
    pub inputs: Vec<InputStamp>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Message)]
pub struct InputStamp {
    pub time: Time,
    pub odometry: bool,
    pub detections: bool,
    pub camera_time: Option<Time>,
}

#[derive(Default)]
pub struct Tracker {
    pub filter: BallFilter,
    assignment_solver: AssignmentSolver,
    last_odometry: Option<Pose2<Odometry>>,
    last_prediction_time: Option<Time>,
}

pub fn camera_is_recent(image_time: Time, camera_time: Time, tolerance: Duration) -> bool {
    u128::from(image_time.as_nanos().abs_diff(camera_time.as_nanos())) <= tolerance.as_nanos()
}

impl Tracker {
    pub fn advance(
        &mut self,
        time: Time,
        odometry: Option<Pose2<Odometry>>,
        detections: Option<&[Object<RobocupObjectLabel>]>,
        camera: Option<&TimeWrapper<CameraMatrix>>,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
    ) -> Result<Vec<BallPercept>> {
        if let Some(odometry) = odometry {
            predict_hypotheses_from_odometry(
                &mut self.filter,
                time,
                odometry,
                &mut self.last_odometry,
                &mut self.last_prediction_time,
                parameters,
            );
        }
        let camera = camera
            .filter(|c| {
                camera_is_recent(
                    time,
                    c.time,
                    parameters.maximum_camera_matrix_time_difference,
                )
            })
            .map(|c| &c.inner);
        let Some(percepts) =
            project_detected_balls(detections, camera, parameters, dimensions.ball_radius)
        else {
            return Ok(Vec::new());
        };
        advance_all_hypotheses(
            &mut self.filter,
            &mut self.assignment_solver,
            time,
            &percepts,
            camera,
            parameters,
            dimensions,
        )?;
        Ok(percepts)
    }

    pub fn finish(
        &mut self,
        time: Time,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
    ) -> Option<BallPosition<Ground>> {
        remove_invalid_and_merge_hypotheses(&mut self.filter, time, parameters, dimensions);
        self.filter
            .best_hypothesis(parameters.validity_output_threshold)
            .map(|h| h.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_range_keeps_far_balls_independently_of_robot_heading() {
        let dimensions = FieldDimensions::SPL_2025;
        let range = ((dimensions.length + 2.0 * dimensions.border_strip_width).powi(2)
            + (dimensions.width + 2.0 * dimensions.border_strip_width).powi(2))
        .sqrt();
        for angle in [0.0_f32, 0.8, 1.6, 3.0] {
            let ball = BallPosition::<Ground> {
                position: linear_algebra::point![
                    0.95 * range * angle.cos(),
                    0.95 * range * angle.sin()
                ],
                velocity: linear_algebra::Vector2::zeros(),
                last_seen: Time::zero(),
            };
            assert!(is_ball_within_field_range(ball, &dimensions));
        }
        let ball = BallPosition::<Ground> {
            position: linear_algebra::point![range + 1.0, 0.0],
            velocity: linear_algebra::Vector2::zeros(),
            last_seen: Time::zero(),
        };
        assert!(!is_ball_within_field_range(ball, &dimensions));
    }

    #[test]
    fn camera_tolerance_is_symmetric_and_inclusive() {
        let image = Time::from_nanos(100_000_000);
        let tolerance = Duration::from_millis(20);
        for time in [80_000_000, 100_000_000, 120_000_000] {
            assert!(camera_is_recent(image, Time::from_nanos(time), tolerance));
        }
        for time in [79_999_999, 120_000_001] {
            assert!(!camera_is_recent(image, Time::from_nanos(time), tolerance));
        }
    }

    #[test]
    fn stale_geometry_skips_measurements_but_keeps_odometry() {
        use geometry::rectangle::Rectangle;
        use linear_algebra::{Isometry3, point, vector};
        use types::bounding_box::BoundingBox;
        let time = Time::from_nanos(100_000_000);
        let mut camera = TimeWrapper {
            time,
            inner: CameraMatrix::from_normalized_focal_and_center(
                nalgebra::vector![0.5, 0.5],
                nalgebra::point![0.5, 0.5],
                vector![640.0, 544.0],
                Isometry3::identity(),
                Isometry3::identity(),
                Isometry3::from_translation(0.0, 0.0, 1.0),
            ),
        };
        let detections = [Object {
            label: RobocupObjectLabel::Ball,
            bounding_box: BoundingBox {
                area: Rectangle {
                    min: point![310.0, 262.0],
                    max: point![330.0, 282.0],
                },
                confidence: 0.9,
            },
        }];
        let mut parameters = BallFilterParameters::default();
        parameters.maximum_camera_matrix_time_difference = Duration::from_millis(20);
        parameters.noise.initial_covariance.fill(1.0);
        parameters.noise.detection_noise.inner.fill(1.0);
        let mut tracker = Tracker::default();
        let dimensions = FieldDimensions::SPL_2025;
        assert_eq!(
            tracker
                .advance(
                    time,
                    None,
                    Some(&detections),
                    Some(&camera),
                    &parameters,
                    &dimensions
                )
                .unwrap()
                .len(),
            1
        );
        let validity = tracker.filter.hypotheses[0].validity;
        camera.time = Time::from_nanos(79_000_000);
        assert!(
            tracker
                .advance(
                    time,
                    Some(Pose2::new(point![0.0, 0.0], 0.0)),
                    Some(&detections),
                    Some(&camera),
                    &parameters,
                    &dimensions
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(tracker.last_prediction_time, Some(time));
        assert_eq!(tracker.filter.hypotheses[0].validity, validity);
        assert_eq!(tracker.filter.hypotheses.len(), 1);
    }
}
