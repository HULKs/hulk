use coordinate_systems::{Local, Robot};
use fagra::{BlockId, EvaluationError, Factor, LinearizationSink, StateKey, StateStore};
use linear_algebra::Vector3;
use nalgebra::{Matrix3, RealField, SMatrix, SVector};

use super::common;
use crate::{
    preintegration::{CorrectedImuDelta, ImuDelta, PreintegrationInformation},
    spline::{LinearizedPoseSpline, LinearizedState, PoseSpline},
    variables::{ImuBias, PoseControl, TrajectoryState, rotation},
};

/// GTSAM-referenced inertial endpoint constraint on evaluated spline states,
/// not on the B-spline control values. One interval stays inside a pose segment.
#[derive(Clone, Debug)]
pub struct PreintegratedImu<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub biases: [StateKey<ImuBias<R>>; 2],
    pub duration: R,
    pub start_tau: R,
    pub end_tau: R,
    pub delta: ImuDelta<R>,
    pub information: PreintegrationInformation<R>,
    /// Positive-up compensation, i.e. minus physical gravity in Local.
    pub gravity_compensation: Vector3<Local, R>,
    /// Robot origin to physical IMU. Integrating at that origin avoids noisy
    /// angular-acceleration differencing and retains tangential/centripetal motion.
    pub position: Vector3<Robot, R>,
}

impl<R: RealField + Copy> PreintegratedImu<R> {
    fn validate(&self) -> Result<(), EvaluationError> {
        common::positive(self.duration)?;
        common::positive(self.delta.duration)?;
        if self.end_tau <= self.start_tau
            || ((self.end_tau - self.start_tau) * self.duration - self.delta.duration).abs()
                > rotation::scalar::<R>(1e-5) * self.delta.duration
        {
            return Err(EvaluationError::InvalidEvaluation);
        }
        common::finite(
            self.position
                .inner
                .iter()
                .chain(self.gravity_compensation.inner.iter())
                .chain(self.delta.rotation.inner.coords.iter()),
        )?;
        match &self.information {
            PreintegrationInformation::Rotation(root) => common::finite(root.iter()),
            PreintegrationInformation::Full(root) => common::finite(root.iter()),
        }
    }

    fn sensor_state(
        &self,
        spline: &PoseSpline<R>,
        tau: R,
    ) -> Result<TrajectoryState<R>, EvaluationError> {
        if self.position == Vector3::zeros() {
            return spline.state(tau);
        }
        let (mut pose, k) = spline.pose_and_kinematics(tau)?;
        let robot_to_local = pose.orientation().rotation::<Robot>();
        let arm_velocity = Vector3::wrap(k.inner.cross(&self.position.inner));
        pose.inner.translation.vector += (robot_to_local * self.position).inner;
        Ok(TrajectoryState {
            pose,
            velocity: spline.velocity(tau)? + robot_to_local * arm_velocity,
        })
    }

    fn sensor_linearized(
        &self,
        spline: &LinearizedPoseSpline<'_, R>,
        tau: R,
    ) -> Result<LinearizedState<R>, EvaluationError> {
        if self.position == Vector3::zeros() {
            return spline.state(tau);
        }
        let (pose, k) = spline.pose_and_kinematics(tau)?;
        let mut sample = LinearizedState::from_pose_and_velocity(pose, spline.velocity(tau)?)?;
        let arm_velocity = k.angular_velocity.inner.cross(&self.position.inner);
        let r_cross = self.position.inner.cross_matrix();
        for (j, w) in sample
            .jacobians
            .iter_mut()
            .zip(&k.angular_velocity_jacobians)
        {
            let theta = j.fixed_rows::<3>(0).into_owned();
            let velocity = j.fixed_rows::<3>(3).into_owned()
                - arm_velocity.cross_matrix() * theta
                - r_cross * w;
            let position = j.fixed_rows::<3>(6).into_owned() - r_cross * theta;
            j.fixed_rows_mut::<3>(3).copy_from(&velocity);
            j.fixed_rows_mut::<3>(6).copy_from(&position);
        }
        let robot_to_local = sample.state.pose.orientation().rotation::<Robot>();
        sample.state.pose.inner.translation.vector += (robot_to_local * self.position).inner;
        sample.state.velocity += robot_to_local * Vector3::wrap(arm_velocity);
        Ok(sample)
    }
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>> + StateStore<ImuBias<R>>> Factor<S>
    for PreintegratedImu<R>
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
        let CorrectedImuDelta {
            rotation: dr,
            velocity: dv,
            position: dp,
            ..
        } = self
            .delta
            .corrected([states.get(self.biases[0])?, states.get(self.biases[1])?])?;
        match &self.information {
            PreintegrationInformation::Rotation(root) => {
                let a = spline.pose(self.start_tau)?;
                let b = spline.pose(self.end_tau)?;
                common::cost(
                    &(root
                        * common::rotation_log(
                            &(dr.inverse()
                                * a.orientation().rotation::<Robot>().inverse()
                                * b.orientation().rotation::<Robot>())
                            .inner,
                        )?),
                )
            }
            PreintegrationInformation::Full(root) => {
                let a = self.sensor_state(&spline, self.start_tau)?;
                let b = self.sensor_state(&spline, self.end_tau)?;
                let inverse = a.pose.orientation().rotation::<Robot>().inverse();
                let dt = self.delta.duration;
                let mut error = SVector::<R, 9>::zeros();
                error
                    .fixed_rows_mut::<3>(0)
                    .copy_from(&common::rotation_log(
                        &(dr.inverse() * inverse * b.pose.orientation().rotation::<Robot>()).inner,
                    )?);
                error.fixed_rows_mut::<3>(3).copy_from(
                    &(inverse * (b.velocity - a.velocity + self.gravity_compensation * dt) - dv)
                        .inner,
                );
                error.fixed_rows_mut::<3>(6).copy_from(
                    &(inverse
                        * (b.pose.position() - a.pose.position() - a.velocity * dt
                            + self.gravity_compensation * (rotation::scalar::<R>(0.5) * dt * dt))
                        - dp)
                        .inner,
                );
                common::cost(&(root * error))
            }
        }
    }
    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        self.validate()?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let linearized = spline.linearize()?;
        let CorrectedImuDelta {
            rotation: dr,
            velocity: dv,
            position: dp,
            rotation_jacobian: bias_rotation,
        } = self
            .delta
            .corrected([states.get(self.biases[0])?, states.get(self.biases[1])?])?;
        // State derivatives are right-local [rotation,velocity,position].
        match &self.information {
            PreintegrationInformation::Rotation(root) => {
                let a = linearized.pose(self.start_tau)?;
                let b = linearized.pose(self.end_tau)?;
                let relative = a.pose.orientation().rotation::<Robot>().inverse()
                    * b.pose.orientation().rotation::<Robot>();
                let difference = dr.inverse() * relative;
                let error = common::rotation_log(&difference.inner)?;
                let inverse = rotation::right_jacobian_inverse(error);
                let left = -inverse * relative.inner.to_rotation_matrix().inverse().into_inner();
                let bias = -inverse
                    * difference.inner.to_rotation_matrix().inverse().into_inner()
                    * bias_rotation;
                let pose = std::array::from_fn(|i| {
                    root * (left * a.jacobians[i].fixed_rows::<3>(0)
                        + inverse * b.jacobians[i].fixed_rows::<3>(0))
                });
                let biases = self
                    .delta
                    .bias_jacobians
                    .each_ref()
                    .map(|j| root * bias * j.fixed_rows::<3>(0));
                common::emit_pose_and_bias(
                    sink,
                    &self.controls,
                    &self.biases,
                    &(root * error),
                    &pose,
                    &biases,
                )
            }
            PreintegrationInformation::Full(root) => {
                let a = self.sensor_linearized(&linearized, self.start_tau)?;
                let b = self.sensor_linearized(&linearized, self.end_tau)?;
                let inverse_rotation = a.state.pose.orientation().rotation::<Robot>().inverse();
                let relative = inverse_rotation * b.state.pose.orientation().rotation::<Robot>();
                let difference = dr.inverse() * relative;
                let angle = common::rotation_log(&difference.inner)?;
                let dt = self.delta.duration;
                let predicted_v = inverse_rotation
                    * (b.state.velocity - a.state.velocity + self.gravity_compensation * dt);
                let predicted_p = inverse_rotation
                    * (b.state.pose.position() - a.state.pose.position() - a.state.velocity * dt
                        + self.gravity_compensation * (rotation::scalar::<R>(0.5) * dt * dt));
                let mut error = SVector::<R, 9>::zeros();
                error.fixed_rows_mut::<3>(0).copy_from(&angle);
                error
                    .fixed_rows_mut::<3>(3)
                    .copy_from(&(predicted_v - dv).inner);
                error
                    .fixed_rows_mut::<3>(6)
                    .copy_from(&(predicted_p - dp).inner);
                let inverse = rotation::right_jacobian_inverse(angle);
                let rel = relative.inner.to_rotation_matrix().into_inner();
                let mut left = SMatrix::<R, 9, 9>::zeros();
                let mut right = SMatrix::<R, 9, 9>::zeros();
                left.fixed_view_mut::<3, 3>(0, 0)
                    .copy_from(&(-inverse * rel.transpose()));
                right.fixed_view_mut::<3, 3>(0, 0).copy_from(&inverse);
                left.fixed_view_mut::<3, 3>(3, 0)
                    .copy_from(&predicted_v.inner.cross_matrix());
                left.fixed_view_mut::<3, 3>(6, 0)
                    .copy_from(&predicted_p.inner.cross_matrix());
                left.fixed_view_mut::<3, 3>(3, 3)
                    .copy_from(&-Matrix3::identity());
                left.fixed_view_mut::<3, 3>(6, 3)
                    .copy_from(&(Matrix3::identity() * -dt));
                left.fixed_view_mut::<3, 3>(6, 6)
                    .copy_from(&-Matrix3::identity());
                right.fixed_view_mut::<3, 3>(3, 3).copy_from(&rel);
                right.fixed_view_mut::<3, 3>(6, 6).copy_from(&rel);
                let left = root * left;
                let right = root * right;
                let pose = std::array::from_fn(|i| left * a.jacobians[i] + right * b.jacobians[i]);
                let rotation_bias = -inverse
                    * difference.inner.to_rotation_matrix().inverse().into_inner()
                    * bias_rotation;
                let biases = self.delta.bias_jacobians.each_ref().map(|j| {
                    let mut result = -*j;
                    result
                        .fixed_rows_mut::<3>(0)
                        .copy_from(&(rotation_bias * j.fixed_rows::<3>(0)));
                    root * result
                });
                common::emit_pose_and_bias(
                    sink,
                    &self.controls,
                    &self.biases,
                    &(root * error),
                    &pose,
                    &biases,
                )
            }
        }
    }
}
