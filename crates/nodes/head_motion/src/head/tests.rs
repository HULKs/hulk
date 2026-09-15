use json5::from_str;
use kinematics::{
    forward::{head_to_left_camera, head_to_robot},
    joints::head::HeadJoint,
};
use linear_algebra::{Isometry3, Rotation2, nalgebra, vector};
use projection::camera_matrix::CameraMatrix;
use serde::Deserialize;

use super::*;

struct Fixture {
    parameters: Parameters,
    joints: JointLimits,
    field: FieldDimensions,
    camera: CameraMatrix,
}

impl Fixture {
    fn new() -> Self {
        #[derive(Deserialize)]
        struct Global {
            joint_limits: JointLimits,
            field_dimensions: FieldDimensions,
        }
        let global: Global = from_str(include_str!(
            "../../../../../etc/parameters/base/global.json5"
        ))
        .unwrap();
        Self {
            parameters: from_str(include_str!(
                "../../../../../etc/parameters/base/head_motion.json5"
            ))
            .unwrap(),
            joints: global.joint_limits,
            field: global.field_dimensions,
            camera: CameraMatrix::from_normalized_focal_and_center(
                nalgebra::vector![0.55, 0.65],
                nalgebra::point![0.5, 0.5],
                vector![640.0, 480.0],
                Isometry3::identity(),
                head_to_robot(&HeadJoints::default()).inverse(),
                head_to_left_camera(-0.2),
            ),
        }
    }

    fn context(&self) -> HeadContext<'_> {
        HeadContext {
            geometry: Some(GazeGeometry {
                camera_matrix: &self.camera,
                ground_to_robot: Isometry3::from_translation(0.0, 0.0, -0.55),
            }),
            joint_limits: Some(&self.joints),
            field_dimensions: Some(&self.field),
            field_side: Some(GlobalFieldSide::Home),
        }
    }
}

fn at(millis: u64) -> Time {
    Time::zero() + Duration::from_millis(millis)
}

fn observation(yaw: f32, pitch: f32) -> HeadObservation {
    HeadObservation {
        positions: HeadJoints { yaw, pitch },
        velocities: HeadJoints::fill(0.0),
    }
}

fn commanded(output: &HeadOutput) -> HeadObservation {
    let commands = &output.joint_control.commands;
    HeadObservation {
        positions: HeadJoints {
            yaw: commands.yaw.position,
            pitch: commands.pitch.position,
        },
        velocities: HeadJoints {
            yaw: commands.yaw.velocity,
            pitch: commands.pitch.velocity,
        },
    }
}

fn requested(output: &HeadOutput) -> HeadJoints<f32> {
    output.joint_control.progress.unwrap().requested_target
}

fn assert_framing(
    fixture: &Fixture,
    output: &HeadOutput,
    target: Point2<Ground>,
    height: f32,
    region: ImageRegion,
) {
    assert!(output.hold_reason.is_none());
    let geometry = fixture.context().geometry.unwrap();
    let camera_point = fixture
        .camera
        .ground_to_left_camera_at(&requested(output), geometry.ground_to_robot)
        * point![target.x(), target.y(), height];
    let pixel = fixture.camera.intrinsics.project(camera_point.coords());
    let region = match region {
        ImageRegion::Center => fixture.parameters.image_region_parameters.center,
        ImageRegion::Top => fixture.parameters.image_region_parameters.top,
        ImageRegion::Bottom => fixture.parameters.image_region_parameters.bottom,
    };
    assert!((pixel - point![region.x() * 640.0, region.y() * 480.0]).norm() < 0.05);
}

#[test]
fn center_uses_current_field_width_and_requested_framing() {
    let mut fixture = Fixture::new();
    let mut controller = HeadController::default();
    let mut center_pitch = Vec::new();
    for width in [4.0, 8.0] {
        fixture.field.width = width;
        for region in [ImageRegion::Center, ImageRegion::Top, ImageRegion::Bottom] {
            controller.observe(observation(0.0, 0.0), at(0)).unwrap();
            let output = controller
                .evaluate(
                    &HeadMotion::Center {
                        image_region_target: region,
                    },
                    &fixture.context(),
                    &fixture.parameters,
                    at(0),
                )
                .unwrap();
            assert_framing(&fixture, &output, point![width / 2.0, 0.0], 0.0, region);
            if region == ImageRegion::Center {
                center_pitch.push(requested(&output).pitch);
            }
        }
    }
    assert!(
        (center_pitch[0] - center_pitch[1]).abs() > 0.05,
        "Center must not use a fixed head pitch"
    );
}

#[test]
fn gaze_requests_preserve_height_and_glancing_consumes_current_target_progress() {
    let fixture = Fixture::new();
    let mut controller = HeadController::default();
    let target = point![2.0, 0.3];
    for height in [0.0, 0.105, 0.5] {
        controller.observe(observation(0.0, 0.0), at(0)).unwrap();
        let output = controller
            .evaluate(
                &HeadMotion::LookAt {
                    target,
                    height_above_ground: height,
                    image_region_target: ImageRegion::Top,
                },
                &fixture.context(),
                &fixture.parameters,
                at(0),
            )
            .unwrap();
        assert_framing(&fixture, &output, target, height, ImageRegion::Top);
    }
    let request = HeadMotion::LookLeftAndRightOf {
        target,
        height_above_ground: 0.105,
    };
    let first = controller
        .evaluate(&request, &fixture.context(), &fixture.parameters, at(0))
        .unwrap();
    let left = Rotation2::new(fixture.parameters.glance.angle) * target;
    assert_framing(&fixture, &first, left, 0.105, ImageRegion::Center);
    controller
        .observe(
            HeadObservation {
                positions: requested(&first),
                velocities: HeadJoints::fill(0.2),
            },
            at(10),
        )
        .unwrap();
    let arrived = controller
        .evaluate(&request, &fixture.context(), &fixture.parameters, at(10))
        .unwrap();
    assert!(arrived.joint_control.progress.unwrap().position_reached);
    assert!(!arrived.joint_control.progress.unwrap().target_reached);
    let moved = point![2.1, 0.4];
    controller.observe(commanded(&arrived), at(20)).unwrap();
    let next = controller
        .evaluate(
            &HeadMotion::LookLeftAndRightOf {
                target: moved,
                height_above_ground: 0.3,
            },
            &fixture.context(),
            &fixture.parameters,
            at(20),
        )
        .unwrap();
    let right = Rotation2::new(-fixture.parameters.glance.angle) * moved;
    assert_framing(&fixture, &next, right, 0.3, ImageRegion::Center);
}

#[test]
fn geometry_loss_captures_reference_once_then_resumes_and_reseeds_after_inactivity() {
    let fixture = Fixture::new();
    let request = HeadMotion::LookAt {
        target: point![2.0, 1.0],
        height_above_ground: 0.105,
        image_region_target: ImageRegion::Center,
    };
    let mut controller = HeadController::default();
    let mut measured = observation(0.0, 0.2);
    let mut previous = None;
    for millis in (0..200).step_by(10) {
        controller.observe(measured, at(millis)).unwrap();
        let output = controller
            .evaluate(
                &request,
                &fixture.context(),
                &fixture.parameters,
                at(millis),
            )
            .unwrap();
        measured = commanded(&output);
        previous = Some(output);
    }
    let previous = previous.unwrap();
    let captured = commanded(&previous).positions;
    let mut missing_geometry = fixture.context();
    missing_geometry.geometry = None;
    let mut old_reference = previous.joint_control.reference;
    for millis in (200..1000).step_by(10) {
        // Measurements may change during hold; the target must remain the captured reference.
        controller
            .observe(observation(-0.1, 0.1), at(millis))
            .unwrap();
        let held = controller
            .evaluate(&request, &missing_geometry, &fixture.parameters, at(millis))
            .unwrap();
        assert_eq!(held.hold_reason, Some(HoldReason::MissingGeometry));
        assert_eq!(requested(&held), captured);
        for joint in [HeadJoint::Yaw, HeadJoint::Pitch] {
            assert!(
                (held.joint_control.reference[joint].velocity - old_reference[joint].velocity)
                    .abs()
                    <= f64::from(fixture.parameters.joint_control.maximum_acceleration[joint])
                        * 0.01
                        + 1e-6
            );
        }
        old_reference = held.joint_control.reference;
    }
    controller
        .observe(observation(-0.1, 0.1), at(1000))
        .unwrap();
    let resumed = controller
        .evaluate(&request, &fixture.context(), &fixture.parameters, at(1000))
        .unwrap();
    assert!(resumed.hold_reason.is_none());
    assert!(!resumed.joint_control.reseeded);
    assert_ne!(requested(&resumed), captured);
    // Many LowState updates without requests must not keep motion ownership alive.
    for millis in (1002..=1200).step_by(2) {
        controller
            .observe(observation(0.05, 0.3), at(millis))
            .unwrap();
    }
    let reactivated = controller
        .evaluate(&request, &missing_geometry, &fixture.parameters, at(1200))
        .unwrap();
    assert!(reactivated.joint_control.reseeded);
    assert_eq!(requested(&reactivated), observation(0.05, 0.3).positions);
    assert_eq!(
        commanded(&reactivated).positions,
        observation(0.05, 0.3).positions
    );
}

#[test]
fn invalid_required_inputs_fail_and_geometry_is_optional_for_joint_space_modes() {
    let fixture = Fixture::new();
    let mut controller = HeadController::default();
    let mut context = fixture.context();
    context.geometry = None;
    context.field_dimensions = None;
    let request = HeadMotion::ZeroAngles;
    assert!(
        controller
            .evaluate(&request, &context, &fixture.parameters, at(0))
            .is_err()
    );
    controller.observe(observation(0.1, 0.2), at(100)).unwrap();
    assert!(
        controller
            .evaluate(&request, &context, &fixture.parameters, at(99))
            .is_err()
    );
    assert!(
        controller
            .evaluate(
                &request,
                &context,
                &fixture.parameters,
                at(100) + fixture.parameters.maximum_observation_age + Duration::from_nanos(1)
            )
            .is_err()
    );
    assert!(
        controller
            .observe(observation(f32::NAN, 0.2), at(101))
            .is_err()
    );
    assert!(
        controller
            .evaluate(&request, &context, &fixture.parameters, at(101))
            .is_err()
    );
    controller.observe(observation(0.1, 0.2), at(102)).unwrap();
    context.joint_limits = None;
    assert!(
        controller
            .evaluate(&request, &context, &fixture.parameters, at(102))
            .is_err()
    );
    let mut invalid = fixture.joints.clone();
    invalid.position.head.yaw = [1.0, -1.0];
    context.joint_limits = Some(&invalid);
    assert!(
        controller
            .evaluate(&request, &context, &fixture.parameters, at(102))
            .is_err()
    );
    context.joint_limits = Some(&fixture.joints);
    assert!(
        controller
            .evaluate(&request, &context, &fixture.parameters, at(102))
            .is_ok()
    );
    let center = controller
        .evaluate(
            &HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            },
            &context,
            &fixture.parameters,
            at(102),
        )
        .unwrap();
    assert_eq!(center.hold_reason, Some(HoldReason::MissingFieldDimensions));
    let damping = controller
        .evaluate(&HeadMotion::Damping, &context, &fixture.parameters, at(102))
        .unwrap();
    assert!(damping.joint_control.progress.is_none());
    assert!(
        damping
            .joint_control
            .commands
            .into_iter()
            .all(|command| command.kp == 0.0 && command.velocity == 0.0 && command.torque == 0.0)
    );
}

#[test]
fn leaving_patterns_clears_feedback_and_damping_exit_reseeds_from_measurements() {
    let mut fixture = Fixture::new();
    fixture.parameters.look_around.dwell_duration = Duration::ZERO;
    let mut controller = HeadController::default();
    let left = fixture.parameters.look_around.left;
    let measurement = HeadObservation {
        positions: left,
        velocities: HeadJoints::fill(0.0),
    };
    controller.observe(measurement, at(0)).unwrap();
    controller
        .evaluate(
            &HeadMotion::LookAround,
            &fixture.context(),
            &fixture.parameters,
            at(0),
        )
        .unwrap();
    controller.observe(measurement, at(10)).unwrap();
    let center = controller
        .evaluate(
            &HeadMotion::LookAround,
            &fixture.context(),
            &fixture.parameters,
            at(10),
        )
        .unwrap();
    assert_eq!(requested(&center), fixture.parameters.look_around.center);
    controller
        .evaluate(
            &HeadMotion::ZeroAngles,
            &fixture.context(),
            &fixture.parameters,
            at(10),
        )
        .unwrap();
    let reentered = controller
        .evaluate(
            &HeadMotion::LookAround,
            &fixture.context(),
            &fixture.parameters,
            at(10),
        )
        .unwrap();
    assert_eq!(requested(&reentered), left);
    let search = controller
        .evaluate(
            &HeadMotion::SearchForLostBall,
            &fixture.context(),
            &fixture.parameters,
            at(10),
        )
        .unwrap();
    assert_eq!(
        requested(&search),
        fixture.parameters.search_for_lost_ball.center
    );
    let mut away = fixture.context();
    away.field_side = Some(GlobalFieldSide::Away);
    let right = controller
        .evaluate(&HeadMotion::LookAround, &away, &fixture.parameters, at(10))
        .unwrap();
    assert_eq!(requested(&right), fixture.parameters.look_around.right);
    controller
        .evaluate(&HeadMotion::Damping, &away, &fixture.parameters, at(10))
        .unwrap();
    let measurement = HeadObservation {
        positions: HeadJoints {
            yaw: 0.1,
            pitch: 0.2,
        },
        velocities: HeadJoints::fill(-0.2),
    };
    controller.observe(measurement, at(20)).unwrap();
    let resumed = controller
        .evaluate(&HeadMotion::ZeroAngles, &away, &fixture.parameters, at(20))
        .unwrap();
    assert!(resumed.joint_control.reseeded);
    assert_eq!(commanded(&resumed).positions, measurement.positions);
    assert_eq!(commanded(&resumed).velocities, measurement.velocities);
}
