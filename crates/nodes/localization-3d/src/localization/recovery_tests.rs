use super::*;
use linear_algebra::{IntoTransform, Vector3, point, vector};
use nalgebra::{Translation3, UnitQuaternion};
use projection::intrinsic::Intrinsic;
use std::time::Duration;
use types::{
    odometry::KinematicOdometryDelta, visual_localization::FieldMarkAssociation,
    visual_odometry::VisualOdometryDelta,
};

fn camera() -> CameraGeometry {
    CameraGeometry {
        robot_to_camera: nalgebra::Isometry3::from_parts(
            Translation3::new(0.12, -0.04, 0.2),
            UnitQuaternion::from_euler_angles(std::f32::consts::PI, 0.0, 0.0),
        )
        .inverse()
        .framed_transform(),
        intrinsics: Intrinsic::new(nalgebra::vector![300.0, 300.0], point![160.0, 120.0]),
    }
}

fn turn(t: f64) -> (f64, f64) {
    let u = ((t - 3.0) / 0.6).clamp(0.0, 1.0);
    (0.6 * u * u * (3.0 - 2.0 * u), 6.0 * u * (1.0 - u))
}

fn truth(t: f64) -> nalgebra::Isometry3<f64> {
    nalgebra::Isometry3::from_parts(
        Translation3::new(-2.0 + 0.9 * t, 1.0, 0.55),
        UnitQuaternion::from_euler_angles(0.0, 0.0, 0.4 + turn(t).0),
    )
}

fn frame(t: f64, half_turn: bool) -> TimeWrapper<VisualLocalizationFrame> {
    let camera = camera();
    TimeWrapper {
        time: Time::from_nanos(1_000_000_000 + (t * 1e9).round() as i64),
        inner: VisualLocalizationFrame {
            epoch: 7,
            source: VisualAssociationSource::Global,
            generation: if t == 0.0 { 0 } else { 1 },
            robot_to_camera: camera.robot_to_camera,
            camera_intrinsic: camera.intrinsics,
            associations: [
                point![-3.0, 0.0, 0.0],
                point![3.0, 0.0, 0.0],
                point![0.0, 3.0, 0.0],
            ]
            .map(|field_point| {
                let p = camera.robot_to_camera.inner
                    * truth(t).cast::<f32>().inverse()
                    * field_point.inner;
                FieldMarkAssociation {
                    field_point: if half_turn {
                        point![-field_point.x(), -field_point.y(), 0.0]
                    } else {
                        field_point
                    },
                    detection: camera.intrinsics.project(Vector3::wrap(p.coords)),
                }
            })
            .to_vec(),
        },
    }
}

fn ingest_motion(localization: &mut Localization, index: u64, with_vo: bool) {
    let t = index as f64 * 0.01;
    let time = Time::from_nanos(1_000_000_000) + Duration::from_millis(index * 10);
    let (yaw, rate) = turn(t);
    localization
        .ingest_imu(
            time,
            ImuState {
                // Deliberately different IMU and Local yaw zeros; only changes are observations.
                roll_pitch_yaw: vector![0.0, 0.0, (2.7 + yaw) as f32],
                angular_velocity: vector![0.0, 0.0, rate as f32],
                ..Default::default()
            },
        )
        .unwrap();
    if index == 0 {
        return;
    }
    let previous_time = time - Duration::from_millis(10);
    let previous = truth(t - 0.01);
    let delta = previous.inverse() * truth(t);
    assert!(
        localization
            .ingest_kinematic_odometry(KinematicOdometryDelta {
                previous_time,
                time,
                current_to_previous: nalgebra::Isometry2::new(
                    delta.translation.vector.xy().cast(),
                    0.0
                )
                .framed_transform(),
            })
            .unwrap()
    );
    if with_vo {
        let camera = camera();
        let camera_delta = camera.robot_to_camera.inner
            * delta.cast::<f32>()
            * camera.robot_to_camera.inner.inverse();
        assert!(
            localization
                .ingest_visual_odometry(
                    VisualOdometer {
                        time,
                        epoch: 4,
                        delta: Some(VisualOdometryDelta {
                            previous_time,
                            current_left_camera_to_previous_left_camera: camera_delta
                        }),
                        current_left_camera_to_visual_odometer: nalgebra::Isometry3::identity(),
                    },
                    Some(&camera),
                    Some(&camera)
                )
                .unwrap()
        );
    }
}

fn new_localization() -> Localization {
    let parameters = Localization3dParameters {
        timing: Default::default(),
        model: Default::default(),
        solver: Default::default(),
        visual: Default::default(),
        inputs: Default::default(),
        imu_preintegration: Default::default(),
        imu_bias: Default::default(),
        kinematic_odometry_noise: Some(Default::default()),
        accelerometer: None,
        initial_height_sigma: 1.0,
        initial_velocity_sigma: 5.0,
        recovery_height_gate: 9.0,
        max_tilt_error: 20.0_f64.to_radians(),
        accelerometer_process_noise_variance: 10.0,
        visual_feature_noise_variance: 100.0,
        field_containment_sigma: 1.0,
        max_heading_error: 20.0_f64.to_radians(),
        max_heading_reference_drift_per_second: 0.5_f64.to_radians(),
        tracking_timeout: Duration::from_secs(2),
        visual_tracking_timeout: Duration::from_secs(2),
    };
    new_localization_with_parameters(&parameters)
}

fn new_localization_with_parameters(parameters: &Localization3dParameters) -> Localization {
    let initial = frame(0.0, false);
    Localization::new(
        initial.time,
        7,
        parameters,
        &FieldDimensions::SPL_2025,
        &camera(),
        nalgebra::Isometry3::from_parts(
            Translation3::new(0.0, 0.0, 0.55),
            UnitQuaternion::from_euler_angles(0.0, 0.0, 2.0),
        )
        .framed_transform(),
    )
    .unwrap()
}

#[test]
#[ignore = "release timing benchmark, run with --ignored --nocapture"]
fn benchmark_tracking_estimation() {
    let parameters = json5::from_str(include_str!(
        "../../../../../etc/parameters/base/localization3d.json5"
    ))
    .unwrap();
    let mut localization = new_localization_with_parameters(&parameters);
    let camera = camera();
    let mut cycles = Vec::new();
    let mut ingestion = Duration::ZERO;
    let mut total_ingestion = Duration::ZERO;
    let mut fields = 0;
    let mut failures = 0;
    let mut max_position_error = 0.0_f64;
    let mut max_rotation_error = 0.0_f64;
    for index in 0..=3000 {
        let t = index as f64 * 0.002;
        let time = Time::from_nanos(1_000_000_000 + index * 2_000_000);
        let (yaw, rate) = turn(t);
        let imu = ImuState {
            roll_pitch_yaw: vector![0.0, 0.0, (2.7 + yaw) as f32],
            angular_velocity: vector![0.0, 0.0, rate as f32],
            linear_acceleration: vector![0.0, 0.0, 9.81],
        };
        let vo = (index > 0 && index % 10 == 0).then(|| VisualOdometer {
            time,
            epoch: 0,
            delta: Some(VisualOdometryDelta {
                previous_time: time - Duration::from_millis(20),
                current_left_camera_to_previous_left_camera: camera.robot_to_camera.inner
                    * (truth(t - 0.02).inverse() * truth(t)).cast::<f32>()
                    * camera.robot_to_camera.inner.inverse(),
            }),
            current_left_camera_to_visual_odometer: nalgebra::Isometry3::identity(),
        });
        let visual = (index % 50 == 0).then(|| {
            let mut frame = frame(t, false);
            frame.inner.generation = localization.status().generation;
            if localization.status().state == LocalizationState::Tracking {
                frame.inner.source = VisualAssociationSource::Tracking;
            }
            frame
        });
        let started = std::time::Instant::now();
        localization.ingest_imu(time, imu).unwrap();
        if let Some(vo) = vo {
            localization
                .ingest_visual_odometry(vo, Some(&camera), Some(&camera))
                .unwrap();
        }
        if let Some(frame) = visual {
            localization
                .ingest_visual_localization_frame(frame)
                .unwrap();
        }
        let elapsed = started.elapsed();
        ingestion += elapsed;
        total_ingestion += elapsed;
        if index % 25 != 0 {
            continue;
        }
        let output = localization.solve(time);
        cycles.push(ingestion + output.diagnostics.estimation_duration);
        ingestion = Duration::ZERO;
        failures += usize::from(output.diagnostics.failure.is_some());
        fields += usize::from(output.estimate.is_some_and(|e| e.robot_to_field.is_some()));
        if let Some(estimate) = output.estimate
            && let Some(field) = estimate.robot_to_field
        {
            let expected = truth((estimate.time.as_nanos() - 1_000_000_000) as f64 * 1e-9);
            max_position_error = max_position_error
                .max((field.pose.inner.translation.vector - expected.translation.vector).norm());
            max_rotation_error =
                max_rotation_error.max(field.pose.inner.rotation.angle_to(&expected.rotation));
        }
    }
    let cycle_count = cycles.len();
    eprintln!("tracking field estimates: {fields}/{}", cycles.len());
    eprintln!(
        "tracking max position error={max_position_error:.6} m, rotation error={:.6} deg",
        max_rotation_error.to_degrees()
    );
    super::flight_recording_tests::report_timing(
        "synthetic-tracking",
        cycles,
        6.0,
        total_ingestion,
        failures,
    );
    assert!(fields > cycle_count * 9 / 10, "{fields} field estimates");
}

fn lost_localization(with_vo: bool) -> (Localization, LocalizationEstimate) {
    let mut localization = new_localization();
    let mut latest = None;
    assert!(
        localization
            .ingest_visual_localization_frame(frame(0.0, false))
            .unwrap()
    );
    for index in 0..=360 {
        ingest_motion(&mut localization, index, with_vo);
        if index > 0 && index % 5 == 0 {
            let time = Time::from_nanos(1_000_000_000) + Duration::from_millis(index * 10);
            let solved = localization.solve(time);
            assert!(solved.estimate.is_some(), "{:?}", solved.diagnostics);
            latest = solved.estimate;
            if index == 20 {
                assert_eq!(localization.status.state, LocalizationState::Tracking);
            }
        }
    }
    assert_eq!(localization.status.state, LocalizationState::LostTrack);
    (localization, latest.unwrap())
}

#[test]
fn startup_preserves_delayed_imu_brackets_and_waits_for_missing_samples() {
    for delayed in [false, true] {
        let mut localization = new_localization();
        let last = if delayed { 20 } else { 5 };
        for index in 0..=last {
            ingest_motion(&mut localization, index, false);
        }
        let mut startup = frame(0.055, false);
        startup.inner.generation = 0;
        assert!(
            localization
                .ingest_visual_localization_frame(startup)
                .unwrap()
        );
        if !delayed {
            localization.solve(Time::from_nanos(1_055_000_000));
            assert_eq!(localization.status.state, LocalizationState::Startup);
            assert!(localization.pending_bootstrap.is_some());
            for index in 6..=20 {
                ingest_motion(&mut localization, index, false);
            }
        }
        let output = localization.solve(Time::from_nanos(1_200_000_000));
        assert_eq!(
            localization.status.state,
            LocalizationState::Tracking,
            "{:?}",
            output.diagnostics
        );
        let estimate = output.estimate.unwrap();
        assert_eq!(estimate.time, Time::from_nanos(1_200_000_000));
        assert!(
            (estimate
                .robot_to_field
                .unwrap()
                .pose
                .inner
                .translation
                .vector
                - truth(0.2).translation.vector)
                .norm()
                < 0.03
        );
        let status = localization.status();
        let snapshot = status
            .heading
            .expect("publish trusted reference with lifecycle");
        assert!(
            snapshot.time < status.time,
            "reference time is not the state-entry time"
        );
    }
}

#[test]
fn delayed_recovery_preserves_motion_and_publishes_only_after_acceptance() {
    for with_vo in [false, true] {
        let (mut localization, before) = lost_localization(with_vo);
        // Between IMU samples, and inside both odometry intervals. The turn ends
        // before delivery, so later zero-rate IMU readings cannot reconstruct it.
        let recovery = frame(3.375, true);
        assert!(
            localization
                .ingest_visual_localization_frame(recovery.clone())
                .unwrap()
        );
        assert_eq!(localization.status.state, LocalizationState::LostTrack);
        assert_eq!(localization.estimator.latest_time(), before.time);
        assert_eq!(localization.status().generation, before.generation);
        ingest_motion(&mut localization, 361, with_vo);
        let now = Time::from_nanos(4_610_000_000);
        let solved = localization.solve(now);
        assert_eq!(
            localization.status.state,
            LocalizationState::Tracking,
            "{:?}",
            solved.diagnostics
        );
        let estimate = solved.estimate.unwrap();
        assert_eq!(estimate.time, now);
        assert_ne!(estimate.generation, before.generation);
        assert_eq!(estimate.generation, localization.status().generation);
        let pose = estimate.robot_to_field.unwrap().pose.inner;
        assert!(
            (pose.translation.vector - truth(3.61).translation.vector).norm() < 0.03,
            "{pose:?}"
        );
        assert!(
            pose.rotation.angle_to(&truth(3.61).rotation) < 0.03,
            "{pose:?}"
        );
        assert!(
            pose.translation.vector.x > 0.0,
            "recovery changed field half"
        );
        let mut stale_tracking = recovery.clone();
        stale_tracking.inner.source = VisualAssociationSource::Tracking;
        assert!(
            !localization
                .ingest_visual_localization_frame(stale_tracking)
                .unwrap()
        );
        assert!(
            !localization
                .ingest_visual_localization_frame(recovery)
                .unwrap()
        );

        // Continued motion and branch-specific tracking remain usable after replacement.
        for index in 362..=370 {
            ingest_motion(&mut localization, index, with_vo);
        }
        let mut tracking = frame(3.7, false);
        tracking.inner.generation = localization.status.generation;
        tracking.inner.source = VisualAssociationSource::Tracking;
        assert!(
            localization
                .ingest_visual_localization_frame(tracking)
                .unwrap()
        );
        let solved = localization.solve(Time::from_nanos(4_700_000_000));
        let pose = solved.estimate.unwrap().robot_to_field.unwrap().pose.inner;
        assert!((pose.translation.vector - truth(3.7).translation.vector).norm() < 0.03);
        assert_eq!(localization.status.state, LocalizationState::Tracking);
    }
}

#[test]
fn rejected_recovery_does_not_replace_the_active_trajectory() {
    let (mut localization, estimate) = lost_localization(false);
    let now = estimate.time;
    let before = estimate.robot_to_local.pose.inner;
    let mut recovery = frame(3.375, true);
    recovery.inner.associations[0].detection.inner.x += 200.0;
    assert!(
        localization
            .ingest_visual_localization_frame(recovery)
            .unwrap()
    );
    let solved = localization.solve(now);
    assert_eq!(localization.status.state, LocalizationState::LostTrack);
    let after = solved.estimate.unwrap();
    assert_eq!(after.time, now);
    assert!(
        (after.robot_to_local.pose.inner.translation.vector - before.translation.vector).norm()
            < 0.001
    );
    assert!(
        after
            .robot_to_local
            .pose
            .inner
            .rotation
            .angle_to(&before.rotation)
            < 0.001
    );
    assert!(
        localization
            .ingest_visual_localization_frame(frame(3.375, true))
            .unwrap()
    );
    localization.solve(now);
    assert_eq!(localization.status.state, LocalizationState::Tracking);
}

#[test]
fn wrong_half_tracking_update_is_removed_before_publication_or_marginalization() {
    let (mut localization, _) = lost_localization(false);
    let now = Time::from_nanos(4_600_000_000);
    localization
        .ingest_visual_localization_frame(frame(3.375, true))
        .unwrap();
    localization.solve(now);
    assert_eq!(localization.status.state, LocalizationState::Tracking);
    let reference = localization.heading_reference.unwrap();
    for index in 361..=370 {
        ingest_motion(&mut localization, index, false);
    }
    let mut wrong = frame(3.7, true);
    wrong.inner.generation = localization.status.generation;
    wrong.inner.source = VisualAssociationSource::Tracking;
    assert!(
        localization
            .ingest_visual_localization_frame(wrong)
            .unwrap()
    );
    let rejected = localization.solve(Time::from_nanos(4_700_000_000));
    assert!(rejected.estimate.is_some(), "{:?}", rejected.diagnostics);
    assert!(!rejected.diagnostics.motion_rebuilt);
    assert!(localization.pending_visual.is_none());
    assert_eq!(
        localization.latest_visual,
        Some(Time::from_nanos(4_375_000_000))
    );
    // The failed frame is no longer in the graph: a motion-only solve recovers
    // immediately, and subsequent marginalization cannot freeze its bad factors.
    for index in 371..=600 {
        ingest_motion(&mut localization, index, false);
        if index % 5 == 0 {
            let output = localization
                .solve(Time::from_nanos(1_000_000_000) + Duration::from_millis(index * 10));
            let pose = output
                .estimate
                .expect("motion graph remains usable")
                .robot_to_field
                .unwrap()
                .pose
                .inner;
            assert!(pose.rotation.angle_to(&truth(index as f64 * 0.01).rotation) < 0.03);
        }
    }
    let imu = linear_algebra::Orientation3::from_euler_angles(0.0, 0.0, 3.3);
    assert!(
        reference
            .expected(imu)
            .rotation_to(localization.heading_reference.unwrap().expected(imu))
            .inner
            .angle()
            .abs()
            < 1e-12
    );
}

#[test]
fn double_flight_fuses_zero_specific_force_without_ground_contact() {
    let mut localization = new_localization();
    let mut parameters = localization.parameters.clone();
    parameters.accelerometer = Some(crate::AccelerometerParameters {
        noise_density: 0.03,
        ..Default::default()
    });
    parameters.kinematic_odometry_noise = None;
    parameters.visual_feature_noise_variance = 4.0;
    localization = new_localization_with_parameters(&parameters);
    let flying_pose = |t: f64| {
        let mut pose = truth(t);
        pose.translation.z = 0.55 + 2.0 * t - 0.5 * 9.81 * t * t;
        pose
    };
    let camera = camera();
    let mut feet = RobotKinematics::default();
    feet.left_leg.sole_to_robot = Isometry3::from_translation(0.0, 0.05, -0.3);
    feet.right_leg.sole_to_robot = Isometry3::from_translation(0.0, -0.05, -0.3);
    let mut observed_peak = 0.0_f64;
    for index in 0..=200 {
        let t = index as f64 * 0.002;
        let time = Time::from_nanos(1_000_000_000 + index * 2_000_000);
        localization
            .ingest_imu(
                time,
                ImuState {
                    roll_pitch_yaw: vector![0.0, 0.0, 2.7],
                    // Specific force is zero in flight, not +g or a missing measurement.
                    linear_acceleration: Vector3::zeros(),
                    ..Default::default()
                },
            )
            .unwrap();
        if index % 10 != 0 {
            continue;
        }
        localization
            .ingest_kinematics(TimeWrapper {
                time,
                inner: feet.clone(),
            })
            .unwrap();
        if index > 0 {
            let delta = flying_pose(t - 0.02).inverse() * flying_pose(t);
            localization
                .ingest_visual_odometry(
                    VisualOdometer {
                        time,
                        epoch: 1,
                        delta: Some(VisualOdometryDelta {
                            previous_time: time - Duration::from_millis(20),
                            current_left_camera_to_previous_left_camera: camera
                                .robot_to_camera
                                .inner
                                * delta.cast::<f32>()
                                * camera.robot_to_camera.inner.inverse(),
                        }),
                        current_left_camera_to_visual_odometer: nalgebra::Isometry3::identity(),
                    },
                    Some(&camera),
                    Some(&camera),
                )
                .unwrap();
        }
        if index % 50 == 0 {
            let mut image = frame(t, false);
            image.inner.generation = localization.status().generation;
            image.inner.source = if localization.status.state == LocalizationState::Tracking {
                VisualAssociationSource::Tracking
            } else {
                VisualAssociationSource::Global
            };
            for a in &mut image.inner.associations {
                let p = camera.robot_to_camera.inner
                    * flying_pose(t).cast::<f32>().inverse()
                    * a.field_point.inner;
                a.detection = camera.intrinsics.project(Vector3::wrap(p.coords));
            }
            localization
                .ingest_visual_localization_frame(image)
                .unwrap();
        }
        let output = localization.solve(time);
        if let Some(pose) = output.estimate.and_then(|e| e.robot_to_field) {
            let z = pose.pose.translation().z();
            assert!(
                (z - flying_pose(t).translation.z).abs() < 0.05,
                "t={t}, z={z}, {:?}",
                output.diagnostics
            );
            observed_peak = observed_peak.max(z);
        }
    }
    assert_eq!(localization.status.state, LocalizationState::Tracking);
    assert!(
        observed_peak > 0.72,
        "the estimator must allow both soles above the floor"
    );
}

#[test]
fn field_inconsistency_does_not_stop_local_motion_or_forget_heading() {
    let (mut localization, before) = lost_localization(true);
    // Simulate a field solution inconsistent with the independent reference.
    let reference = HeadingReference::new(
        before.time,
        linear_algebra::Orientation2::new(1.8),
        linear_algebra::Orientation3::from_euler_angles(0.0, 0.0, 3.3),
    );
    localization.heading_reference = Some(reference);
    ingest_motion(&mut localization, 361, true);
    let output = localization.solve(Time::from_nanos(4_610_000_000));
    assert!(
        output.diagnostics.motion_rebuilt,
        "{:?}",
        output.diagnostics
    );
    let estimate = output.estimate.unwrap();
    assert!(estimate.robot_to_field.is_none());
    assert_eq!(estimate.generation, before.generation);
    assert_eq!(localization.status.state, LocalizationState::LostTrack);
    for index in 362..=380 {
        ingest_motion(&mut localization, index, true);
        let output =
            localization.solve(Time::from_nanos(1_000_000_000) + Duration::from_millis(index * 10));
        assert!(output.estimate.is_some(), "{:?}", output.diagnostics);
        assert!(!output.diagnostics.motion_rebuilt);
    }
    let imu = linear_algebra::Orientation3::from_euler_angles(0.0, 0.0, 3.3);
    assert!(
        reference
            .expected(imu)
            .rotation_to(localization.heading_reference.unwrap().expected(imu))
            .angle()
            .abs()
            < 1e-12
    );
}
