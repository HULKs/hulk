//! Uniform split cubic B-spline: cumulative SO(3) rotation and Euclidean position.
//!
//! Each segment uses four consecutive pose controls, with uniform spacing `dt`.
//! If controls are labeled at `t[i-1], t[i], t[i+1], t[i+2]`, this segment covers
//! `[t[i], t[i+1]]` and `tau = (t - t[i]) / dt`. The next segment shifts the control
//! array by one. Controls generally do not lie on the trajectory. At simple interior
//! knots the curve is C² (away from rotation-log branch seams).
//!
//! Prepare from current estimates on every evaluation pass, then reuse within a
//! factor batch. Velocity and acceleration are derivatives of position: no extra
//! velocity controls or kinematic consistency constraints are needed. The caller
//! supplies boundary controls and retains the full support when marginalizing;
//! this segment evaluator does not clamp, extrapolate, or manage timestamps.
//!
//! Rotation uses the cumulative construction and derivative recurrence from
//! [Sommer et al.](https://arxiv.org/abs/1911.08860), adapted to right control
//! increments. All geometry and Jacobians use fixed-size, allocation-free storage.

use coordinate_systems::{Local, Robot};
use fagra::EvaluationError;
use linear_algebra::{Pose3, Vector3};
use nalgebra::{Isometry3, Matrix3, RealField, SMatrix, UnitQuaternion, Vector3 as Vector};

use crate::{
    finite,
    variables::{
        PoseControl, TrajectoryState,
        rotation::{self, scalar},
    },
};

#[cfg(test)]
mod tests;

/// One cubic segment, parameterized by normalized time in [0, 1].
#[derive(Debug)]
pub struct PoseSpline<R: RealField + Copy = f64> {
    rotations: [UnitQuaternion<R>; 4],
    rotation_deltas: [Vector<R>; 3],
    smooth_rotation: bool,
    /// Local position polynomial, in ascending powers of tau.
    position: [Vector<R>; 4],
    inverse_duration: R,
}

/// Pose and four control Jacobians in control-array order.
#[derive(Debug)]
pub struct LinearizedPose<R: RealField + Copy = f64> {
    pub pose: Pose3<Local, R>,
    /// Both output and control coordinates are right-local `[rotation, translation]`.
    pub jacobians: [SMatrix<R, 6, 6>; 4],
}

/// Local-frame velocity and derivatives with respect to the four controls.
#[derive(Debug)]
pub struct LinearizedVelocity<R: RealField + Copy = f64> {
    pub velocity: Vector3<Local, R>,
    /// Vector-component derivatives; control columns are `[rotation, translation]`.
    pub jacobians: [SMatrix<R, 3, 6>; 4],
}

/// Evaluated navigation state, useful for initial anchors and motion priors.
#[derive(Debug)]
pub struct LinearizedState<R: RealField + Copy = f64> {
    pub state: TrajectoryState<R>,
    /// Output SE₂(3) right-local `[rotation, velocity, position]` versus each
    /// control's right-local `[rotation, translation]`.
    pub jacobians: [SMatrix<R, 9, 6>; 4],
}

impl<R: RealField + Copy> LinearizedState<R> {
    pub(crate) fn from_pose_and_velocity(
        pose: LinearizedPose<R>,
        velocity: LinearizedVelocity<R>,
    ) -> Result<Self, EvaluationError> {
        let inverse = pose
            .pose
            .inner
            .rotation
            .to_rotation_matrix()
            .inverse()
            .into_inner();
        let jacobians = std::array::from_fn(|i| {
            let mut j = SMatrix::<R, 9, 6>::zeros();
            j.fixed_rows_mut::<3>(0)
                .copy_from(&pose.jacobians[i].fixed_rows::<3>(0));
            j.fixed_rows_mut::<3>(3)
                .copy_from(&(inverse * velocity.jacobians[i]));
            j.fixed_rows_mut::<3>(6)
                .copy_from(&pose.jacobians[i].fixed_rows::<3>(3));
            j
        });
        finite(jacobians.iter().flat_map(|j| j.iter()))?;
        Ok(Self {
            state: TrajectoryState {
                pose: pose.pose,
                velocity: velocity.velocity,
            },
            jacobians,
        })
    }
}

/// Kinematics and vector-component derivatives with respect to all four controls.
#[derive(Debug)]
pub struct LinearizedKinematics<R: RealField + Copy = f64> {
    /// Time-varying body angular velocity in Robot axes, rad/s.
    pub angular_velocity: Vector3<Robot, R>,
    pub angular_velocity_jacobians: [SMatrix<R, 3, 6>; 4],
}

/// Evaluated pose in Local and body angular velocity in Robot axes.
pub type PoseKinematics<R = f64> = (Pose3<Local, R>, Vector3<Robot, R>);

/// Control-dependent derivative preparation, shared across observations in a batch.
#[derive(Debug)]
pub struct LinearizedPoseSpline<'a, R: RealField + Copy = f64> {
    spline: &'a PoseSpline<R>,
    control_rotations: [Matrix3<R>; 4],
    /// Derivatives of each consecutive rotation log with respect to its two controls.
    delta_jacobians: [[Matrix3<R>; 2]; 3],
}

impl<R: RealField + Copy> PoseSpline<R> {
    /// Prepare four consecutive controls and a positive knot spacing in seconds.
    /// Quaternions must satisfy nalgebra's unit invariant. Rejects nonfinite inputs,
    /// polynomial overflow, and unrepresentable inverse-duration scaling.
    pub fn new(controls: [&PoseControl<R>; 4], duration: R) -> Result<Self, EvaluationError> {
        if !duration.is_finite() || duration <= R::zero() {
            return Err(EvaluationError::InvalidEvaluation);
        }
        for control in controls {
            finite(
                control
                    .pose
                    .inner
                    .rotation
                    .coords
                    .iter()
                    .chain(control.pose.inner.translation.vector.iter()),
            )?;
        }
        let rotations = controls.map(|control| control.pose.inner.rotation);
        let relative: [_; 3] = std::array::from_fn(|i| rotations[i].inverse() * rotations[i + 1]);
        let rotation_deltas = relative.map(|r| rotation::log(&r));
        let smooth_rotation = relative.iter().all(|r| r.w.abs() > scalar(1e-6));
        let p = controls.map(|control| control.pose.inner.translation.vector);
        let d0 = p[1] - p[0];
        let d1 = p[2] - p[1];
        let d2 = p[3] - p[2];
        let c = scalar::<R>;
        // Difference form avoids summing large absolute positions with opposing signs.
        let position = [
            p[1] + (d1 - d0) * c(1.0 / 6.0),
            (d0 + d1) * c(0.5),
            (d1 - d0) * c(0.5),
            (d2 - d1 * c(2.0) + d0) * c(1.0 / 6.0),
        ];
        let inverse_duration = duration.recip();
        let inverse_duration_squared = inverse_duration * inverse_duration;
        if !inverse_duration.is_finite()
            || !inverse_duration_squared.is_finite()
            || inverse_duration_squared <= R::zero()
        {
            return Err(EvaluationError::InvalidEvaluation);
        }
        finite(
            position
                .iter()
                .chain(rotation_deltas.iter())
                .flat_map(|v| v.iter()),
        )?;
        Ok(Self {
            rotations,
            rotation_deltas,
            smooth_rotation,
            position,
            inverse_duration,
        })
    }

    /// Robot pose in Local. At a relative rotation of π, values follow the chosen
    /// log branch; derivative preparation rejects that ambiguous branch seam.
    pub fn pose(&self, tau: R) -> Result<Pose3<Local, R>, EvaluationError> {
        validate_tau(tau)?;
        let basis = Basis::new(tau);
        self.pose_from_increments(tau, &self.increments(&basis))
    }

    /// Local translational velocity in m/s, exactly the spline position derivative.
    pub fn velocity(&self, tau: R) -> Result<Vector3<Local, R>, EvaluationError> {
        validate_tau(tau)?;
        let [_, a, b, c] = self.position;
        let velocity = ((c * (scalar::<R>(3.0) * tau) + b * scalar::<R>(2.0)) * tau + a)
            * self.inverse_duration;
        finite(velocity.iter())?;
        Ok(Vector3::wrap(velocity))
    }

    /// Physical navigation state with `velocity = d(position)/dt`.
    pub fn state(&self, tau: R) -> Result<TrajectoryState<R>, EvaluationError> {
        Ok(TrajectoryState {
            pose: self.pose(tau)?,
            velocity: self.velocity(tau)?,
        })
    }

    /// Body angular velocity in the evaluated Robot axes, rad/s.
    pub fn kinematics(&self, tau: R) -> Result<Vector3<Robot, R>, EvaluationError> {
        validate_tau(tau)?;
        let basis = Basis::new(tau);
        let increments = self.increments(&basis);
        self.kinematics_from_increments(&basis, &increments)
    }

    /// Shared value evaluation for gyro/tilt and mounting offsets: compute the three
    /// rotational exponentials once for both pose and kinematics.
    pub fn pose_and_kinematics(&self, tau: R) -> Result<PoseKinematics<R>, EvaluationError> {
        validate_tau(tau)?;
        let basis = Basis::new(tau);
        let increments = self.increments(&basis);
        Ok((
            self.pose_from_increments(tau, &increments)?,
            self.kinematics_from_increments(&basis, &increments)?,
        ))
    }

    fn kinematics_from_increments(
        &self,
        basis: &Basis<R>,
        increments: &[UnitQuaternion<R>; 3],
    ) -> Result<Vector3<Robot, R>, EvaluationError> {
        let mut omega = Vector::zeros();
        for (i, increment) in increments.iter().enumerate() {
            omega = increment.inverse() * omega
                + self.rotation_deltas[i] * (basis.cumulative_first[i] * self.inverse_duration);
        }
        finite(omega.iter())?;
        Ok(Vector3::wrap(omega))
    }

    /// Prepare analytical right-control Jacobians. Rejects any consecutive rotation
    /// pair with `abs(relative quaternion w) <= 1e-6` (about 2e-6 radians from π).
    pub fn linearize(&self) -> Result<LinearizedPoseSpline<'_, R>, EvaluationError> {
        self.check_smooth_rotation()?;
        let control_rotations = self.rotations.map(|r| r.to_rotation_matrix().into_inner());
        let delta_jacobians = std::array::from_fn(|i| {
            let inverse = rotation::right_jacobian_inverse(self.rotation_deltas[i]);
            [
                -inverse * control_rotations[i + 1].transpose() * control_rotations[i],
                inverse,
            ]
        });
        finite(delta_jacobians.iter().flatten().flat_map(|j| j.iter()))?;
        Ok(LinearizedPoseSpline {
            spline: self,
            control_rotations,
            delta_jacobians,
        })
    }

    /// Factor cost and Jacobians must share a domain: LM must not accept a
    /// lower-cost trial at a rotation-log seam that it cannot then linearize.
    pub(crate) fn check_smooth_rotation(&self) -> Result<(), EvaluationError> {
        if !self.smooth_rotation {
            return Err(EvaluationError::InvalidEvaluation);
        }
        Ok(())
    }

    fn increments(&self, basis: &Basis<R>) -> [UnitQuaternion<R>; 3] {
        std::array::from_fn(|i| rotation::exp(self.rotation_deltas[i] * basis.cumulative[i]))
    }

    fn pose_from_increments(
        &self,
        tau: R,
        increments: &[UnitQuaternion<R>; 3],
    ) -> Result<Pose3<Local, R>, EvaluationError> {
        let [p, a, b, c] = self.position;
        let position = ((c * tau + b) * tau + a) * tau + p;
        let rotation = self.rotations[0] * increments[0] * increments[1] * increments[2];
        finite(position.iter().chain(rotation.coords.iter()))?;
        Ok(Pose3::wrap(Isometry3::from_parts(
            position.into(),
            rotation,
        )))
    }
}

impl<R: RealField + Copy> LinearizedPoseSpline<'_, R> {
    pub fn pose(&self, tau: R) -> Result<LinearizedPose<R>, EvaluationError> {
        validate_tau(tau)?;
        let basis = Basis::new(tau);
        let increments = self.spline.increments(&basis);
        self.pose_from_increments(tau, &basis, &increments, &self.increment_jacobians(&basis))
    }

    fn increment_jacobians(&self, basis: &Basis<R>) -> [Matrix3<R>; 3] {
        std::array::from_fn(|i| {
            rotation::right_jacobian(self.spline.rotation_deltas[i] * basis.cumulative[i])
                * basis.cumulative[i]
        })
    }

    fn pose_from_increments(
        &self,
        tau: R,
        basis: &Basis<R>,
        increments: &[UnitQuaternion<R>; 3],
        increment_jacobians: &[Matrix3<R>; 3],
    ) -> Result<LinearizedPose<R>, EvaluationError> {
        let pose = self.spline.pose_from_increments(tau, increments)?;
        let output_inverse = pose
            .inner
            .rotation
            .to_rotation_matrix()
            .inverse()
            .into_inner();
        let mut angular = [Matrix3::<R>::zeros(); 4];
        let mut suffix_inverse = Matrix3::<R>::identity();
        // Propagate each increment's right-local perturbation through its suffix.
        for i in (0..3).rev() {
            let j = suffix_inverse * increment_jacobians[i];
            angular[i] += j * self.delta_jacobians[i][0];
            angular[i + 1] += j * self.delta_jacobians[i][1];
            suffix_inverse *= increments[i].to_rotation_matrix().inverse().into_inner();
        }
        angular[0] += suffix_inverse;
        let jacobians = std::array::from_fn(|i| {
            let mut j = SMatrix::<R, 6, 6>::zeros();
            j.fixed_view_mut::<3, 3>(0, 0).copy_from(&angular[i]);
            // A right SE(3) translation increment is expressed in control axes.
            j.fixed_view_mut::<3, 3>(3, 3)
                .copy_from(&(output_inverse * self.control_rotations[i] * basis.value[i]));
            j
        });
        finite(jacobians.iter().flat_map(|j| j.iter()))?;
        Ok(LinearizedPose { pose, jacobians })
    }

    pub fn velocity(&self, tau: R) -> Result<LinearizedVelocity<R>, EvaluationError> {
        let velocity = self.spline.velocity(tau)?;
        let basis = Basis::new(tau);
        let jacobians =
            self.translation_jacobians(basis.first.map(|x| x * self.spline.inverse_duration))?;
        Ok(LinearizedVelocity {
            velocity,
            jacobians,
        })
    }

    pub fn state(&self, tau: R) -> Result<LinearizedState<R>, EvaluationError> {
        LinearizedState::from_pose_and_velocity(self.pose(tau)?, self.velocity(tau)?)
    }

    pub fn kinematics(&self, tau: R) -> Result<LinearizedKinematics<R>, EvaluationError> {
        validate_tau(tau)?;
        let basis = Basis::new(tau);
        let increments = self.spline.increments(&basis);
        self.kinematics_from_increments(&basis, &increments, &self.increment_jacobians(&basis))
    }

    /// Share rotational exponentials and exponential Jacobians for joint IMU evaluation.
    pub fn pose_and_kinematics(
        &self,
        tau: R,
    ) -> Result<(LinearizedPose<R>, LinearizedKinematics<R>), EvaluationError> {
        validate_tau(tau)?;
        let basis = Basis::new(tau);
        let increments = self.spline.increments(&basis);
        let jacobians = self.increment_jacobians(&basis);
        Ok((
            self.pose_from_increments(tau, &basis, &increments, &jacobians)?,
            self.kinematics_from_increments(&basis, &increments, &jacobians)?,
        ))
    }

    fn kinematics_from_increments(
        &self,
        basis: &Basis<R>,
        increments: &[UnitQuaternion<R>; 3],
        increment_jacobians: &[Matrix3<R>; 3],
    ) -> Result<LinearizedKinematics<R>, EvaluationError> {
        let mut omega = Vector::<R>::zeros();
        let mut derivatives = [Matrix3::<R>::zeros(); 4];
        // Differentiate the body-rate recurrence: ω_new = Aᵀ ω_old + β̇ d.
        for (i, increment) in increments.iter().enumerate() {
            let inverse = increment.to_rotation_matrix().inverse().into_inner();
            let rotated = inverse * omega;
            let beta_dot = basis.cumulative_first[i] * self.spline.inverse_duration;
            let j = rotated.cross_matrix() * increment_jacobians[i]
                + Matrix3::<R>::identity() * beta_dot;
            // Earlier increments have introduced no derivatives, then controls
            // 0..=1, then 0..=2. The remaining matrices are structurally zero.
            let active = if i == 0 { 0 } else { i + 1 };
            for derivative in &mut derivatives[..active] {
                *derivative = inverse * *derivative;
            }
            derivatives[i] += j * self.delta_jacobians[i][0];
            derivatives[i + 1] += j * self.delta_jacobians[i][1];
            omega = rotated + self.spline.rotation_deltas[i] * beta_dot;
        }
        let angular_velocity_jacobians = derivatives.map(|d| {
            let mut j = SMatrix::<R, 3, 6>::zeros();
            j.fixed_view_mut::<3, 3>(0, 0).copy_from(&d);
            j
        });
        finite(
            omega
                .iter()
                .chain(angular_velocity_jacobians.iter().flat_map(|j| j.iter())),
        )?;
        Ok(LinearizedKinematics {
            angular_velocity: Vector3::wrap(omega),
            angular_velocity_jacobians,
        })
    }

    fn translation_jacobians(
        &self,
        weights: [R; 4],
    ) -> Result<[SMatrix<R, 3, 6>; 4], EvaluationError> {
        let jacobians = std::array::from_fn(|i| {
            let mut j = SMatrix::<R, 3, 6>::zeros();
            j.fixed_view_mut::<3, 3>(0, 3)
                .copy_from(&(self.control_rotations[i] * weights[i]));
            j
        });
        finite(jacobians.iter().flat_map(|j| j.iter()))?;
        Ok(jacobians)
    }
}

/// Degree-three cardinal basis and normalized-time derivatives.
struct Basis<R> {
    value: [R; 4],
    first: [R; 4],
    cumulative: [R; 3],
    cumulative_first: [R; 3],
}

impl<R: RealField + Copy> Basis<R> {
    fn new(u: R) -> Self {
        let c = scalar::<R>;
        let u2 = u * u;
        let u3 = u2 * u;
        let one_minus = R::one() - u;
        let value = [
            one_minus * one_minus * one_minus * c(1.0 / 6.0),
            (c(3.0) * u3 - c(6.0) * u2 + c(4.0)) * c(1.0 / 6.0),
            (-c(3.0) * u3 + c(3.0) * u2 + c(3.0) * u + R::one()) * c(1.0 / 6.0),
            u3 * c(1.0 / 6.0),
        ];
        let first = [
            -one_minus * one_minus * c(0.5),
            c(1.5) * u2 - c(2.0) * u,
            -c(1.5) * u2 + u + c(0.5),
            u2 * c(0.5),
        ];
        Self {
            value,
            first,
            cumulative: [R::one() - value[0], value[2] + value[3], value[3]],
            cumulative_first: [-first[0], first[2] + first[3], first[3]],
        }
    }
}

fn validate_tau<R: RealField + Copy>(tau: R) -> Result<(), EvaluationError> {
    if tau >= R::zero() && tau <= R::one() {
        Ok(())
    } else {
        Err(EvaluationError::InvalidEvaluation)
    }
}
