use fagra::{
    Tangent, Variable,
    testing::{
        TestScalar, TestVariable, Tolerance,
        proptest::{prelude::*, test_runner::TestRunner},
    },
};
use linear_algebra::{Framed, Pose3};
use nalgebra::{Isometry3, UnitQuaternion, Vector3};

use super::PoseSpline;
use crate::variables::PoseControl;

fn control(position: Vector3<f64>, rotation: UnitQuaternion<f64>) -> PoseControl {
    PoseControl {
        pose: Framed::wrap(Isometry3::from_parts(position.into(), rotation)),
    }
}

fn assert_close<R: TestScalar>(actual: R, expected: R, tolerance: Tolerance, context: &str) {
    assert!(
        tolerance.close(actual.test_value(), expected.test_value()),
        "{context}: {} != {}",
        actual.test_value(),
        expected.test_value()
    );
}

#[test]
fn stationary_and_known_polynomial_motion() {
    let p = Vector3::new(1.0, -2.0, 3.0);
    let orientation = UnitQuaternion::from_euler_angles(0.2, -0.4, 0.7);
    let stationary = control(p, orientation);
    let spline = PoseSpline::new([&stationary; 4], 0.2).unwrap();
    for tau in [0.0, 0.3, 1.0] {
        assert!(
            (spline.pose(tau).unwrap().inner.to_homogeneous()
                - stationary.pose.inner.to_homogeneous())
            .norm()
                < 1e-12
        );
        assert!(spline.velocity(tau).unwrap().inner.norm() < 1e-12);
        let k = spline.kinematics(tau).unwrap();
        assert!(k.inner.norm() < 1e-12);
    }

    let duration = 0.7;
    let velocity = Vector3::new(0.4, -0.8, 1.2);
    let omega = Vector3::new(0.1, -0.2, 0.3);
    for acceleration in [Vector3::zeros(), Vector3::new(-0.5, 0.3, 0.9)] {
        let controls: [_; 4] = std::array::from_fn(|i| {
            let t = (i as f64 - 1.0) * duration;
            // Cardinal cubic controls for a quadratic:
            // subtract the basis's dt²/3 second-moment offset.
            control(
                p + velocity * t + acceleration * (0.5 * (t * t - duration * duration / 3.0)),
                orientation * UnitQuaternion::from_scaled_axis(omega * t),
            )
        });
        let spline = PoseSpline::new(controls.each_ref(), duration).unwrap();
        for tau in [0.0, 0.2, 0.7, 1.0] {
            let t = tau * duration;
            let sample = spline.state(tau).unwrap();
            assert!(
                (sample.pose.inner.translation.vector
                    - (p + velocity * t + acceleration * (0.5 * t * t)))
                    .norm()
                    < 1e-12
            );
            let expected_rotation = orientation * UnitQuaternion::from_scaled_axis(omega * t);
            assert!(
                (sample.pose.inner.rotation.to_rotation_matrix().matrix()
                    - expected_rotation.to_rotation_matrix().matrix())
                .norm()
                    < 1e-12
            );
            assert!((sample.velocity.inner - velocity - acceleration * t).norm() < 1e-12);
            let k = spline.kinematics(tau).unwrap();
            assert!((k.inner - omega).norm() < 1e-12);
        }
    }
}

fn sample_controls<R: TestScalar>() -> [PoseControl<R>; 5] {
    std::array::from_fn(|i| {
        let c = R::from_test_value;
        let x = i as f64;
        PoseControl {
            pose: Pose3::wrap(Isometry3::from_parts(
                Vector3::new(c(x * x * 0.2), c(x * -0.1), c(x.sin())).into(),
                UnitQuaternion::from_euler_angles(c(0.2 * x), c(0.15 * x * x), c(-0.3 * x)),
            )),
        }
    })
}

#[test]
fn controls_are_not_interpolation_samples() {
    let controls = sample_controls::<f64>();
    let spline = PoseSpline::new(
        [&controls[0], &controls[1], &controls[2], &controls[3]],
        0.2,
    )
    .unwrap();
    let p = controls.each_ref().map(|c| c.pose.inner.translation.vector);
    let at_start = spline.pose(0.0).unwrap().inner.translation.vector;
    let at_end = spline.pose(1.0).unwrap().inner.translation.vector;
    assert!((at_start - (p[0] + p[1] * 4.0 + p[2]) / 6.0).norm() < 1e-12);
    assert!((at_end - (p[1] + p[2] * 4.0 + p[3]) / 6.0).norm() < 1e-12);
    assert!((at_start - p[1]).norm() > 0.01);
}

fn continuity<R: TestScalar>() {
    let controls = sample_controls::<R>();
    let c = R::from_test_value;
    let dt = c(0.2);
    let left =
        PoseSpline::new([&controls[0], &controls[1], &controls[2], &controls[3]], dt).unwrap();
    let right =
        PoseSpline::new([&controls[1], &controls[2], &controls[3], &controls[4]], dt).unwrap();
    let tolerance = Tolerance {
        absolute: (512.0 * R::EPSILON / 0.04).max(1e-10),
        relative: (512.0 * R::EPSILON).max(1e-10),
    };
    let a = left.state(R::one()).unwrap();
    let b = right.state(R::zero()).unwrap();
    assert!(a.equivalent(&b, tolerance));
    let ka = left.kinematics(R::one()).unwrap();
    let kb = right.kinematics(R::zero()).unwrap();
    for i in 0..3 {
        assert_close(ka.inner[i], kb.inner[i], tolerance, "C1 rotation");
    }
    // Time AD verifies C2 continuity without a production acceleration API.
    let dual = controls.each_ref().map(TestVariable::to_dual);
    let dl = PoseSpline::new([&dual[0], &dual[1], &dual[2], &dual[3]], dt.dual(0.0)).unwrap();
    let dr = PoseSpline::new([&dual[1], &dual[2], &dual[3], &dual[4]], dt.dual(0.0)).unwrap();
    let a = dl.kinematics(R::one().dual(1.0 / dt.test_value())).unwrap();
    let b = dr
        .kinematics(R::zero().dual(1.0 / dt.test_value()))
        .unwrap();
    let va = dl.velocity(R::one().dual(1.0 / dt.test_value())).unwrap();
    let vb = dr.velocity(R::zero().dual(1.0 / dt.test_value())).unwrap();
    for i in 0..3 {
        let (_, da) = R::parts(a.inner[i]);
        let (_, db) = R::parts(b.inner[i]);
        assert!(tolerance.close(da, db), "C2 rotation: {da} != {db}");
        let (_, da) = R::parts(va.inner[i]);
        let (_, db) = R::parts(vb.inner[i]);
        assert!(tolerance.close(da, db), "C2 position: {da} != {db}");
    }
}

#[test]
fn continuity_f64() {
    continuity::<f64>();
}
#[test]
fn continuity_f32() {
    continuity::<f32>();
}

#[test]
fn duration_scaling_and_quaternion_sign() {
    let [a, b, c, d, _] = sample_controls::<f64>();
    let controls = [a, b, c, d];
    let flipped = controls.each_ref().map(|control| {
        let mut flipped = control.clone();
        flipped.pose.inner.rotation =
            UnitQuaternion::new_unchecked(-*control.pose.inner.rotation.quaternion());
        flipped
    });
    let fast = PoseSpline::new(controls.each_ref(), 0.4).unwrap();
    let slow = PoseSpline::new(flipped.each_ref(), 0.8).unwrap();
    for tau in [0.0, 0.3, 1.0] {
        assert!(
            (fast.pose(tau).unwrap().inner.to_homogeneous()
                - slow.pose(tau).unwrap().inner.to_homogeneous())
            .norm()
                < 1e-12
        );
        assert!(
            (fast.velocity(tau).unwrap().inner - slow.velocity(tau).unwrap().inner * 2.0).norm()
                < 1e-12
        );
        let f = fast.kinematics(tau).unwrap();
        let s = slow.kinematics(tau).unwrap();
        assert!((f.inner - s.inner * 2.0).norm() < 1e-12);
    }
}

fn check_derivatives<R: TestScalar>(controls: &[PoseControl<R>; 4], duration: R, tau: R) {
    let spline = PoseSpline::new(controls.each_ref(), duration).unwrap();
    let linearized = spline.linearize().unwrap();
    let pose = linearized.pose(tau).unwrap();
    let velocity = linearized.velocity(tau).unwrap();
    let state = linearized.state(tau).unwrap();
    let kinematics = linearized.kinematics(tau).unwrap();
    let (shared_pose, shared_kinematics) = linearized.pose_and_kinematics(tau).unwrap();
    let (value_pose, value_kinematics) = spline.pose_and_kinematics(tau).unwrap();
    let base_pose = PoseControl { pose: pose.pose }.to_dual();
    let base_state = state.state.to_dual();
    let tolerance = Tolerance::for_scalar::<R>();
    assert!(
        PoseControl {
            pose: shared_pose.pose
        }
        .equivalent(&PoseControl { pose: value_pose }, tolerance)
    );
    for i in 0..4 {
        for (shared, separate) in shared_pose.jacobians[i]
            .iter()
            .zip(pose.jacobians[i].iter())
        {
            assert_close(*shared, *separate, tolerance, "shared pose Jacobian");
        }
        for (shared, separate) in shared_kinematics.angular_velocity_jacobians[i]
            .iter()
            .zip(kinematics.angular_velocity_jacobians[i].iter())
        {
            assert_close(*shared, *separate, tolerance, "shared gyro Jacobian");
        }
    }
    for i in 0..3 {
        assert_close(
            value_kinematics.inner[i],
            shared_kinematics.angular_velocity.inner[i],
            tolerance,
            "shared gyro value",
        );
    }
    // Keep pose checks tight: only derivative outputs get duration-scaled absolute
    // tolerances. Acceleration roundoff must not loosen rotational Jacobian checks.
    let velocity_tolerance = Tolerance {
        absolute: tolerance.absolute / duration.test_value().min(1.0),
        ..tolerance
    };
    let acceleration_tolerance = Tolerance {
        absolute: tolerance.absolute / duration.test_value().min(1.0).powi(2),
        ..tolerance
    };
    assert!(
        state
            .state
            .equivalent(&spline.state(tau).unwrap(), tolerance)
    );
    let value_kinematics = spline.kinematics(tau).unwrap();
    for row in 0..3 {
        assert_close(
            kinematics.angular_velocity.inner[row],
            value_kinematics.inner[row],
            velocity_tolerance,
            "value/linearized omega",
        );
    }
    for control in 0..4 {
        for column in 0..6 {
            let mut dual_controls = controls.each_ref().map(TestVariable::to_dual);
            let mut delta = Tangent::<PoseControl<R::Dual>>::zeros();
            delta[column] = R::zero().dual(1.0);
            dual_controls[control] = dual_controls[control].retract(&delta);
            let dual = PoseSpline::new(dual_controls.each_ref(), duration.dual(0.0)).unwrap();
            let dual_state = dual.state(tau.dual(0.0)).unwrap();
            let local_pose = base_pose.local(&PoseControl {
                pose: dual_state.pose,
            });
            let local_state = base_state.local(&dual_state);
            let dual_kinematics = dual.kinematics(tau.dual(0.0)).unwrap();
            for row in 0..9 {
                let (_, derivative) = R::parts(local_state[row]);
                let actual = state.jacobians[control][(row, column)].test_value();
                let tolerance = if (3..6).contains(&row) {
                    velocity_tolerance
                } else {
                    tolerance
                };
                assert!(
                    tolerance.close(actual, derivative),
                    "state control {control} ({row}, {column}): {actual} != {derivative}"
                );
            }
            for row in 0..6 {
                let (_, derivative) = R::parts(local_pose[row]);
                let actual = pose.jacobians[control][(row, column)].test_value();
                assert!(
                    tolerance.close(actual, derivative),
                    "pose control {control} ({row}, {column}): {actual} != {derivative}"
                );
            }
            for row in 0..3 {
                for (name, value, jacobian, tolerance) in [
                    (
                        "velocity",
                        dual_state.velocity.inner[row],
                        &velocity.jacobians,
                        velocity_tolerance,
                    ),
                    (
                        "angular velocity",
                        dual_kinematics.inner[row],
                        &kinematics.angular_velocity_jacobians,
                        velocity_tolerance,
                    ),
                ] {
                    let (_, derivative) = R::parts(value);
                    let actual = jacobian[control][(row, column)].test_value();
                    assert!(
                        tolerance.close(actual, derivative),
                        "{name} control {control} ({row}, {column}): {actual} != {derivative}"
                    );
                }
            }
        }
    }
    // Independent time AD confirms velocity = p_dot, acceleration = v_dot and
    // body angular velocity = vee(R^T R_dot), including noncommuting rotations.
    let dual_controls = controls.each_ref().map(TestVariable::to_dual);
    let dual = PoseSpline::new(dual_controls.each_ref(), duration.dual(0.0)).unwrap();
    let time = tau.dual(duration.test_value().recip());
    let dual_state = dual.state(time).unwrap();
    let p = controls.each_ref().map(|c| c.pose.inner.translation.vector);
    let two = R::from_test_value(2.0);
    let acceleration = ((p[2] - p[1] * two + p[0]) * (R::one() - tau)
        + (p[3] - p[2] * two + p[1]) * tau)
        / (duration * duration);
    let local_pose = base_pose.local(&PoseControl {
        pose: dual_state.pose,
    });
    for row in 0..3 {
        for (name, value, expected, tolerance) in [
            (
                "p_dot",
                dual_state.pose.inner.translation.vector[row],
                velocity.velocity.inner[row],
                velocity_tolerance,
            ),
            (
                "v_dot",
                dual_state.velocity.inner[row],
                acceleration[row],
                acceleration_tolerance,
            ),
            (
                "R_dot",
                local_pose[row],
                kinematics.angular_velocity.inner[row],
                velocity_tolerance,
            ),
        ] {
            let (_, derivative) = R::parts(value);
            assert!(
                tolerance.close(derivative, expected.test_value()),
                "{name}: {derivative} != {}",
                expected.test_value()
            );
        }
    }
}

fn derivatives<R: TestScalar>() {
    let c = R::from_test_value;
    for angle in [
        0.0,
        1e-10,
        0.099999,
        0.100001,
        0.499999,
        0.500001,
        std::f64::consts::PI - 1e-4,
    ] {
        let controls: [_; 4] = std::array::from_fn(|i| {
            let mut control = sample_controls::<R>()[i].clone();
            control.pose.inner.rotation =
                UnitQuaternion::from_axis_angle(&Vector3::z_axis(), c(i as f64 * angle));
            control
        });
        for tau in [0.0, 0.3, 1.0] {
            check_derivatives(&controls, c(0.2), c(tau));
        }
    }
    let strategy = (
        prop::array::uniform4(PoseControl::<R>::states()),
        0.1..2.0,
        0.0..1.0,
    )
        .prop_filter(
            "consecutive rotation logs away from seam",
            |(controls, _, _)| {
                controls.windows(2).all(|pair| {
                    (pair[0].pose.inner.rotation.inverse() * pair[1].pose.inner.rotation)
                        .w
                        .test_value()
                        .abs()
                        > 1e-3
                })
            },
        );
    TestRunner::default()
        .run(&strategy, |(controls, dt, tau)| {
            check_derivatives(&controls, c(dt), c(tau));
            Ok(())
        })
        .unwrap();
}

#[test]
fn control_jacobians_and_time_derivatives_f64() {
    derivatives::<f64>();
}
#[test]
fn control_jacobians_and_time_derivatives_f32() {
    derivatives::<f32>();
}

#[test]
fn jacobians_match_independent_central_differences() {
    // Perturb with nalgebra's quaternion/translation operations, not our Lie exp;
    // compare output derivatives without our Lie log. This complements dual AD.
    let [a, b, c, d, _] = sample_controls::<f64>();
    let controls = [a, b, c, d];
    let h = 1e-6;
    let tolerance = Tolerance {
        absolute: 2e-7,
        relative: 2e-7,
    };
    for duration in [0.02, 0.2, 1.0] {
        let spline = PoseSpline::new(controls.each_ref(), duration).unwrap();
        let linearized = spline.linearize().unwrap();
        for tau in [0.0, 0.37, 1.0] {
            let pose = linearized.pose(tau).unwrap();
            let velocity = linearized.velocity(tau).unwrap();
            let kinematics = linearized.kinematics(tau).unwrap();
            let output_inverse = pose.pose.inner.rotation.inverse();
            for index in 0..4 {
                for column in 0..6 {
                    let evaluate = |step| {
                        let mut perturbed = controls.clone();
                        let mut increment = Vector3::zeros();
                        increment[column % 3] = step;
                        if column < 3 {
                            perturbed[index].pose.inner.rotation *=
                                UnitQuaternion::from_scaled_axis(increment);
                        } else {
                            perturbed[index].pose.inner.translation.vector +=
                                controls[index].pose.inner.rotation * increment;
                        }
                        let spline = PoseSpline::new(perturbed.each_ref(), duration).unwrap();
                        (
                            spline.pose(tau).unwrap(),
                            spline.velocity(tau).unwrap(),
                            spline.kinematics(tau).unwrap(),
                        )
                    };
                    let (plus_pose, plus_velocity, plus_kinematics) = evaluate(h);
                    let (minus_pose, minus_velocity, minus_kinematics) = evaluate(-h);
                    let angle = ((output_inverse * plus_pose.inner.rotation).scaled_axis()
                        - (output_inverse * minus_pose.inner.rotation).scaled_axis())
                        / (2.0 * h);
                    let translation = output_inverse
                        * (plus_pose.inner.translation.vector
                            - minus_pose.inner.translation.vector)
                        / (2.0 * h);
                    let v = (plus_velocity.inner - minus_velocity.inner) / (2.0 * h);
                    let omega = (plus_kinematics.inner - minus_kinematics.inner) / (2.0 * h);
                    for row in 0..3 {
                        assert_close(
                            pose.jacobians[index][(row, column)],
                            angle[row],
                            tolerance,
                            "finite-difference rotation",
                        );
                        assert_close(
                            pose.jacobians[index][(row + 3, column)],
                            translation[row],
                            tolerance,
                            "finite-difference translation",
                        );
                        let velocity_tolerance = Tolerance {
                            absolute: tolerance.absolute / duration,
                            ..tolerance
                        };
                        assert_close(
                            velocity.jacobians[index][(row, column)],
                            v[row],
                            velocity_tolerance,
                            "finite-difference velocity",
                        );
                        assert_close(
                            kinematics.angular_velocity_jacobians[index][(row, column)],
                            omega[row],
                            velocity_tolerance,
                            "finite-difference omega",
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn rejects_invalid_domains_and_branch_seams() {
    let control = PoseControl::<f64>::identity();
    for dt in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-300, 1e300] {
        assert!(PoseSpline::new([&control; 4], dt).is_err());
    }
    let spline = PoseSpline::new([&control; 4], 0.2).unwrap();
    let linearized = spline.linearize().unwrap();
    for tau in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(spline.pose(tau).is_err());
        assert!(spline.velocity(tau).is_err());
        assert!(spline.state(tau).is_err());
        assert!(spline.kinematics(tau).is_err());
        assert!(spline.pose_and_kinematics(tau).is_err());
        assert!(linearized.pose(tau).is_err());
        assert!(linearized.velocity(tau).is_err());
        assert!(linearized.state(tau).is_err());
        assert!(linearized.kinematics(tau).is_err());
        assert!(linearized.pose_and_kinematics(tau).is_err());
    }
    for index in 0..4 {
        let mut controls: [_; 4] = std::array::from_fn(|_| control.clone());
        controls[index].pose.inner.translation.vector.x = f64::NAN;
        assert!(PoseSpline::new(controls.each_ref(), 0.2).is_err());
        controls[index] = control.clone();
        controls[index].pose.inner.rotation =
            UnitQuaternion::new_unchecked(nalgebra::Quaternion::new(f64::NAN, 0.0, 0.0, 0.0));
        assert!(PoseSpline::new(controls.each_ref(), 0.2).is_err());
    }
    for index in 1..4 {
        let mut controls: [_; 4] = std::array::from_fn(|_| control.clone());
        for c in &mut controls[index..] {
            c.pose.inner.rotation =
                UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f64::consts::PI);
        }
        let seam = PoseSpline::new(controls.each_ref(), 0.2).unwrap();
        assert!(seam.pose(0.5).is_ok());
        assert!(seam.linearize().is_err());
    }
}
