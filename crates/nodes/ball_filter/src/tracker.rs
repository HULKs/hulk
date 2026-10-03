//! Shared production update path for live execution and MCAP replay.
use super::*;
use serde::{Deserialize, Serialize};

pub const SELECTED_OBSTACLES_TOPIC: &str = "ball_filter/obstacles";

pub const FIELD_PRIOR_POSE_TOPIC: &str = "ball_filter/field_prior_pose";

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
    last_detection_time: Option<Time>,
    obstacle_odometry: crate::obstacle_input::OdometryHistory,
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
        self.advance_with_obstacles(
            time, odometry, detections, camera, None, parameters, dimensions,
        )
    }

    /// The obstacle snapshot is expressed in Ground at its own source timestamp.
    /// Live execution records the exact selected payload for deterministic replay.
    pub fn advance_with_obstacles(
        &mut self,
        time: Time,
        odometry: Option<Pose2<Odometry>>,
        detections: Option<&[Object<RobocupObjectLabel>]>,
        camera: Option<&TimeWrapper<CameraMatrix>>,
        obstacles: Option<&TimeWrapper<Vec<types::obstacles::Obstacle>>>,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
    ) -> Result<Vec<BallPercept>> {
        if let Some(odometry) = odometry {
            self.obstacle_odometry.insert(time, odometry);
            predict_hypotheses_from_odometry(
                &mut self.filter,
                time,
                odometry,
                &mut self.last_odometry,
                &mut self.last_prediction_time,
                parameters,
            );
        }
        if !parameters.visible_missed_detection_timeout.is_zero() {
            // Odometry-only updates cannot provide negative perception evidence.
            if detections.is_none() {
                return Ok(Vec::new());
            }
            if self
                .last_detection_time
                .is_some_and(|previous| time <= previous)
            {
                return Ok(Vec::new());
            }
            self.last_detection_time = Some(time);
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
            if !parameters.visible_missed_detection_timeout.is_zero() {
                for hypothesis in &mut self.filter.hypotheses {
                    if let Some(evidence) = &mut hypothesis.negative_evidence {
                        evidence.pause();
                    }
                }
            }
            return Ok(Vec::new());
        };
        let obstacles = self.obstacle_odometry.align(
            time,
            obstacles,
            parameters.maximum_obstacle_time_difference,
        );
        advance_all_hypotheses(
            &mut self.filter,
            &mut self.assignment_solver,
            time,
            &percepts,
            camera,
            obstacles.as_deref(),
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
        self.finish_with_field_pose(time, parameters, dimensions, None)
    }

    /// Applies a selected, timestamp-matched pose only to output confidence.
    /// Replay supplies the exact pose recorded by the live node; legacy inputs
    /// pass None and preserve localization-independent selection.
    pub fn finish_with_field_pose(
        &mut self,
        time: Time,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
        ground_to_field: Option<Isometry2<Ground, Field>>,
    ) -> Option<BallPosition<Ground>> {
        remove_invalid_and_merge_hypotheses(&mut self.filter, time, parameters, dimensions);
        self.filter
            .best_hypothesis_with_field_pose(parameters, dimensions, ground_to_field)
            .map(|h| h.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn negative_evidence_fixture() -> (Tracker, CameraMatrix, BallFilterParameters, FieldDimensions)
    {
        use linear_algebra::{Isometry3, point, vector};
        let camera = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            vector![640.0, 544.0],
            Isometry3::identity(),
            Isometry3::identity(),
            Isometry3::from_translation(0.0, 0.0, 1.0),
        );
        let dimensions = FieldDimensions::SPL_2025;
        let position = camera
            .pixel_to_ground_with_z(point![320.0, 272.0], dimensions.ball_radius)
            .unwrap();
        let mut hypothesis = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![position.x(), position.y(), 0.0, 0.0],
                covariance: Matrix4::identity() * 0.01,
            },
            Time::zero(),
        );
        hypothesis.validity = 25.0;
        let tracker = Tracker {
            filter: BallFilter {
                hypotheses: vec![hypothesis],
            },
            ..Default::default()
        };
        let mut parameters = BallFilterParameters::default();
        parameters.maximum_camera_matrix_time_difference = Duration::from_millis(20);
        parameters.visible_missed_detection_timeout = Duration::from_millis(160);
        parameters.maximum_obstacle_time_difference = Duration::from_millis(100);
        parameters.hidden_validity_exponential_decay_factor = 1.0;
        parameters.visible_validity_exponential_decay_factor = 1.0;
        parameters.velocity_decay_factor = 1.0;
        parameters.validity_output_threshold = 0.5;
        parameters.validity_discard_threshold = 0.01;
        parameters.maximum_matching_cost = 1.0;
        parameters.maximum_number_of_hypotheses = 15;
        parameters.hypothesis_timeout = Duration::from_secs(30);
        parameters.noise.detection_noise.inner.fill(0.01);
        (tracker, camera, parameters, dimensions)
    }

    fn image_object(label: RobocupObjectLabel, confidence: f32) -> Object<RobocupObjectLabel> {
        Object {
            label,
            bounding_box: types::bounding_box::BoundingBox {
                area: geometry::rectangle::Rectangle {
                    min: linear_algebra::point![300.0, 240.0],
                    max: linear_algebra::point![340.0, 304.0],
                },
                confidence,
            },
        }
    }

    fn detector_frame(
        tracker: &mut Tracker,
        camera: &CameraMatrix,
        millis: i64,
        detections: &[Object<RobocupObjectLabel>],
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
    ) {
        let time = Time::from_nanos(millis * 1_000_000);
        tracker
            .advance_with_obstacles(
                time,
                None,
                Some(detections),
                Some(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }),
                Some(&TimeWrapper {
                    time,
                    inner: vec![],
                }),
                parameters,
                dimensions,
            )
            .unwrap();
    }

    #[test]
    fn clear_detector_misses_expire_even_a_long_observed_high_confidence_track() {
        let (mut tracker, camera, parameters, dimensions) = negative_evidence_fixture();
        for millis in [40, 80, 120, 160] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
            assert_eq!(tracker.filter.hypotheses.len(), 1);
        }
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        detector_frame(&mut tracker, &camera, 200, &[], &parameters, &dimensions);
        assert!(tracker.filter.hypotheses.is_empty());
    }

    #[test]
    fn matched_ball_resets_misses_and_legacy_disabled_timeout_preserves_old_behavior() {
        let (mut tracker, camera, parameters, dimensions) = negative_evidence_fixture();
        for millis in [40, 80, 120] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        let ball = image_object(RobocupObjectLabel::Ball, 1.0);
        detector_frame(
            &mut tracker,
            &camera,
            160,
            &[ball],
            &parameters,
            &dimensions,
        );
        assert_eq!(tracker.filter.hypotheses.len(), 1);
        assert!(tracker.filter.hypotheses[0].negative_evidence.is_none());
        assert_eq!(
            tracker.filter.hypotheses[0].last_seen,
            Time::from_nanos(160_000_000)
        );
        for millis in [200, 240, 280, 320] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
            assert_eq!(tracker.filter.hypotheses.len(), 1);
        }
        detector_frame(&mut tracker, &camera, 360, &[], &parameters, &dimensions);
        assert!(tracker.filter.hypotheses.is_empty());

        let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
        parameters.visible_missed_detection_timeout = Duration::ZERO;
        parameters.visible_validity_exponential_decay_factor = 0.9;
        let robot = image_object(RobocupObjectLabel::Robot, 1.0);
        detector_frame(
            &mut tracker,
            &camera,
            40,
            &[robot],
            &parameters,
            &dimensions,
        );
        assert!((tracker.filter.hypotheses[0].validity - 22.5).abs() < 1e-5);
        for millis in (80..1000).step_by(40) {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        assert_eq!(tracker.filter.hypotheses.len(), 1);
        assert!(tracker.filter.hypotheses[0].negative_evidence.is_none());
    }

    #[test]
    fn robot_occlusion_pauses_exposure_and_uses_hidden_decay() {
        let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
        for millis in [40, 80] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        parameters.visible_validity_exponential_decay_factor = 0.5;
        parameters.hidden_validity_exponential_decay_factor = 0.99;
        robot_obstacle_frame(&mut tracker, &camera, 120, &parameters, &dimensions);
        assert!((tracker.filter.hypotheses[0].validity - 24.75).abs() < 1e-5);
        let evidence = tracker.filter.hypotheses[0]
            .negative_evidence
            .as_ref()
            .unwrap();
        assert_eq!(evidence.visible_missed_duration, Duration::from_millis(40));
        assert!(evidence.last_clear_frame.is_none());
        detector_frame(&mut tracker, &camera, 160, &[], &parameters, &dimensions);
        assert_eq!(
            tracker.filter.hypotheses[0]
                .negative_evidence
                .as_ref()
                .unwrap()
                .visible_missed_duration,
            Duration::from_millis(40)
        );
    }

    #[test]
    fn looking_away_or_through_a_robot_keeps_hidden_hypotheses() {
        let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
        parameters.visible_validity_exponential_decay_factor = 0.5;
        for millis in (40..1040).step_by(40) {
            robot_obstacle_frame(&mut tracker, &camera, millis, &parameters, &dimensions);
        }
        let turned_away = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            linear_algebra::vector![640.0, 544.0],
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::from_translation(100.0, 0.0, 1.0),
        );
        for millis in (1040..2040).step_by(40) {
            detector_frame(
                &mut tracker,
                &turned_away,
                millis,
                &[],
                &parameters,
                &dimensions,
            );
        }
        let ball = tracker
            .finish(Time::from_nanos(2_000_000_000), &parameters, &dimensions)
            .unwrap();
        assert_eq!(ball.last_seen, Time::zero());
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        assert_eq!(
            tracker.filter.hypotheses[0]
                .negative_evidence
                .as_ref()
                .unwrap()
                .visible_missed_duration,
            Duration::ZERO
        );
    }

    #[test]
    fn stale_geometry_odometry_only_and_first_frame_after_gap_cannot_expire_ball() {
        let (mut tracker, camera, parameters, dimensions) = negative_evidence_fixture();
        for millis in [40, 80, 120, 160] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        let before = tracker.filter.hypotheses[0]
            .negative_evidence
            .as_ref()
            .unwrap()
            .visible_missed_duration;
        for tick in 81..=100 {
            tracker
                .advance(
                    Time::from_nanos(tick * 2_000_000),
                    Some(Pose2::new(linear_algebra::point![0.0, 0.0], 0.0)),
                    None,
                    None,
                    &parameters,
                    &dimensions,
                )
                .unwrap();
        }
        tracker
            .advance(
                Time::from_nanos(200_000_000),
                None,
                Some(&[]),
                Some(&TimeWrapper {
                    time: Time::zero(),
                    inner: camera.clone(),
                }),
                &parameters,
                &dimensions,
            )
            .unwrap();
        detector_frame(&mut tracker, &camera, 240, &[], &parameters, &dimensions);
        detector_frame(&mut tracker, &camera, 240, &[], &parameters, &dimensions);
        detector_frame(&mut tracker, &camera, 220, &[], &parameters, &dimensions);
        detector_frame(&mut tracker, &camera, 10_000, &[], &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses.len(), 1);
        assert_eq!(
            tracker.filter.hypotheses[0]
                .negative_evidence
                .as_ref()
                .unwrap()
                .visible_missed_duration,
            before
        );
        detector_frame(&mut tracker, &camera, 10_040, &[], &parameters, &dimensions);
        assert!(tracker.filter.hypotheses.is_empty());
    }

    fn robot_obstacle_frame(
        tracker: &mut Tracker,
        camera: &CameraMatrix,
        millis: i64,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
    ) {
        let time = Time::from_nanos(millis * 1_000_000);
        let robot = Obstacle::robot(linear_algebra::point![0.0, 0.0], 0.2, 0.3);
        tracker
            .advance_with_obstacles(
                time,
                None,
                Some(&[]),
                Some(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }),
                Some(&TimeWrapper {
                    time,
                    inner: vec![robot],
                }),
                parameters,
                dimensions,
            )
            .unwrap();
    }

    #[test]
    fn missing_stale_or_unalignable_obstacles_pause_negative_evidence() {
        let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
        for millis in [40, 80, 120, 160] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        parameters.visible_validity_exponential_decay_factor = 0.1;
        for (millis, source) in [(200, None), (240, Some(40)), (280, Some(240))] {
            let time = Time::from_nanos(millis * 1_000_000);
            let obstacles = source.map(|source| TimeWrapper {
                time: Time::from_nanos(source * 1_000_000),
                inner: vec![],
            });
            tracker
                .advance_with_obstacles(
                    time,
                    None,
                    Some(&[]),
                    Some(&TimeWrapper {
                        time,
                        inner: camera.clone(),
                    }),
                    obstacles.as_ref(),
                    &parameters,
                    &dimensions,
                )
                .unwrap();
            assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
            assert_eq!(
                tracker.filter.hypotheses[0]
                    .negative_evidence
                    .as_ref()
                    .unwrap()
                    .visible_missed_duration,
                Duration::from_millis(120)
            );
        }
        detector_frame(&mut tracker, &camera, 320, &[], &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses.len(), 1);
        detector_frame(&mut tracker, &camera, 360, &[], &parameters, &dimensions);
        assert!(tracker.filter.hypotheses.is_empty());
    }

    #[test]
    fn robot_occlusion_uses_foreground_depth_and_preserves_camera_fringe_safeguards() {
        use linear_algebra::point;
        use types::obstacles::ObstacleKind;
        let (tracker, camera, _, dimensions) = negative_evidence_fixture();
        let mut ball = tracker.filter.hypotheses[0].position();
        ball.position = point![0.3, 0.0];
        let visible = |position: &BallPosition<Ground>, obstacles: Option<&[Obstacle]>| {
            negative_evidence::clearly_visible(position, &camera, dimensions.ball_radius, obstacles)
        };
        assert!(visible(&ball, Some(&[])));
        assert!(!visible(&ball, None));
        let mut robot = Obstacle::robot(point![0.15, 0.0], 0.05, 0.1);
        assert!(!visible(&ball, Some(&[robot])));
        robot.position = point![1.0, 0.0];
        assert!(
            visible(&ball, Some(&[robot])),
            "robot behind ball must not shield it"
        );
        robot.position = point![0.15, 0.5];
        assert!(visible(&ball, Some(&[robot])));
        robot.position = point![0.15, 0.0];
        for kind in [
            ObstacleKind::Ball,
            ObstacleKind::GoalPost,
            ObstacleKind::Person,
            ObstacleKind::Unknown,
        ] {
            robot.kind = kind;
            assert!(
                visible(&ball, Some(&[robot])),
                "only world-state robots shield"
            );
        }
        robot.kind = ObstacleKind::Robot;
        robot.position = point![f32::NAN, 0.0];
        assert!(
            !visible(&ball, Some(&[robot])),
            "malformed robot state is unknown visibility"
        );
        for pixel in [point![1.0, 272.0], point![-100.0, 272.0]] {
            let mut hidden = ball;
            hidden.position = camera
                .pixel_to_ground_with_z(pixel, dimensions.ball_radius)
                .unwrap();
            assert!(!visible(&hidden, Some(&[])));
        }
        let distant_camera = CameraMatrix::from_normalized_focal_and_center(
            nalgebra::vector![0.5, 0.5],
            nalgebra::point![0.5, 0.5],
            linear_algebra::vector![640.0, 544.0],
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::identity(),
            linear_algebra::Isometry3::from_translation(0.0, 0.0, 100.0),
        );
        assert!(!negative_evidence::clearly_visible(
            &ball,
            &distant_camera,
            dimensions.ball_radius,
            Some(&[])
        ));
    }

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
