use std::time::Duration;

use nalgebra::{matrix, vector, Quaternion, UnitQuaternion, Vector3};

#[derive(Debug)]
pub struct AccelerometerNormZero;

/// Madgwick's filter for orientation estimation.
///
/// This implementation is based on the following:
/// <https://github.com/jmagnuson/ahrs-rs>
pub trait Madgwick {
    fn update_with_imu(
        &mut self,
        gyroscope: Vector3<f32>,
        accelerometer: Vector3<f32>,
        filter_gain: f32,
        sample_period: Duration,
    ) -> Result<(), AccelerometerNormZero>;

    fn update_with_gyroscope(&mut self, gyroscope: Vector3<f32>, sample_period: Duration);
}

impl Madgwick for UnitQuaternion<f32> {
    #[allow(non_snake_case)]
    fn update_with_imu(
        &mut self,
        gyroscope: Vector3<f32>,
        accelerometer: Vector3<f32>,
        filter_gain: f32,
        sample_period: Duration,
    ) -> Result<(), AccelerometerNormZero> {
        let q = self.as_ref();

        // Normalize accelerometer measurement
        let Some(accel) = accelerometer.try_normalize(0.0) else {
            return Err(AccelerometerNormZero);
        };

        // Gradient descent algorithm corrective step
        #[rustfmt::skip]
        let F = vector![
            2.0 * (      q.i * q.k - q.w * q.j) - accel.x,
            2.0 * (      q.w * q.i + q.j * q.k) - accel.y,
            2.0 * (0.5 - q.i * q.i - q.j * q.j) - accel.z,
            0.0
        ];

        let J_t = matrix![
            -2.0 * q.j, 2.0 * q.i,        0.0, 0.0;
             2.0 * q.k, 2.0 * q.w, -4.0 * q.i, 0.0;
            -2.0 * q.w, 2.0 * q.k, -4.0 * q.j, 0.0;
             2.0 * q.i, 2.0 * q.j,        0.0, 0.0
        ];

        // Try to normalize step, falling back to gyro update if not possible
        let Some(step) = (J_t * F).try_normalize(0.0) else {
            self.update_with_gyroscope(gyroscope, sample_period);
            return Ok(());
        };

        // Compute rate of change of quaternion
        let q_dot = (q * Quaternion::from_parts(0.0, gyroscope)) * 0.5
            - Quaternion::from_vector(step) * filter_gain;

        // Integrate to yield quaternion
        let dt = sample_period.as_secs_f32();
        *self = UnitQuaternion::from_quaternion(q + q_dot * dt);

        Ok(())
    }

    fn update_with_gyroscope(&mut self, gyroscope: Vector3<f32>, sample_period: Duration) {
        let q = self.as_ref();

        // Compute rate of change for quaternion
        let q_dot = q * Quaternion::from_parts(0.0, gyroscope) * 0.5;

        // Integrate to yield quaternion
        let dt = sample_period.as_secs_f32();
        *self = UnitQuaternion::from_quaternion(q + q_dot * dt);
    }
}
