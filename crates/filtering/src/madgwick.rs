use nalgebra::{matrix, vector, Quaternion, RealField, Scalar, SimdValue, UnitQuaternion, Vector3};
use num_traits::{One, Zero};

#[derive(Debug)]
pub struct AccelerometerNormZero;

/// Madgwick's filter for orientation estimation.
///
/// This implementation is based on the following:
/// https://github.com/jmagnuson/ahrs-rs
pub trait Madgwick<T> {
    fn update_with_imu(
        &mut self,
        gyroscope: &Vector3<T>,
        accelerometer: &Vector3<T>,
        filter_gain: T,
        sample_period: T,
    ) -> Result<(), AccelerometerNormZero>;

    fn update_with_gyroscope(&mut self, gyroscope: &Vector3<T>, sample_period: T);
}

impl<T> Madgwick<T> for UnitQuaternion<T>
where
    T: Scalar + SimdValue + RealField + One + Zero + Copy,
{
    #[allow(non_snake_case)]
    fn update_with_imu(
        &mut self,
        gyroscope: &Vector3<T>,
        accelerometer: &Vector3<T>,
        filter_gain: T,
        sample_period: T,
    ) -> Result<(), AccelerometerNormZero> {
        let q = self.as_ref();

        let zero: T = nalgebra::zero();
        let two: T = nalgebra::convert(2.0);
        let four: T = nalgebra::convert(4.0);
        let half: T = nalgebra::convert(0.5);

        // Normalize accelerometer measurement
        let Some(accel) = accelerometer.try_normalize(zero) else {
            return Err(AccelerometerNormZero);
        };

        // Gradient descent algorithm corrective step
        #[rustfmt::skip]
        let F = vector![
            two * (       q.i * q.k - q.w * q.j) - accel.x,
            two * (       q.w * q.i + q.j * q.k) - accel.y,
            two * (half - q.i * q.i - q.j * q.j) - accel.z,
            zero
        ];

        #[rustfmt::skip]
        let J_t = matrix![
            -two * q.j, two * q.i,        zero, zero;
             two * q.k, two * q.w, -four * q.i, zero;
            -two * q.w, two * q.k, -four * q.j, zero;
             two * q.i, two * q.j,        zero, zero
        ];

        // Try to normalize step, falling back to gyro update if not possible
        let Some(step) = (J_t * F).try_normalize(zero) else {
            self.update_with_gyroscope(gyroscope, sample_period);
            return Ok(());
        };

        // Compute rate of change of quaternion
        let q_dot = (q * Quaternion::from_parts(zero, *gyroscope)) * half
            - Quaternion::from_vector(step) * filter_gain;

        // Integrate to yield quaternion
        *self = UnitQuaternion::from_quaternion(q + q_dot * sample_period);

        Ok(())
    }

    fn update_with_gyroscope(&mut self, gyroscope: &Vector3<T>, sample_period: T) {
        let q = self.as_ref();

        let zero: T = nalgebra::zero();
        let half: T = nalgebra::convert(0.5);

        // Compute rate of change for quaternion
        let q_dot = q * Quaternion::from_parts(zero, *gyroscope) * half;

        // Integrate to yield quaternion
        *self = UnitQuaternion::from_quaternion(q + q_dot * sample_period);
    }
}
