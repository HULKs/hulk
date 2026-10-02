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
    fn moving_ball_predicts_to_odometry_time_between_camera_frames() {
        use geometry::rectangle::Rectangle;
        use linear_algebra::{Isometry3, point, vector};
        use types::bounding_box::BoundingBox;

        let camera = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 544.0],
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::from_translation(0.0, 0.0, 1.0),
        );
        let dimensions = FieldDimensions::SPL_2025;
        let mut parameters = BallFilterParameters::default();
        parameters.maximum_camera_matrix_time_difference = Duration::from_millis(20);
        parameters.hidden_validity_exponential_decay_factor = 1.0;
        parameters.visible_validity_exponential_decay_factor = 1.0;
        parameters.velocity_decay_factor = 1.0; // Constant-speed reference, without friction.
        parameters.log_likelihood_of_zero_velocity_threshold = 0.5;
        parameters.maximum_matching_cost = 10.0;
        parameters.validity_output_threshold = 0.5;
        parameters.maximum_number_of_hypotheses = 15;
        parameters.hypothesis_timeout = Duration::from_secs(20);
        parameters.noise.initial_covariance = nalgebra::vector![0.01, 0.01, 1.0, 1.0];
        parameters.noise.process_noise_moving = nalgebra::vector![1e-6, 1e-6, 1e-4, 1e-4];
        parameters.noise.process_noise_resting.fill(1e-6);
        parameters.noise.detection_noise.inner.fill(0.02);
        let mut tracker = Tracker::default();

        // Real simulator cadence: 500 Hz odometry, 25 Hz images. Finish with
        // 40 ms without a new image, so returning the last measurement fails.
        for tick in 0..=520 {
            let time = Time::from_nanos(tick * 2_000_000);
            let detections = if tick <= 500 && tick % 20 == 0 {
                let ground_position = point![0.2 + 1.5 * tick as f32 * 0.002, 0.0];
                let pixel = camera
                    .ground_with_z_to_pixel(ground_position, dimensions.ball_radius)
                    .unwrap();
                let radius = camera
                    .get_pixel_radius(dimensions.ball_radius, pixel)
                    .unwrap();
                Some(vec![Object {
                    label: RobocupObjectLabel::Ball,
                    bounding_box: BoundingBox {
                        area: Rectangle {
                            min: pixel - vector![radius, radius],
                            max: pixel + vector![radius, radius],
                        },
                        confidence: 1.0,
                    },
                }])
            } else {
                None
            };
            tracker
                .advance(
                    time,
                    Some(Pose2::new(point![0.0, 0.0], 0.0)),
                    detections.as_deref(),
                    Some(&TimeWrapper {
                        time,
                        inner: camera.clone(),
                    }),
                    &parameters,
                    &dimensions,
                )
                .unwrap();
        }
        let ball = tracker
            .finish(Time::from_nanos(1_040_000_000), &parameters, &dimensions)
            .unwrap();
        assert!(
            (ball.position.x() - 1.76).abs() < 0.01,
            "position: {:?}",
            ball.position
        );
        assert!(
            (ball.velocity.x() - 1.5).abs() < 0.05,
            "velocity: {:?}",
            ball.velocity
        );
        assert_eq!(ball.last_seen, Time::from_nanos(1_000_000_000));
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
        parameters.velocity_decay_factor = 1.0;
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
