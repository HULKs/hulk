use std::time::SystemTime;

use factrs::{
    core::Vector3,
    linalg::{ForwardProp, Matrix3, Numeric, VectorX},
    traits::{Residual, Variable},
    variables::{MatrixLieGroup, SE23},
};

use crate::{
    SE23Spline,
    measurements::ImuMeasurement,
    utils::{interval_dt, tau},
};

#[derive(Debug, Clone)]
pub(crate) struct IntervalGaussianProcessImuFactor {
    measurements: Vec<ImuMeasurement>,
    measurement_taus: Vec<f64>,
    gyroscope_information_root: Matrix3<f64>,
    accelerometer_information_root: Matrix3<f64>,
    use_accelerometer_measurements: bool,
    gravity: Vector3<f64>,
    start_time: SystemTime,
    end_time: SystemTime,
    duration: f64,
}

#[factrs::mark]
impl Residual for IntervalGaussianProcessImuFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        let residuals_per_measurement = if self.use_accelerometer_measurements {
            6
        } else {
            3
        };
        self.measurements.len() * residuals_per_measurement
    }

    fn residual<T: Numeric>(&self, (start, end): (SE23<T>, SE23<T>)) -> VectorX<T> {
        // TODO: check if exists in Residual
        self.residuals_on_spline(start, end)
    }
}

impl IntervalGaussianProcessImuFactor {
    pub(crate) fn new(
        measurements: Vec<ImuMeasurement>,
        gyroscope_noise: Matrix3<f64>,
        accelerometer_noise: Matrix3<f64>,
        use_accelerometer_measurements: bool,
        gravity: Vector3<f64>,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Self {
        // The inverse of the lower Cholesky factor is required to whiten the residuals
        // such that the resulting error vectors have a covariance of the identity matrix.
        let gyroscope_information_root = gyroscope_noise
            .cholesky()
            .expect("gyroscope noise covariance must be positive definite")
            .l()
            .try_inverse()
            .expect("gyroscope lower triangular matrix must be invertible");

        let accelerometer_information_root = accelerometer_noise
            .cholesky()
            .expect("accelerometer noise covariance must be positive definite")
            .l()
            .try_inverse()
            .expect("accelerometer lower triangular matrix must be invertible");

        let duration = interval_dt::<f64>(start_time, end_time);
        let measurement_taus = measurements
            .iter()
            .map(|measurement| tau::<f64>(start_time, end_time, measurement.time))
            .collect();

        Self {
            measurements,
            measurement_taus,
            gyroscope_information_root,
            accelerometer_information_root,
            use_accelerometer_measurements,
            gravity,
            start_time,
            end_time,
            duration,
        }
    }

    pub(crate) fn extend_measurements(
        &mut self,
        measurements: impl IntoIterator<Item = ImuMeasurement>,
    ) {
        for measurement in measurements {
            self.measurement_taus.push(tau::<f64>(
                self.start_time,
                self.end_time,
                measurement.time,
            ));
            self.measurements.push(measurement);
        }
    }

    fn residuals_on_spline<T: Numeric>(
        &self,
        pose_start: SE23<T>,
        pose_end: SE23<T>,
    ) -> VectorX<T> {
        let dt = T::from(self.duration);
        let spline = SE23Spline::new(pose_start, pose_end, dt);

        assert_eq!(self.measurements.len(), self.measurement_taus.len());

        let residuals_per_measurement = if self.use_accelerometer_measurements {
            6
        } else {
            3
        };
        let mut residual = VectorX::<T>::zeros(residuals_per_measurement * self.measurements.len());

        for (index, (measurement, measurement_tau)) in self
            .measurements
            .iter()
            .zip(self.measurement_taus.iter())
            .enumerate()
        {
            let measurement_tau = T::from(*measurement_tau);
            let current_pose = spline.evaluate(measurement_tau);
            let kinematics = spline.evaluate_derivative(measurement_tau);

            let predicted_gyro = kinematics.angular_velocity_local;

            let gyro_residual =
                predicted_gyro - measurement.state.angular_velocity.inner.cast::<T>();

            let whitened_gyroscope_error =
                self.gyroscope_information_root.cast::<T>() * gyro_residual;

            residual
                .fixed_view_mut::<3, 1>(residuals_per_measurement * index, 0)
                .copy_from(&whitened_gyroscope_error);

            if self.use_accelerometer_measurements {
                let predicted_accel = current_pose.rot().inverse().apply(
                    (kinematics.linear_acceleration_global + self.gravity.cast::<T>()).as_view(),
                );
                let accel_residual =
                    predicted_accel - measurement.state.linear_acceleration.inner.cast::<T>();
                let whitened_accelerometer_error =
                    self.accelerometer_information_root.cast::<T>() * accel_residual;

                residual
                    .fixed_view_mut::<3, 1>(residuals_per_measurement * index + 3, 0)
                    .copy_from(&whitened_accelerometer_error);
            }
        }

        residual
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use booster::ImuState;
    use factrs::core::SO3;
    use linear_algebra::IntoFramed;
    use nalgebra::{Vector3, vector};

    #[test]
    fn gyro_only_factor_ignores_accelerometer_bias() {
        let now = SystemTime::now();
        let dt = Duration::from_millis(20);

        let mut measurements = Vec::new();
        for i in 0..10 {
            let time = now + Duration::from_secs_f64(i as f64 * dt.as_secs_f64());
            measurements.push(ImuMeasurement {
                time,
                state: ImuState {
                    roll_pitch_yaw: Vector3::zeros().framed(),
                    angular_velocity: Vector3::zeros().framed(),
                    linear_acceleration: vector![0.0, 0.0, 100.0].framed(),
                },
            });
        }
        let imu_factor = IntervalGaussianProcessImuFactor::new(
            measurements,
            Matrix3::identity(),
            Matrix3::identity(),
            false,
            Vector3::new(0., 0., 9.81),
            now,
            now + Duration::from_secs(1),
        );
        let start = SE23::from_rot_vel_trans(SO3::identity(), Vector3::zeros(), Vector3::zeros());
        let end = SE23::from_rot_vel_trans(
            SO3::identity(),
            vector![2.0, 0.0, 0.0],
            vector![1.0, 0.0, 0.0],
        );

        let residual = imu_factor.residuals_on_spline(start, end);

        assert_eq!(residual.len(), 3 * 10);
        for sample_index in 0..10 {
            let offset = 3 * sample_index;
            assert!(residual.fixed_rows::<3>(offset).norm() < 1.0e-9);
        }
    }

    #[test]
    fn stationary_accelerometer_measurement_matches_gravity() {
        let now = SystemTime::now();
        let measurements = vec![ImuMeasurement {
            time: now + Duration::from_millis(100),
            state: ImuState {
                roll_pitch_yaw: Vector3::zeros().framed(),
                angular_velocity: Vector3::zeros().framed(),
                linear_acceleration: vector![0.0, 0.0, 9.81].framed(),
            },
        }];
        let imu_factor = IntervalGaussianProcessImuFactor::new(
            measurements,
            Matrix3::identity(),
            Matrix3::identity(),
            true,
            Vector3::new(0., 0., 9.81),
            now,
            now + Duration::from_secs(1),
        );
        let state: SE23 =
            SE23::from_rot_vel_trans(SO3::identity(), Vector3::zeros(), Vector3::zeros());

        let residual = imu_factor.residuals_on_spline(state.clone(), state);

        assert_eq!(residual.len(), 6);
        assert!(residual.norm() < 1.0e-6, "residual was {residual:?}");
    }
}
