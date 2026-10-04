use coordinate_systems::Robot;
use fagra::{BlockId, EvaluationError, Factor, LinearizationSink, StateKey, StateStore};
use linear_algebra::Vector3;
use nalgebra::{Matrix3, RealField, SMatrix, SVector, UnitQuaternion};

use super::common;
use crate::variables::{ImuBias, PoseControl};

fn validate_bias_tau<R: RealField + Copy>(tau: R) -> Result<(), EvaluationError> {
    if tau >= R::zero() && tau <= R::one() {
        Ok(())
    } else {
        Err(EvaluationError::InvalidEvaluation)
    }
}

/// Instantaneous gyro and optional SDK tilt, sharing spline preparation when both are active.
///
/// Gyro residuals are body angular velocity plus interpolated gyro bias minus measured
/// angular velocity.
/// Bias uses two independent coarse knots, never the pose spline. Each sample contributes
/// gyroscope rows (unless their root is zero) and optional tilt rows within one factor scope.
/// Noise roots whiten robot-axis errors.
#[derive(Clone, Debug)]
pub struct ImuKinematics<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub biases: [StateKey<ImuBias<R>>; 2],
    pub duration: R,
    pub tau: R,
    /// Linear interpolation fraction on the independent, coarse bias interval.
    pub bias_tau: R,
    /// Angular velocity in Robot axes, rad/s.
    pub angular_velocity: Vector3<Robot, R>,
    /// SDK attitude's unit up direction in Robot axes. Comparing all three
    /// components of Rᵀ*[0,0,1] distinguishes inversion without observing yaw.
    /// The exact antipode is a stationary maximum.
    pub measured_up: Option<Vector3<Robot, R>>,
    pub gyroscope_information_root: Matrix3<R>,
    pub tilt_information_root: Matrix3<R>,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>> + StateStore<ImuBias<R>>> Factor<S>
    for ImuKinematics<R>
{
    type Scalar = R;
    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
        for key in self.biases {
            visitor(key.block_id());
        }
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        self.validate()?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let biases = [states.get(self.biases[0])?, states.get(self.biases[1])?];
        let observe_gyro = self
            .gyroscope_information_root
            .iter()
            .any(|v| *v != R::zero());
        let mut cost = R::zero();
        let gyro_bias = biases[0].gyroscope.inner * (R::one() - self.bias_tau)
            + biases[1].gyroscope.inner * self.bias_tau;
        common::finite(gyro_bias.iter())?;
        let (k, pose) = match (observe_gyro, self.measured_up.is_some()) {
            (true, true) => {
                let (pose, k) = spline.pose_and_kinematics(self.tau)?;
                (Some(k), Some(pose))
            }
            (true, false) => (Some(spline.kinematics(self.tau)?), None),
            (false, true) => (None, Some(spline.pose(self.tau)?)),
            (false, false) => (None, None),
        };
        if let Some(k) = k {
            cost += common::cost(
                &(self.gyroscope_information_root
                    * (k.inner + gyro_bias - self.angular_velocity.inner)),
            )?;
        }
        if let Some((measured, pose)) = self.measured_up.zip(pose.as_ref()) {
            cost += common::cost(&tilt(
                &pose.inner.rotation,
                &measured,
                &self.tilt_information_root,
            )?)?;
        }
        common::checked_cost(cost)
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        self.validate()?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let linearized = spline.linearize()?;
        let biases = [states.get(self.biases[0])?, states.get(self.biases[1])?];
        let observe_gyro = self
            .gyroscope_information_root
            .iter()
            .any(|v| *v != R::zero());
        let weights = [R::one() - self.bias_tau, self.bias_tau];
        let gyro_bias =
            biases[0].gyroscope.inner * weights[0] + biases[1].gyroscope.inner * weights[1];
        common::finite(gyro_bias.iter())?;
        let (k, pose) = match (observe_gyro, self.measured_up.is_some()) {
            (true, true) => {
                let (pose, k) = linearized.pose_and_kinematics(self.tau)?;
                (Some(k), Some(pose))
            }
            (true, false) => (Some(linearized.kinematics(self.tau)?), None),
            (false, true) => (None, Some(linearized.pose(self.tau)?)),
            (false, false) => (None, None),
        };
        if let Some(k) = k {
            let residual = self.gyroscope_information_root
                * (k.angular_velocity.inner + gyro_bias - self.angular_velocity.inner);
            let jacobians = k
                .angular_velocity_jacobians
                .map(|j| self.gyroscope_information_root * j);
            let gyro_jacobians = weights.map(|weight| {
                let mut j = SMatrix::<R, 3, 6>::zeros();
                j.fixed_columns_mut::<3>(0)
                    .copy_from(&(self.gyroscope_information_root * weight));
                j
            });
            common::emit_pose_and_bias(
                sink,
                &self.controls,
                &self.biases,
                &residual,
                &jacobians,
                &gyro_jacobians,
            )?;
        }
        if let Some((measured, pose)) = self.measured_up.zip(pose.as_ref()) {
            let residual = tilt(
                &pose.pose.inner.rotation,
                &measured,
                &self.tilt_information_root,
            )?;
            let derivative = tilt_jacobian(&pose.pose.inner.rotation, &self.tilt_information_root);
            let jacobians = pose
                .jacobians
                .each_ref()
                .map(|j| derivative * j.fixed_rows::<3>(0));
            common::emit(sink, &self.controls, &residual, &jacobians)?;
        }
        Ok(())
    }
}

impl<R: RealField + Copy> ImuKinematics<R> {
    fn validate(&self) -> Result<(), EvaluationError> {
        validate_bias_tau(self.tau)?;
        validate_bias_tau(self.bias_tau)?;
        common::finite(
            self.gyroscope_information_root
                .iter()
                .chain(self.tilt_information_root.iter())
                .chain(self.angular_velocity.inner.iter()),
        )
    }
}

fn tilt<R: RealField + Copy>(
    rotation: &UnitQuaternion<R>,
    measured: &Vector3<Robot, R>,
    root: &Matrix3<R>,
) -> Result<nalgebra::Vector3<R>, EvaluationError> {
    common::validate_up(&measured.inner)?;
    common::finite(root.iter())?;
    Ok(root * (common::up(rotation) - measured.inner))
}

fn tilt_jacobian<R: RealField + Copy>(
    rotation: &UnitQuaternion<R>,
    root: &Matrix3<R>,
) -> Matrix3<R> {
    root * common::up(rotation).cross_matrix()
}

/// Scalar change in evaluated yaw from segment start to `end_tau`, without an absolute yaw anchor.
///
/// Residual: `W * wrap(heading(R(end_tau)) - heading(R(0)) - measured_yaw_change)`.
/// Both rotations are evaluated on the spline, not read from control poses.
/// Heading is `atan2(R[1, 0], R[0, 0])`, not the z component of an SO(3) log.
/// Heading is undefined when the robot x axis is vertical; such evaluations must
/// fail. Wrapped-angle derivatives apply away from the principal branch cut.
/// Use `end_tau = 1` for a completed interval. A temporary current-interval
/// constraint must be removed before marginalization when its permanent replacement arrives.
#[derive(Clone, Debug)]
pub struct RelativeYaw<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub duration: R,
    /// Normalized observation time in [0, 1].
    pub end_tau: R,
    /// Wrapped end-minus-start yaw, in radians.
    pub measured_yaw_change: R,
    /// Whitens the yaw difference in radians. For independent attitude samples,
    /// difference variance is the sum of their yaw variances.
    pub information_root: R,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>> Factor<S> for RelativeYaw<R> {
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        common::positive(self.information_root)?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let error = common::yaw_error(
            &spline.pose(R::zero())?.inner.rotation,
            &spline.pose(self.end_tau)?.inner.rotation,
            self.measured_yaw_change,
        )?;
        common::cost(&SVector::<R, 1>::new(self.information_root * error))
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        common::positive(self.information_root)?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let linearized = spline.linearize()?;
        let a = linearized.pose(R::zero())?;
        let b = linearized.pose(self.end_tau)?;
        let error = common::yaw_error(
            &a.pose.inner.rotation,
            &b.pose.inner.rotation,
            self.measured_yaw_change,
        )?;
        let ja = common::heading_jacobian(&a.pose.inner.rotation)? * self.information_root;
        let jb = common::heading_jacobian(&b.pose.inner.rotation)? * self.information_root;
        let jacobians = std::array::from_fn(|i| {
            jb * b.jacobians[i].fixed_rows::<3>(0) - ja * a.jacobians[i].fixed_rows::<3>(0)
        });
        common::emit(
            sink,
            &self.controls,
            &SVector::<R, 1>::new(self.information_root * error),
            &jacobians,
        )
    }
}
