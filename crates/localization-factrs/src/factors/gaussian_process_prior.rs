use factrs::{
    linalg::{ForwardProp, Matrix3, Numeric, VectorX},
    traits::{Residual, Variable},
    variables::{MatrixLieGroup, SE23},
};
use nalgebra::{SMatrix, SVector};

const DEFAULT_PROCESS_COVARIANCE_SCALE: f64 = 10.0;

#[derive(Debug, Clone)]
pub struct GaussianProcessPriorFactor {
    duration: f64,
    information_root: SMatrix<f64, 9, 9>,
    use_start_velocity: bool,
}

impl GaussianProcessPriorFactor {
    pub fn new(
        duration: f64,
        gyroscope_process_noise: &Matrix3<f64>,
        accelerometer_process_noise: &Matrix3<f64>,
    ) -> Self {
        Self::new_with_process_covariance_scale(
            duration,
            gyroscope_process_noise,
            accelerometer_process_noise,
            DEFAULT_PROCESS_COVARIANCE_SCALE,
        )
    }

    pub fn new_with_process_covariance_scale(
        duration: f64,
        gyroscope_process_noise: &Matrix3<f64>,
        accelerometer_process_noise: &Matrix3<f64>,
        process_covariance_scale: f64,
    ) -> Self {
        Self::new_with_options(
            duration,
            gyroscope_process_noise,
            accelerometer_process_noise,
            process_covariance_scale,
            true,
        )
    }

    pub fn new_zero_start_velocity_bridge(
        duration: f64,
        gyroscope_process_noise: &Matrix3<f64>,
        accelerometer_process_noise: &Matrix3<f64>,
        process_covariance_scale: f64,
    ) -> Self {
        Self::new_with_options(
            duration,
            gyroscope_process_noise,
            accelerometer_process_noise,
            process_covariance_scale,
            false,
        )
    }

    fn new_with_options(
        duration: f64,
        gyroscope_process_noise: &Matrix3<f64>,
        accelerometer_process_noise: &Matrix3<f64>,
        process_covariance_scale: f64,
        use_start_velocity: bool,
    ) -> Self {
        assert!(duration > 0.0, "duration must be positive");
        assert!(
            process_covariance_scale > 0.0,
            "process covariance scale must be positive"
        );

        let covariance = Self::process_noise_covariance(
            gyroscope_process_noise,
            accelerometer_process_noise,
            duration,
        ) * process_covariance_scale;

        let information_root = covariance
            .cholesky()
            .expect("process covariance must be positive definite")
            .l()
            .try_inverse()
            .expect("process covariance Cholesky factor must be invertible");

        Self {
            duration,
            information_root,
            use_start_velocity,
        }
    }

    /// Discrete covariance for the residual order:
    ///
    /// [rotation, velocity, position].
    ///
    /// Rotation is modeled as angular-rate random walk:
    ///     theta ~ N(0, Qg * dt)
    ///
    /// Translation is modeled as white-noise-on-acceleration:
    ///     v1 = v0 + w_v
    ///     p1 = p0 + v0 * dt + w_p
    ///
    /// with
    ///     cov([w_v, w_p]) =
    ///     [Qa * dt,        Qa * dt^2 / 2]
    ///     [Qa * dt^2 / 2,  Qa * dt^3 / 3]
    pub fn process_noise_covariance(
        gyroscope_process_noise: &Matrix3<f64>,
        accelerometer_process_noise: &Matrix3<f64>,
        dt: f64,
    ) -> SMatrix<f64, 9, 9> {
        assert!(dt > 0.0, "dt must be positive");

        let mut covariance = SMatrix::<f64, 9, 9>::zeros();

        covariance
            .fixed_view_mut::<3, 3>(0, 0)
            .copy_from(&(gyroscope_process_noise * dt));

        covariance
            .fixed_view_mut::<3, 3>(3, 3)
            .copy_from(&(accelerometer_process_noise * dt));

        covariance
            .fixed_view_mut::<3, 3>(3, 6)
            .copy_from(&(accelerometer_process_noise * (dt * dt / 2.0)));

        covariance
            .fixed_view_mut::<3, 3>(6, 3)
            .copy_from(&(accelerometer_process_noise * (dt * dt / 2.0)));

        covariance
            .fixed_view_mut::<3, 3>(6, 6)
            .copy_from(&(accelerometer_process_noise * (dt * dt * dt / 3.0)));

        covariance
    }

    fn raw_residual<T: Numeric>(&self, start_pose: &SE23<T>, end_pose: &SE23<T>) -> SVector<T, 9> {
        let dt = T::from(self.duration);

        let start_rotation_inverse = start_pose.rot().inverse();

        let rotation_error_log = start_rotation_inverse.compose(end_pose.rot()).log();
        let rotation_error = SVector::<T, 3>::from_column_slice(rotation_error_log.as_slice());

        let start_velocity = if self.use_start_velocity {
            start_pose.uvw().into_owned()
        } else {
            SVector::<T, 3>::zeros()
        };
        let velocity_error_global = end_pose.uvw() - start_velocity;
        let position_error_global = end_pose.xyz() - start_pose.xyz() - start_velocity * dt;

        let velocity_error_local = start_rotation_inverse.apply(velocity_error_global.as_view());
        let position_error_local = start_rotation_inverse.apply(position_error_global.as_view());

        let mut residual = SVector::<T, 9>::zeros();

        residual
            .fixed_view_mut::<3, 1>(0, 0)
            .copy_from(&rotation_error);

        residual
            .fixed_view_mut::<3, 1>(3, 0)
            .copy_from(&velocity_error_local);

        residual
            .fixed_view_mut::<3, 1>(6, 0)
            .copy_from(&position_error_local);

        residual
    }

    fn residual_impl<T: Numeric>(&self, start_pose: &SE23<T>, end_pose: &SE23<T>) -> VectorX<T> {
        let raw_residual = self.raw_residual(start_pose, end_pose);
        let whitened = self.information_root.cast::<T>() * raw_residual;

        let mut residual = VectorX::<T>::zeros(9);
        residual.copy_from(&whitened);
        residual
    }
}

#[factrs::mark]
impl Residual for GaussianProcessPriorFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        9
    }

    fn residual<T: Numeric>(&self, (start_pose, end_pose): (SE23<T>, SE23<T>)) -> VectorX<T> {
        self.residual_impl(&start_pose, &end_pose)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use factrs::{
        core::{SO3, Values, Vector3},
        traits::Variable,
        variables::SE23,
    };
    use nalgebra::{Matrix3, vector};

    use crate::symbols::State;

    fn state(position: Vector3, velocity: Vector3) -> SE23 {
        SE23::from_rot_vel_trans(SO3::identity(), velocity, position)
    }

    #[test]
    fn residual_is_zero_for_constant_velocity_segment() {
        let factor =
            GaussianProcessPriorFactor::new(2.0, &Matrix3::identity(), &Matrix3::identity());

        let start = state(vector![1.0, 2.0, 3.0], vector![0.5, -1.0, 2.0]);

        let end = state(
            vector![2.0, 0.0, 7.0], // p0 + v0 * 2.0
            vector![0.5, -1.0, 2.0],
        );

        let residual = factor.residual_impl(&start, &end);

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn bridge_residual_does_not_propagate_stale_start_velocity() {
        let factor = GaussianProcessPriorFactor::new_zero_start_velocity_bridge(
            2.0,
            &Matrix3::identity(),
            &Matrix3::identity(),
            1.0,
        );

        let start = state(vector![1.0, 2.0, 3.0], vector![0.0, 3.0, 0.0]);
        let end = state(vector![1.0, 2.0, 3.0], Vector3::zeros());

        let residual = factor.residual_impl(&start, &end);

        assert!(
            residual.iter().all(|value| value.abs() < 1e-9),
            "expected zero residual, got {residual:?}"
        );
    }

    #[test]
    fn residual_is_nonzero_for_accelerating_segment() {
        let factor =
            GaussianProcessPriorFactor::new(1.0, &Matrix3::identity(), &Matrix3::identity());

        let start = state(vector![0.0, 0.0, 0.0], vector![0.0, 0.0, 0.0]);
        let end = state(vector![1.0, 0.0, 0.0], vector![2.0, 0.0, 0.0]);

        let residual = factor.residual_impl(&start, &end);

        assert!(
            residual.norm() > 1e-9,
            "expected nonzero residual for acceleration"
        );
    }

    #[test]
    fn process_covariance_is_positive_definite() {
        let covariance = GaussianProcessPriorFactor::process_noise_covariance(
            &(Matrix3::identity() * 0.01),
            &(Matrix3::identity() * 0.1),
            0.2,
        );

        assert!(covariance.cholesky().is_some());
    }

    #[test]
    fn jacobian_has_expected_shape() {
        let factor =
            GaussianProcessPriorFactor::new(1.0, &Matrix3::identity(), &Matrix3::identity());

        let mut values = Values::new();
        values.insert(
            State(0),
            state(vector![0.0, 0.0, 0.0], vector![1.0, 0.0, 0.0]),
        );
        values.insert(
            State(1),
            state(vector![1.0, 0.0, 0.0], vector![1.0, 0.0, 0.0]),
        );

        let keys = [State(0).into(), State(1).into()];
        let jacobian =
            factrs::residuals::ErasedResidual::residual_jacobian(&factor, &values, &keys)
                .expect("factor should linearize")
                .diff;

        assert_eq!(jacobian.nrows(), 9);
        assert_eq!(jacobian.ncols(), 18);
    }
}
