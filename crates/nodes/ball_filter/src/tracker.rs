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
    field_decay_clock: crate::field_prior::ValidityDecayClock,
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
    #[expect(
        clippy::too_many_arguments,
        reason = "Keep independently timestamped sensor inputs explicit at the replay boundary"
    )]
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
        // Admission is independent of retention policy. Repeated exposures must
        // never run association, confidence updates, decay, or spawning twice.
        // Odometry above still advances independently of detector availability.
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
            if negative_evidence::enabled(parameters)
                || validity_decay::enabled(parameters)
                || competition::enabled(parameters)
            {
                for hypothesis in &mut self.filter.hypotheses {
                    hypothesis.validity_decay_evidence = None;
                    hypothesis.leadership_evidence = None;
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

    /// Applies the selected, timestamp-matched field pose to the output prior
    /// and optional stored-validity decay. Replay supplies the exact recorded
    /// pose; a missing pose pauses field-dependent decay. Disabling trust in
    /// localization clears its clock so re-enabling cannot charge unknown time.
    pub fn finish_with_field_pose(
        &mut self,
        time: Time,
        parameters: &BallFilterParameters,
        dimensions: &FieldDimensions,
        ground_to_field: Option<Isometry2<Ground, Field>>,
    ) -> Option<BallPosition<Ground>> {
        let valid_pose = ground_to_field.is_some_and(|pose| {
            pose.inner
                .to_homogeneous()
                .iter()
                .all(|value| value.is_finite())
        }) && parameters.field_boundary_validity_decay_rate.is_finite()
            && parameters.field_boundary_validity_decay_rate > 0.0;
        let elapsed = if parameters.good_localization {
            self.field_decay_clock.elapsed(time, valid_pose)
        } else {
            self.field_decay_clock.reset();
            Duration::ZERO
        };
        field_prior::decay_stored_validity(
            &mut self.filter.hypotheses,
            elapsed,
            ground_to_field,
            dimensions,
            parameters,
        );
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

    fn merge_track(x: f32, velocity: f32, validity: f32, milliseconds: i64) -> BallHypothesis {
        let mut track = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![x, 0.0, velocity, 0.0],
                covariance: Matrix4::identity() * 0.1,
            },
            Time::from_nanos(milliseconds * 1_000_000),
        );
        track.validity = validity;
        track
    }

    #[test]
    fn moving_duplicates_merge_without_losing_velocity_or_counting_confidence_twice() {
        for reverse in [false, true] {
            let (mut tracker, _, mut parameters, dimensions) = negative_evidence_fixture();
            parameters.hypothesis_merge_distance = 0.1;
            let tracks = [
                merge_track(1.0, 3.0, 10.0, 40),
                merge_track(1.06, 3.1, 4.0, 80),
            ];
            tracker.filter.hypotheses = if reverse {
                tracks.into_iter().rev().collect()
            } else {
                tracks.into()
            };
            let selected = tracker
                .finish(Time::from_nanos(120_000_000), &parameters, &dimensions)
                .unwrap();
            assert_eq!(tracker.filter.hypotheses.len(), 1);
            let merged = &tracker.filter.hypotheses[0];
            assert!(matches!(merged.mode, BallMode::Moving(_)));
            assert_eq!(merged.validity, 10.0);
            assert_eq!(selected.last_seen, Time::from_nanos(80_000_000));
            assert!((selected.position.x() - 1.03).abs() < 1e-5);
            assert!((selected.velocity.x() - 3.05).abs() < 1e-5);
            assert!(
                (merged.position_covariance() - Matrix2::identity() * 0.1).norm() < 1e-5,
                "duplicate tracks must not halve their shared uncertainty"
            );
        }
    }

    #[test]
    fn distinct_moving_balls_and_unconfirmed_kick_hypotheses_are_not_merged() {
        let mut cases = vec![
            // Crossing balls, despite matching positions and broad uncertainty.
            vec![
                merge_track(1.0, 3.0, 10.0, 40),
                merge_track(1.02, -3.0, 5.0, 80),
            ],
            // Two detections in one actual image retain independent support.
            vec![
                merge_track(1.0, 3.0, 10.0, 80),
                merge_track(1.02, 3.0, 5.0, 80),
            ],
            // A new, uncertain hypothesis must establish its own motion first.
            vec![
                merge_track(1.0, 0.2, 10.0, 40),
                merge_track(1.02, 0.0, 1.0, 80),
            ],
            // Position separation still has a hard configured limit.
            vec![
                merge_track(1.0, 3.0, 10.0, 40),
                merge_track(1.6, 3.0, 5.0, 80),
            ],
        ];
        let mut precise = vec![
            merge_track(1.0, 3.0, 10.0, 40),
            merge_track(1.08, 3.0, 5.0, 80),
        ];
        for track in &mut precise {
            let BallMode::Moving(state) = &mut track.mode else {
                unreachable!()
            };
            state.covariance = Matrix4::identity() * 0.00001;
        }
        cases.push(precise);
        for tracks in cases {
            let (mut tracker, _, mut parameters, dimensions) = negative_evidence_fixture();
            parameters.hypothesis_merge_distance = 0.5;
            tracker.filter.hypotheses = tracks;
            tracker.finish(Time::from_nanos(120_000_000), &parameters, &dimensions);
            assert_eq!(tracker.filter.hypotheses.len(), 2);
        }
    }

    #[test]
    fn resting_duplicates_still_merge_but_mixed_motion_modes_remain_separate() {
        for mixed in [false, true] {
            let (mut tracker, _, mut parameters, dimensions) = negative_evidence_fixture();
            parameters.hypothesis_merge_distance = 0.1;
            let mut old = merge_track(1.0, 0.0, 10.0, 40);
            old.mode = BallMode::Resting(MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 0.0],
                covariance: Matrix2::identity() * 0.1,
            });
            let mut recent = merge_track(1.04, 2.0, 4.0, 80);
            if !mixed {
                recent.mode = BallMode::Resting(MultivariateNormalDistribution {
                    mean: nalgebra::vector![1.04, 0.0],
                    covariance: Matrix2::identity() * 0.1,
                });
            }
            tracker.filter.hypotheses = vec![old, recent];
            tracker.finish(Time::from_nanos(120_000_000), &parameters, &dimensions);
            assert_eq!(tracker.filter.hypotheses.len(), if mixed { 2 } else { 1 });
            if !mixed {
                let merged = &tracker.filter.hypotheses[0];
                assert!((merged.position().position.x() - 1.02).abs() < 1e-5);
                assert_eq!(merged.validity, 10.0);
                assert_eq!(merged.last_seen, Time::from_nanos(80_000_000));
            }
        }
    }

    #[test]
    fn merging_an_intermediate_track_cannot_erase_same_exposure_two_ball_support() {
        let (mut tracker, _, mut parameters, dimensions) = negative_evidence_fixture();
        parameters.hypothesis_merge_distance = 0.1;
        tracker.filter.hypotheses = vec![
            merge_track(1.0, 1.0, 10.0, 40),
            merge_track(1.01, 1.0, 4.0, 80),
            merge_track(1.02, 1.0, 4.0, 40),
        ];
        for milliseconds in [120, 122, 124, 160] {
            tracker.finish(
                Time::from_nanos(milliseconds * 1_000_000),
                &parameters,
                &dimensions,
            );
            assert_eq!(tracker.filter.hypotheses.len(), 2);
        }
    }

    #[test]
    fn a_failed_covariance_merge_keeps_both_hypotheses_and_does_not_panic() {
        let old = merge_track(1.0, 1.0, 10.0, 40);
        let mut singular = merge_track(1.01, 1.0, 4.0, 80);
        let BallMode::Moving(state) = &mut singular.mode else {
            unreachable!()
        };
        state.covariance = Matrix4::zeros();
        let mut filter = BallFilter {
            hypotheses: vec![old, singular],
        };
        filter.remove_hypotheses(|_| true, |_, _| true);
        assert_eq!(filter.hypotheses.len(), 2);
        assert_eq!(filter.hypotheses[0].position().position.x(), 1.0);
        assert_eq!(filter.hypotheses[0].validity, 10.0);
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

    fn learned_decay_parameters() -> (Tracker, CameraMatrix, BallFilterParameters, FieldDimensions)
    {
        let (tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
        parameters.visible_missed_detection_timeout = Duration::ZERO;
        parameters.visible_validity_exponential_decay_factor = 0.5;
        parameters.hidden_validity_exponential_decay_factor = 0.3;
        parameters.visible_missed_validity_decay_rate = Some(1.0);
        parameters.hidden_validity_decay_rate = Some(0.1);
        (tracker, camera, parameters, dimensions)
    }

    #[test]
    fn learned_visible_and_hidden_rates_replace_factors_and_are_frame_rate_independent() {
        for hidden in [false, true] {
            for step in [20, 40, 100] {
                let (mut tracker, camera, parameters, dimensions) = learned_decay_parameters();
                for millis in (0..=1000).step_by(step) {
                    if hidden {
                        robot_obstacle_frame(
                            &mut tracker,
                            &camera,
                            millis,
                            &parameters,
                            &dimensions,
                        );
                    } else {
                        detector_frame(
                            &mut tracker,
                            &camera,
                            millis,
                            &[],
                            &parameters,
                            &dimensions,
                        );
                    }
                }
                let expected = 25.0
                    * if hidden {
                        (-0.1_f32).exp()
                    } else {
                        (-1.0_f32).exp()
                    };
                assert!(
                    (tracker.filter.hypotheses[0].validity - expected).abs() < 0.001,
                    "hidden={hidden}, step={step}"
                );
            }
        }
        let (mut tracker, camera, mut parameters, dimensions) = learned_decay_parameters();
        parameters.hidden_validity_decay_rate = Some(0.0);
        parameters.visible_missed_validity_decay_rate = Some(0.0);
        for millis in (0..=1000).step_by(40) {
            if millis < 500 {
                detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
            } else {
                robot_obstacle_frame(&mut tracker, &camera, millis, &parameters, &dimensions);
            }
        }
        assert_eq!(
            tracker.filter.hypotheses[0].validity, 25.0,
            "zero must really disable unmatched confidence decay"
        );
    }

    #[test]
    fn learned_retention_pauses_on_unknown_gaps_transitions_and_duplicate_frames() {
        let (mut tracker, camera, parameters, dimensions) = learned_decay_parameters();
        detector_frame(&mut tracker, &camera, 40, &[], &parameters, &dimensions);
        // Category change starts a new interval rather than charging a mixed one.
        robot_obstacle_frame(&mut tracker, &camera, 80, &parameters, &dimensions);
        let time = Time::from_nanos(120_000_000);
        tracker
            .advance_with_obstacles(
                time,
                None,
                Some(&[]),
                Some(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }),
                None,
                &parameters,
                &dimensions,
            )
            .unwrap();
        robot_obstacle_frame(&mut tracker, &camera, 160, &parameters, &dimensions);
        robot_obstacle_frame(&mut tracker, &camera, 160, &parameters, &dimensions);
        robot_obstacle_frame(&mut tracker, &camera, 140, &parameters, &dimensions);
        robot_obstacle_frame(&mut tracker, &camera, 10_000, &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        robot_obstacle_frame(&mut tracker, &camera, 10_040, &parameters, &dimensions);
        let expected = 25.0 * (-0.1_f32 * 0.04).exp();
        assert!((tracker.filter.hypotheses[0].validity - expected).abs() < 1e-5);
    }

    #[test]
    fn learned_retention_does_not_count_odometry_ticks_or_bridge_stale_camera_frames() {
        let (mut tracker, camera, parameters, dimensions) = learned_decay_parameters();
        detector_frame(&mut tracker, &camera, 40, &[], &parameters, &dimensions);
        for tick in 21..=40 {
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
        let time = Time::from_nanos(80_000_000);
        tracker
            .advance_with_obstacles(
                time,
                None,
                Some(&[]),
                Some(&TimeWrapper {
                    time: Time::zero(),
                    inner: camera.clone(),
                }),
                Some(&TimeWrapper {
                    time,
                    inner: vec![],
                }),
                &parameters,
                &dimensions,
            )
            .unwrap();
        detector_frame(&mut tracker, &camera, 120, &[], &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        detector_frame(&mut tracker, &camera, 160, &[], &parameters, &dimensions);
        assert!((tracker.filter.hypotheses[0].validity - 25.0 * (-0.04_f32).exp()).abs() < 1e-5);
    }

    #[test]
    fn matched_confidence_update_and_nullable_legacy_policy_remain_unchanged() {
        let (mut tracker, camera, parameters, dimensions) = learned_decay_parameters();
        let ball = image_object(RobocupObjectLabel::Ball, 1.0);
        detector_frame(&mut tracker, &camera, 40, &[ball], &parameters, &dimensions);
        let matched = 25.0 * 0.5 + 1.0;
        assert!((tracker.filter.hypotheses[0].validity - matched).abs() < 1e-5);
        assert!(
            tracker.filter.hypotheses[0]
                .validity_decay_evidence
                .is_none()
        );
        detector_frame(&mut tracker, &camera, 80, &[], &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses[0].validity, matched);
        detector_frame(&mut tracker, &camera, 120, &[], &parameters, &dimensions);
        assert!((tracker.filter.hypotheses[0].validity - matched * (-0.04_f32).exp()).abs() < 1e-5);
        let (mut tracker, camera, mut parameters, dimensions) = learned_decay_parameters();
        parameters.hidden_validity_decay_rate = None;
        parameters.visible_missed_validity_decay_rate = None;
        for millis in [40, 80] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0 * 0.5 * 0.5);
        assert!(
            tracker.filter.hypotheses[0]
                .validity_decay_evidence
                .is_none()
        );
    }

    #[test]
    fn legacy_retention_rejects_duplicate_and_backdated_exposures_before_any_mutation() {
        let (mut tracker, camera, mut parameters, dimensions) = learned_decay_parameters();
        parameters.hidden_validity_decay_rate = None;
        parameters.visible_missed_validity_decay_rate = None;
        parameters.competing_hypothesis_validity_decay_rate = None;
        parameters.near_visible_missed_validity_decay_rate = None;
        parameters.visible_missed_detection_timeout = Duration::ZERO;
        parameters.near_visible_missed_detection_timeout = Duration::ZERO;
        let ball = image_object(RobocupObjectLabel::Ball, 1.0);
        detector_frame(&mut tracker, &camera, 40, &[ball], &parameters, &dimensions);
        let before = tracker.filter.hypotheses[0].clone();
        let mut unmatched = ball;
        unmatched.bounding_box.area.min = linear_algebra::point![450.0, 240.0];
        unmatched.bounding_box.area.max = linear_algebra::point![490.0, 304.0];
        for milliseconds in [40, 20] {
            for detections in [vec![], vec![ball], vec![unmatched]] {
                detector_frame(
                    &mut tracker,
                    &camera,
                    milliseconds,
                    &detections,
                    &parameters,
                    &dimensions,
                );
                assert_eq!(tracker.filter.hypotheses.len(), 1);
                let after = &tracker.filter.hypotheses[0];
                assert_eq!(after.validity, before.validity);
                assert_eq!(after.last_seen, before.last_seen);
                assert_eq!(after.position().position, before.position().position);
                assert_eq!(after.position().velocity, before.position().velocity);
                assert_eq!(after.position_covariance(), before.position_covariance());
            }
        }
        // Admission does not prevent odometry-only motion compensation.
        for (milliseconds, x) in [(42, 0.0), (44, 0.1)] {
            tracker
                .advance(
                    Time::from_nanos(milliseconds * 1_000_000),
                    Some(Pose2::new(linear_algebra::point![x, 0.0], 0.0)),
                    None,
                    None,
                    &parameters,
                    &dimensions,
                )
                .unwrap();
        }
        let after = &tracker.filter.hypotheses[0];
        assert_eq!(after.validity, before.validity);
        assert!(
            (after.position().position.x() - before.position().position.x() + 0.1).abs() < 1e-5
        );
    }

    #[test]
    fn visibility_distinguishes_confirmed_hidden_from_uncertain_fringe_and_coverage() {
        use negative_evidence::{Visibility, classify};
        let (tracker, camera, _, dimensions) = negative_evidence_fixture();
        let mut ball = tracker.filter.hypotheses[0].position();
        assert_eq!(
            classify(&ball, &camera, dimensions.ball_radius, None),
            Visibility::Unknown
        );
        ball.position = camera
            .pixel_to_ground_with_z(linear_algebra::point![1.0, 272.0], dimensions.ball_radius)
            .unwrap();
        assert_eq!(
            classify(&ball, &camera, dimensions.ball_radius, Some(&[])),
            Visibility::Unknown
        );
        ball.position = camera
            .pixel_to_ground_with_z(
                linear_algebra::point![-100.0, 272.0],
                dimensions.ball_radius,
            )
            .unwrap();
        assert_eq!(
            classify(&ball, &camera, dimensions.ball_radius, Some(&[])),
            Visibility::Hidden
        );
    }

    fn competition_fixture() -> (Tracker, CameraMatrix, BallFilterParameters, FieldDimensions) {
        let (mut tracker, camera, mut parameters, dimensions) = learned_decay_parameters();
        parameters.hidden_validity_decay_rate = Some(0.0);
        parameters.visible_missed_validity_decay_rate = Some(0.0);
        parameters.visible_validity_exponential_decay_factor = 1.0;
        parameters.hidden_validity_exponential_decay_factor = 1.0;
        parameters.competing_hypothesis_validity_decay_rate = Some(0.5);
        let mut competitor = BallHypothesis::new(
            MultivariateNormalDistribution {
                mean: nalgebra::vector![0.5, 0.0, 0.0, 0.0],
                covariance: Matrix4::identity() * 0.01,
            },
            Time::zero(),
        );
        competitor.validity = 5.0;
        tracker.filter.hypotheses.push(competitor);
        (tracker, camera, parameters, dimensions)
    }

    #[test]
    fn sustained_observed_leader_softly_decays_only_unmatched_competitors_after_warmup() {
        for step in [20, 40] {
            let (mut tracker, camera, parameters, dimensions) = competition_fixture();
            let ball = image_object(RobocupObjectLabel::Ball, 1.0);
            for millis in (0..=1000).step_by(step) {
                detector_frame(
                    &mut tracker,
                    &camera,
                    millis,
                    &[ball],
                    &parameters,
                    &dimensions,
                );
                assert_eq!(
                    tracker.filter.hypotheses[1].validity, 5.0,
                    "warmup must not retrospectively decay competitors"
                );
            }
            for millis in ((1000 + step as i64)..=2000).step_by(step) {
                detector_frame(
                    &mut tracker,
                    &camera,
                    millis,
                    &[ball],
                    &parameters,
                    &dimensions,
                );
            }
            assert!((tracker.filter.hypotheses[1].validity - 5.0 * (-0.5_f32).exp()).abs() < 1e-4);
            assert_eq!(tracker.filter.hypotheses[1].last_seen, Time::zero());
            assert!(tracker.filter.hypotheses[0].validity > 25.0);
        }
    }

    #[test]
    fn competition_resets_on_miss_unknown_gap_and_new_leader() {
        let (mut tracker, camera, parameters, dimensions) = competition_fixture();
        let ball = image_object(RobocupObjectLabel::Ball, 1.0);
        for millis in (0..=1000).step_by(40) {
            detector_frame(
                &mut tracker,
                &camera,
                millis,
                &[ball],
                &parameters,
                &dimensions,
            );
        }
        detector_frame(&mut tracker, &camera, 1040, &[], &parameters, &dimensions);
        assert!(tracker.filter.hypotheses[0].leadership_evidence.is_none());
        detector_frame(
            &mut tracker,
            &camera,
            1080,
            &[ball],
            &parameters,
            &dimensions,
        );
        assert_eq!(
            tracker.filter.hypotheses[0]
                .leadership_evidence
                .as_ref()
                .unwrap()
                .first_match,
            Time::from_nanos(1_080_000_000)
        );
        let time = Time::from_nanos(1_120_000_000);
        tracker
            .advance_with_obstacles(
                time,
                None,
                Some(&[ball]),
                Some(&TimeWrapper {
                    time,
                    inner: camera.clone(),
                }),
                None,
                &parameters,
                &dimensions,
            )
            .unwrap();
        assert!(tracker.filter.hypotheses[0].leadership_evidence.is_none());
        detector_frame(
            &mut tracker,
            &camera,
            5000,
            &[ball],
            &parameters,
            &dimensions,
        );
        detector_frame(
            &mut tracker,
            &camera,
            10_000,
            &[ball],
            &parameters,
            &dimensions,
        );
        assert_eq!(
            tracker.filter.hypotheses[0]
                .leadership_evidence
                .as_ref()
                .unwrap()
                .first_match,
            Time::from_nanos(10_000_000_000)
        );
        assert_eq!(tracker.filter.hypotheses[1].validity, 5.0);
        tracker.filter.hypotheses[1].validity = 100.0;
        let mut second_ball = ball;
        let second_center = camera
            .ground_with_z_to_pixel(linear_algebra::point![0.5, 0.0], dimensions.ball_radius)
            .unwrap();
        second_ball.bounding_box.area = geometry::rectangle::Rectangle {
            min: second_center - linear_algebra::vector![2.0, 2.0],
            max: second_center + linear_algebra::vector![2.0, 2.0],
        };
        let old_leader_validity = tracker.filter.hypotheses[0].validity;
        detector_frame(
            &mut tracker,
            &camera,
            10_040,
            &[second_ball],
            &parameters,
            &dimensions,
        );
        assert!(tracker.filter.hypotheses[0].leadership_evidence.is_none());
        assert_eq!(
            tracker.filter.hypotheses[1]
                .leadership_evidence
                .as_ref()
                .unwrap()
                .first_match,
            Time::from_nanos(10_040_000_000)
        );
        assert_eq!(tracker.filter.hypotheses[0].validity, old_leader_validity);
    }

    #[test]
    fn current_matches_and_disabled_competition_remain_protected() {
        for rate in [None, Some(0.0), Some(0.5)] {
            let (mut tracker, camera, mut parameters, dimensions) = competition_fixture();
            parameters.competing_hypothesis_validity_decay_rate = rate;
            let first = image_object(RobocupObjectLabel::Ball, 1.0);
            let mut second = first;
            let center = camera
                .ground_with_z_to_pixel(linear_algebra::point![0.5, 0.0], dimensions.ball_radius)
                .unwrap();
            second.bounding_box.area = geometry::rectangle::Rectangle {
                min: center - linear_algebra::vector![2.0, 2.0],
                max: center + linear_algebra::vector![2.0, 2.0],
            };
            for millis in (0..=2000).step_by(40) {
                let detections = if rate == Some(0.5) {
                    vec![first, second]
                } else {
                    vec![first]
                };
                detector_frame(
                    &mut tracker,
                    &camera,
                    millis,
                    &detections,
                    &parameters,
                    &dimensions,
                );
            }
            if rate == Some(0.5) {
                assert!(
                    tracker.filter.hypotheses[1].validity > 50.0,
                    "a second actually seen ball must remain viable"
                );
            } else {
                assert_eq!(tracker.filter.hypotheses[1].validity, 5.0);
            }
        }
    }

    #[test]
    fn near_clear_misses_have_learnable_cadence_independent_decay_and_fixed_expiry() {
        for rate in [None, Some(0.0), Some(20.0), Some(40.0)] {
            for cadence in [20, 40] {
                let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
                parameters.visible_missed_detection_timeout = Duration::from_secs(1);
                parameters.near_visible_missed_detection_timeout = Duration::from_millis(120);
                parameters.near_visible_missed_detection_distance = 1.0;
                parameters.near_visible_missed_validity_decay_rate = rate;
                parameters.visible_missed_validity_decay_rate = Some(1.0);
                for millis in (40..=120).step_by(cadence) {
                    detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
                }
                let expected = 25.0 * (-(1.0 + rate.unwrap_or(0.0)) * 0.08_f32).exp();
                assert!((tracker.filter.hypotheses[0].validity - expected).abs() < 1e-4);
                detector_frame(&mut tracker, &camera, 160, &[], &parameters, &dimensions);
                assert!(
                    tracker.filter.hypotheses.is_empty(),
                    "hard expiry is independent of learned rate"
                );
            }
        }
    }

    #[test]
    fn near_clear_clock_resets_on_occlusion_unknown_geometry_gaps_and_real_match() {
        for interruption in ["robot", "missing obstacles", "stale camera", "gap", "match"] {
            let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
            parameters.visible_missed_detection_timeout = Duration::ZERO;
            parameters.near_visible_missed_detection_timeout = Duration::from_millis(120);
            parameters.near_visible_missed_detection_distance = 1.0;
            parameters.near_visible_missed_validity_decay_rate = Some(20.0);
            parameters.visible_missed_validity_decay_rate = Some(0.0);
            parameters.hidden_validity_decay_rate = Some(0.0);
            for millis in [40, 80, 120] {
                detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
            }
            let before = tracker.filter.hypotheses[0].validity;
            let mut next = 200;
            match interruption {
                "robot" => {
                    robot_obstacle_frame(&mut tracker, &camera, 160, &parameters, &dimensions)
                }
                "missing obstacles" | "stale camera" => {
                    let time = Time::from_nanos(160_000_000);
                    let camera_time = if interruption == "stale camera" {
                        Time::zero()
                    } else {
                        time
                    };
                    tracker
                        .advance_with_obstacles(
                            time,
                            None,
                            Some(&[]),
                            Some(&TimeWrapper {
                                time: camera_time,
                                inner: camera.clone(),
                            }),
                            None,
                            &parameters,
                            &dimensions,
                        )
                        .unwrap();
                }
                "gap" => next = 1000,
                "match" => detector_frame(
                    &mut tracker,
                    &camera,
                    160,
                    &[image_object(RobocupObjectLabel::Ball, 1.0)],
                    &parameters,
                    &dimensions,
                ),
                _ => unreachable!(),
            }
            detector_frame(&mut tracker, &camera, next, &[], &parameters, &dimensions);
            if interruption != "match" {
                assert!(
                    (tracker.filter.hypotheses[0].validity - before).abs() < 1e-5,
                    "{interruption}"
                );
            }
            let evidence = tracker.filter.hypotheses[0]
                .negative_evidence
                .as_ref()
                .unwrap();
            assert_eq!(
                evidence.near.as_ref().unwrap().duration,
                Duration::ZERO,
                "{interruption}"
            );
            for millis in [next + 40, next + 80] {
                detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
                assert_eq!(tracker.filter.hypotheses.len(), 1);
            }
            detector_frame(
                &mut tracker,
                &camera,
                next + 120,
                &[],
                &parameters,
                &dimensions,
            );
            assert!(tracker.filter.hypotheses.is_empty());
        }
    }

    #[test]
    fn approaching_ball_does_not_inherit_far_misses_and_leaving_near_resets_clock() {
        let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
        parameters.visible_missed_detection_timeout = Duration::from_secs(1);
        parameters.near_visible_missed_detection_timeout = Duration::from_millis(120);
        parameters.near_visible_missed_detection_distance = 0.1;
        parameters.near_visible_missed_validity_decay_rate = Some(20.0);
        let set_distance = |tracker: &mut Tracker, distance| {
            let BallMode::Moving(state) = &mut tracker.filter.hypotheses[0].mode else {
                panic!("moving fixture")
            };
            state.mean.x = distance;
        };
        set_distance(&mut tracker, 0.2);
        for millis in [40, 80, 120, 160, 200] {
            detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
        }
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        set_distance(&mut tracker, 0.0);
        detector_frame(&mut tracker, &camera, 240, &[], &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
        detector_frame(&mut tracker, &camera, 280, &[], &parameters, &dimensions);
        let before = tracker.filter.hypotheses[0].validity;
        set_distance(&mut tracker, 0.2);
        detector_frame(&mut tracker, &camera, 320, &[], &parameters, &dimensions);
        set_distance(&mut tracker, 0.0);
        detector_frame(&mut tracker, &camera, 360, &[], &parameters, &dimensions);
        assert_eq!(tracker.filter.hypotheses[0].validity, before);
        assert_eq!(
            tracker.filter.hypotheses[0]
                .negative_evidence
                .as_ref()
                .unwrap()
                .near
                .as_ref()
                .unwrap()
                .duration,
            Duration::ZERO
        );
    }

    #[test]
    fn disabled_near_policy_retains_legacy_behavior_even_with_rate_configured() {
        for (timeout, distance) in [(Duration::ZERO, 1.0), (Duration::from_millis(120), 0.0)] {
            let (mut tracker, camera, mut parameters, dimensions) = negative_evidence_fixture();
            parameters.visible_missed_detection_timeout = Duration::ZERO;
            parameters.near_visible_missed_detection_timeout = timeout;
            parameters.near_visible_missed_detection_distance = distance;
            parameters.near_visible_missed_validity_decay_rate = Some(40.0);
            for millis in (40..1000).step_by(40) {
                detector_frame(&mut tracker, &camera, millis, &[], &parameters, &dimensions);
            }
            assert_eq!(tracker.filter.hypotheses[0].validity, 25.0);
            assert!(tracker.filter.hypotheses[0].negative_evidence.is_none());
        }
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
