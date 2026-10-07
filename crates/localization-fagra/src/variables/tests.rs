use std::{hint::black_box, time::Instant};

use fagra::{
    Tangent, Variable,
    testing::{TestScalar, TestVariable, Tolerance, proptest::prelude::*},
};
use linear_algebra::{Framed, Transform};
use nalgebra::{Isometry2, Isometry3, SVector, UnitComplex, UnitQuaternion, Vector3};

use super::{CameraIntrinsics, FieldAlignment, ImuBias, PoseControl, TrajectoryState};

impl<R: TestScalar> TestVariable for PoseControl<R> {
    type Dual = PoseControl<R::Dual>;

    fn states() -> impl Strategy<Value = Self> {
        TrajectoryState::<R>::states().prop_map(|state| Self { pose: state.pose })
    }

    fn increments() -> impl Strategy<Value = Tangent<Self>> {
        prop::array::uniform6(-1.5..1.5)
            .prop_map(|v| SVector::from_column_slice(&v.map(R::from_test_value)))
    }

    fn to_dual(&self) -> Self::Dual {
        Self::Dual {
            pose: Framed::wrap(Isometry3::from_parts(
                self.pose
                    .inner
                    .translation
                    .vector
                    .map(|x| x.dual(0.0))
                    .into(),
                UnitQuaternion::new_unchecked(nalgebra::Quaternion::from_vector(
                    self.pose.inner.rotation.coords.map(|x| x.dual(0.0)),
                )),
            )),
        }
    }

    fn equivalent(&self, other: &Self, tolerance: Tolerance) -> bool {
        close(
            self.pose.inner.to_homogeneous().as_slice(),
            other.pose.inner.to_homogeneous().as_slice(),
            tolerance,
        )
    }

    fn log_is_smooth(&self) -> bool {
        self.pose.inner.rotation.w.test_value().abs() > 1e-3
    }
}

fn close<R: TestScalar>(a: &[R], b: &[R], tolerance: Tolerance) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(&a, &b)| tolerance.close(a.test_value(), b.test_value()))
}

impl<R: TestScalar> TestVariable for TrajectoryState<R> {
    type Dual = TrajectoryState<R::Dual>;

    fn states() -> impl Strategy<Value = Self> {
        prop::array::uniform9(-2.0..2.0).prop_map(|values| {
            let v = values.map(R::from_test_value);
            Self {
                pose: Framed::wrap(Isometry3::from_parts(
                    Vector3::new(v[6], v[7], v[8]).into(),
                    UnitQuaternion::from_euler_angles(v[0], v[1], v[2]),
                )),
                velocity: Framed::wrap(Vector3::new(v[3], v[4], v[5])),
            }
        })
    }

    fn increments() -> impl Strategy<Value = Tangent<Self>> {
        prop::array::uniform9(-1.5..1.5)
            .prop_map(|v| SVector::from_column_slice(&v.map(R::from_test_value)))
    }

    fn to_dual(&self) -> Self::Dual {
        Self::Dual {
            pose: PoseControl { pose: self.pose }.to_dual().pose,
            velocity: Framed::wrap(self.velocity.inner.map(|x| x.dual(0.0))),
        }
    }

    fn equivalent(&self, other: &Self, tolerance: Tolerance) -> bool {
        close(
            self.pose.inner.to_homogeneous().as_slice(),
            other.pose.inner.to_homogeneous().as_slice(),
            tolerance,
        ) && close(
            self.velocity.inner.as_slice(),
            other.velocity.inner.as_slice(),
            tolerance,
        )
    }

    fn log_is_smooth(&self) -> bool {
        self.pose.inner.rotation.quaternion().w.test_value().abs() > 1e-3
    }
}

impl<R: TestScalar> TestVariable for FieldAlignment<R> {
    type Dual = FieldAlignment<R::Dual>;

    fn states() -> impl Strategy<Value = Self> {
        prop::array::uniform3(-3.0..3.0).prop_map(|v| {
            let v = v.map(R::from_test_value);
            Self {
                local_to_field: Transform::wrap(Isometry2::new(
                    nalgebra::Vector2::new(v[1], v[2]),
                    v[0],
                )),
            }
        })
    }

    fn increments() -> impl Strategy<Value = Tangent<Self>> {
        prop::array::uniform3(-3.0..3.0)
            .prop_map(|v| SVector::from_column_slice(&v.map(R::from_test_value)))
    }

    fn to_dual(&self) -> Self::Dual {
        let rotation = self.local_to_field.inner.rotation;
        Self::Dual {
            local_to_field: Transform::wrap(Isometry2::from_parts(
                self.local_to_field
                    .inner
                    .translation
                    .vector
                    .map(|x| x.dual(0.0))
                    .into(),
                UnitComplex::new_unchecked(nalgebra::Complex::new(
                    rotation.re.dual(0.0),
                    rotation.im.dual(0.0),
                )),
            )),
        }
    }

    fn equivalent(&self, other: &Self, tolerance: Tolerance) -> bool {
        close(
            self.local_to_field.inner.to_homogeneous().as_slice(),
            other.local_to_field.inner.to_homogeneous().as_slice(),
            tolerance,
        )
    }

    fn log_is_smooth(&self) -> bool {
        self.local_to_field
            .inner
            .rotation
            .angle()
            .test_value()
            .abs()
            < std::f64::consts::PI - 1e-3
    }
}

impl<R: TestScalar> TestVariable for CameraIntrinsics<R> {
    type Dual = CameraIntrinsics<R::Dual>;

    fn tolerance() -> Tolerance {
        let mut tolerance = Tolerance::for_scalar::<R>();
        // Round trips subtract pixel coordinates up to 1000: f32 cancellation
        // is governed by the stored calibration scale, not the small increment.
        tolerance.absolute = tolerance.absolute.max(4.0 * 1000.0 * R::EPSILON);
        tolerance
    }

    fn states() -> impl Strategy<Value = Self> {
        prop::array::uniform4(-1000.0..1000.0).prop_map(|v| {
            let v = v.map(R::from_test_value);
            Self {
                focal_lengths: Framed::wrap(nalgebra::Vector2::new(v[0], v[1])),
                optical_center: Framed::wrap(nalgebra::Point2::new(v[2], v[3])),
            }
        })
    }

    fn increments() -> impl Strategy<Value = Tangent<Self>> {
        prop::array::uniform4(-10.0..10.0)
            .prop_map(|v| SVector::from_column_slice(&v.map(R::from_test_value)))
    }

    fn to_dual(&self) -> Self::Dual {
        Self::Dual {
            focal_lengths: Framed::wrap(self.focal_lengths.inner.map(|x| x.dual(0.0))),
            optical_center: Framed::wrap(self.optical_center.inner.map(|x| x.dual(0.0))),
        }
    }

    fn equivalent(&self, other: &Self, tolerance: Tolerance) -> bool {
        close(
            self.focal_lengths.inner.as_slice(),
            other.focal_lengths.inner.as_slice(),
            tolerance,
        ) && close(
            self.optical_center.inner.coords.as_slice(),
            other.optical_center.inner.coords.as_slice(),
            tolerance,
        )
    }
}

impl<R: TestScalar> TestVariable for ImuBias<R> {
    type Dual = ImuBias<R::Dual>;
    fn states() -> impl Strategy<Value = Self> {
        Self::increments().prop_map(|v| Self::exp(&v))
    }
    fn increments() -> impl Strategy<Value = Tangent<Self>> {
        prop::array::uniform6(-0.2..0.2)
            .prop_map(|v| SVector::from_column_slice(&v.map(R::from_test_value)))
    }
    fn to_dual(&self) -> Self::Dual {
        Self::Dual {
            gyroscope: Framed::wrap(self.gyroscope.inner.map(|x| x.dual(0.0))),
            accelerometer: Framed::wrap(self.accelerometer.inner.map(|x| x.dual(0.0))),
        }
    }
    fn equivalent(&self, other: &Self, tolerance: Tolerance) -> bool {
        close(self.log().as_slice(), other.log().as_slice(), tolerance)
    }
}
fagra::variable_tests!(bias_f64, ImuBias<f64>);
fagra::variable_tests!(bias_f32, ImuBias<f32>);
fagra::variable_tests!(trajectory_f64, TrajectoryState<f64>);
fagra::variable_tests!(pose_control_f64, PoseControl<f64>);
fagra::variable_tests!(pose_control_f32, PoseControl<f32>);
fagra::variable_tests!(trajectory_f32, TrajectoryState<f32>);
fagra::variable_tests!(alignment_f64, FieldAlignment<f64>);
fagra::variable_tests!(alignment_f32, FieldAlignment<f32>);
fagra::variable_tests!(intrinsics_f64, CameraIntrinsics<f64>);
fagra::variable_tests!(intrinsics_f32, CameraIntrinsics<f32>);

/// Explicit edge cases supplement random sampling. Compare analytical Jacobians
/// against AD of value-level exp, and check inverse Jacobians and chart round trips.
fn check_increment<V: TestVariable>(coords: &[V::Scalar]) {
    let delta = V::tangent_from_slice(coords);
    let state = V::exp(&delta);
    let tolerance = Tolerance::for_scalar::<V::Scalar>();
    assert!(
        close(state.log().as_slice(), coords, tolerance),
        "exp/log at {coords:?}"
    );
    let j = V::right_jacobian(&delta);
    let inverse = V::right_jacobian_inverse(&delta);
    let base = state.to_dual();
    for column in 0..coords.len() {
        let seeded: Vec<_> = coords
            .iter()
            .enumerate()
            .map(|(i, &x)| x.dual(if i == column { 1.0 } else { 0.0 }))
            .collect();
        let perturbed = V::Dual::exp(&V::Dual::tangent_from_slice(&seeded));
        let local = base.local(&perturbed);
        for row in 0..coords.len() {
            let (_, derivative) = V::Scalar::parts(local[row]);
            assert!(
                tolerance.close(j[(row, column)].test_value(), derivative),
                "exp Jacobian ({row}, {column}) at {coords:?}"
            );
            let product: f64 = (0..coords.len())
                .map(|k| (inverse[(row, k)] * j[(k, column)]).test_value())
                .sum();
            assert!(
                tolerance.close(product, if row == column { 1.0 } else { 0.0 }),
                "inverse Jacobian ({row}, {column}) at {coords:?}"
            );
        }
    }
}

fn edges<R: TestScalar>() {
    for angle in [
        0.0,
        1e-10,
        1e-5,
        0.019999,
        0.020001,
        0.099999,
        0.100001,
        0.499999,
        0.500001,
        std::f64::consts::PI - 1e-4,
    ] {
        for sign in [-1.0, 1.0] {
            let angle = sign * angle;
            check_increment::<FieldAlignment<R>>(&[angle, 1.2, -2.3].map(R::from_test_value));
            for axis in 0..3 {
                let mut coords = [0.0, 0.0, 0.0, 1.2, -2.3, 0.7, -0.5, 2.7, 1.1];
                coords[axis] = angle;
                check_increment::<TrajectoryState<R>>(&coords.map(R::from_test_value));
            }
        }
    }
}

#[test]
fn branch_edges_f64() {
    edges::<f64>();
}

#[test]
fn branch_edges_f32() {
    edges::<f32>();
}

#[test]
fn quarter_turn_couples_velocity_and_position() {
    let angle = std::f64::consts::FRAC_PI_2;
    let scale = angle.recip();
    let delta = SVector::<f64, 9>::from_row_slice(&[0.0, 0.0, angle, 1.0, 0.0, 0.0, 0.0, 2.0, 0.0]);
    let state = TrajectoryState::exp(&delta);
    assert!((state.velocity.inner - Vector3::new(scale, scale, 0.0)).norm() < 1e-12);
    assert!(
        (state.pose.inner.translation.vector - Vector3::new(-2.0 * scale, 2.0 * scale, 0.0)).norm()
            < 1e-12
    );
    assert!((state.pose.inner.rotation * Vector3::x() - Vector3::y()).norm() < 1e-12);
    let planar = FieldAlignment::exp(&Vector3::new(angle, 1.0, 0.0));
    assert!(
        (planar.local_to_field.inner.translation.vector - nalgebra::Vector2::new(scale, scale))
            .norm()
            < 1e-12
    );

    let negative_quaternion = TrajectoryState {
        pose: Framed::wrap(Isometry3::from_parts(
            state.pose.inner.translation,
            UnitQuaternion::new_unchecked(-*state.pose.inner.rotation.quaternion()),
        )),
        velocity: state.velocity,
    };
    assert!((negative_quaternion.log() - state.log()).norm() < 1e-12);
}

#[test]
#[ignore = "release-mode operation timing; run with --release --ignored --nocapture"]
fn lie_operation_timings() {
    fn measure<V: Variable<Scalar = f64>>(name: &str, coords: &[f64]) {
        let delta = V::tangent_from_slice(coords);
        let state = V::exp(&delta);
        let start = Instant::now();
        for _ in 0..20_000 {
            black_box(V::exp(black_box(&delta)));
            black_box(black_box(&state).log());
            black_box(black_box(&state).compose(black_box(&state)));
            black_box(black_box(&state).inverse());
            black_box(black_box(&state).adjoint());
            black_box(V::right_jacobian(black_box(&delta)));
            black_box(V::right_jacobian_inverse(black_box(&delta)));
        }
        println!(
            "{name}: {:.0} ns / seven-operation bundle",
            start.elapsed().as_nanos() as f64 / 20_000.0
        );
    }
    measure::<TrajectoryState>("SE₂(3)", &[0.2, -0.1, 0.3, 1.0, 2.0, 3.0, -2.0, 0.5, 1.5]);
    measure::<FieldAlignment>("SE(2)", &[0.4, 1.0, -2.0]);
    measure::<CameraIntrinsics>("R⁴", &[500.0, 500.0, 320.0, 240.0]);
}
