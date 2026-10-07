use fagra::{BlockId, EvaluationError, Factor, LinearizationSink, StateKey, StateStore};
use nalgebra::{RealField, SMatrix, SVector};

use super::common;
use crate::variables::{PoseControl, TrajectoryState, rotation};

/// Sampled process-model prior between the evaluated endpoints of a spline segment.
///
/// Residual order is rotation, velocity, position, expressed relative to the start
/// orientation. The information root includes duration-dependent process covariance.
/// For robot-to-local rotations `R0`, `R1`, local positions `p0`, `p1`, and local
/// velocities `v0 = ṗ(0)`, `v1 = ṗ(1)`, the raw residual is
/// `[Log(R0⁻¹ R1), R0ᵀ(v1 - v0), R0ᵀ(p1 - p0 - duration * v0)]`.
/// When `use_start_velocity` is false, substitute zero for `v0` in both terms;
/// this changes the prediction but does not change the spline's kinematics.
/// All quantities come from `spline.state(0)` and `spline.state(1)`, not control
/// values. The four controls contribute to both endpoint evaluations; combine their
/// Jacobian contributions before emission. This sampled prior preserves the process
/// model's intent, but is not the old independent pose/velocity GP parameterization.
#[derive(Clone, Debug)]
pub struct MotionPrior<R: RealField + Copy = f64> {
    pub controls: [StateKey<PoseControl<R>>; 4],
    pub duration: R,
    pub information_root: SMatrix<R, 9, 9>,
    /// False for long-gap bridges that predict from zero rather than stale velocity.
    pub use_start_velocity: bool,
}

impl<R: RealField + Copy, S: StateStore<PoseControl<R>>> Factor<S> for MotionPrior<R> {
    type Scalar = R;

    fn visit_variables(&self, mut visitor: impl FnMut(BlockId)) {
        for key in self.controls {
            visitor(key.block_id());
        }
    }

    fn cost(&self, states: &S) -> Result<R, EvaluationError> {
        common::finite(self.information_root.iter())?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        common::cost(
            &(self.information_root
                * self.error(&spline.state(R::zero())?, &spline.state(R::one())?)?),
        )
    }

    fn linearize<L: LinearizationSink<Scalar = R>>(
        &self,
        states: &S,
        sink: &mut L,
    ) -> Result<(), EvaluationError> {
        common::finite(self.information_root.iter())?;
        let spline = common::spline(states, &self.controls, self.duration)?;
        let linearized = spline.linearize()?;
        let start = linearized.state(R::zero())?;
        let end = linearized.state(R::one())?;
        let error = self.error(&start.state, &end.state)?;
        let relative = (start.state.pose.inner.rotation.inverse() * end.state.pose.inner.rotation)
            .to_rotation_matrix()
            .into_inner();
        let inverse = rotation::right_jacobian_inverse(error.fixed_rows::<3>(0).into_owned());
        let mut a = SMatrix::<R, 9, 9>::zeros();
        let mut b = SMatrix::<R, 9, 9>::zeros();
        a.fixed_view_mut::<3, 3>(0, 0)
            .copy_from(&(-inverse * relative.transpose()));
        b.fixed_view_mut::<3, 3>(0, 0).copy_from(&inverse);
        a.fixed_view_mut::<3, 3>(3, 0)
            .copy_from(&error.fixed_rows::<3>(3).into_owned().cross_matrix());
        a.fixed_view_mut::<3, 3>(6, 0)
            .copy_from(&error.fixed_rows::<3>(6).into_owned().cross_matrix());
        a.fixed_view_mut::<3, 3>(6, 6)
            .set_diagonal(&nalgebra::Vector3::repeat(-R::one()));
        if self.use_start_velocity {
            a.fixed_view_mut::<3, 3>(3, 3)
                .set_diagonal(&nalgebra::Vector3::repeat(-R::one()));
            a.fixed_view_mut::<3, 3>(6, 3)
                .set_diagonal(&nalgebra::Vector3::repeat(-self.duration));
        }
        b.fixed_view_mut::<3, 3>(3, 3).copy_from(&relative);
        b.fixed_view_mut::<3, 3>(6, 6).copy_from(&relative);
        let a = self.information_root * a;
        let b = self.information_root * b;
        let jacobians = std::array::from_fn(|i| a * start.jacobians[i] + b * end.jacobians[i]);
        common::emit(
            sink,
            &self.controls,
            &(self.information_root * error),
            &jacobians,
        )
    }
}

impl<R: RealField + Copy> MotionPrior<R> {
    /// Square-root information for rotational diffusion and translational white
    /// noise on acceleration. Arguments are duration and the two positive spectral
    /// densities. The velocity/position covariance includes their shared-noise cross term.
    pub fn information_root(
        duration: R,
        rotation_noise: R,
        acceleration_noise: R,
    ) -> Result<SMatrix<R, 9, 9>, EvaluationError> {
        for value in [duration, rotation_noise, acceleration_noise] {
            common::positive(value)?;
        }
        let mut covariance = SMatrix::<R, 9, 9>::zeros();
        let identity = nalgebra::Matrix3::<R>::identity();
        covariance
            .fixed_view_mut::<3, 3>(0, 0)
            .copy_from(&(identity * rotation_noise * duration));
        covariance
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&(identity * acceleration_noise * duration));
        covariance.fixed_view_mut::<3, 3>(6, 6).copy_from(
            &(identity * acceleration_noise * duration.powi(3) / rotation::scalar::<R>(3.0)),
        );
        let cross = identity * acceleration_noise * duration.powi(2) / rotation::scalar::<R>(2.0);
        covariance.fixed_view_mut::<3, 3>(3, 6).copy_from(&cross);
        covariance.fixed_view_mut::<3, 3>(6, 3).copy_from(&cross);
        common::finite(covariance.iter())?;
        let root = covariance
            .cholesky()
            .and_then(|l| l.l().try_inverse())
            .ok_or(EvaluationError::InvalidEvaluation)?;
        common::finite(root.iter())?;
        Ok(root)
    }

    fn error(
        &self,
        start: &TrajectoryState<R>,
        end: &TrajectoryState<R>,
    ) -> Result<SVector<R, 9>, EvaluationError> {
        let inverse = start.pose.inner.rotation.inverse();
        let rotation = common::rotation_log(&(inverse * end.pose.inner.rotation))?;
        let velocity = if self.use_start_velocity {
            start.velocity.inner
        } else {
            nalgebra::Vector3::zeros()
        };
        let mut error = SVector::<R, 9>::zeros();
        error.fixed_rows_mut::<3>(0).copy_from(&rotation);
        error
            .fixed_rows_mut::<3>(3)
            .copy_from(&(inverse * (end.velocity.inner - velocity)));
        error.fixed_rows_mut::<3>(6).copy_from(
            &(inverse
                * (end.pose.inner.translation.vector
                    - start.pose.inner.translation.vector
                    - velocity * self.duration)),
        );
        Ok(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_root_whitens_correlated_velocity_and_position() {
        let dt: f64 = 0.2;
        let q = 7.0;
        let axis = nalgebra::Matrix3::new(
            0.03 * dt,
            0.0,
            0.0,
            0.0,
            q * dt,
            q * dt * dt / 2.0,
            0.0,
            q * dt * dt / 2.0,
            q * dt * dt * dt / 3.0,
        );
        let covariance = SMatrix::<f64, 9, 9>::from_fn(|i, j| {
            if i % 3 == j % 3 {
                axis[(i / 3, j / 3)]
            } else {
                0.0
            }
        });
        let root = MotionPrior::information_root(dt, 0.03, q).unwrap();
        assert!((root * covariance * root.transpose() - SMatrix::identity()).amax() < 1.0e-12);
        let root = MotionPrior::<f32>::information_root(dt as f32, 0.03, q as f32).unwrap();
        assert!(
            (root * covariance.cast::<f32>() * root.transpose() - SMatrix::identity()).amax()
                < 1.0e-5
        );
        for (dt, rotation, acceleration) in [
            (0.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
            (1.0, 0.0, 1.0),
            (1.0, 1.0, f64::NAN),
            (f64::MAX, 1.0, 1.0),
        ] {
            assert!(MotionPrior::information_root(dt, rotation, acceleration).is_err());
        }
    }
}
