use nalgebra::{
    Matrix4, Quaternion, RealField, Scalar, SimdValue, UnitQuaternion, Vector3, Vector4,
};
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

    fn update_with_gyro(&mut self, gyroscope: &Vector3<T>, sample_period: T);
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
        let F = Vector4::new(
            two*(       q[0]*q[2] - q[3]*q[1]) - accel[0],
            two*(       q[3]*q[0] + q[1]*q[2]) - accel[1],
            two*(half - q[0]*q[0] - q[1]*q[1]) - accel[2],
            zero
        );

        #[rustfmt::skip]
        let J_t = Matrix4::new(
            -two*q[1], two*q[0],       zero, zero,
             two*q[2], two*q[3], -four*q[0], zero,
            -two*q[3], two*q[2], -four*q[1], zero,
             two*q[0], two*q[1],       zero, zero
        );

        // Try to normalize step, falling back to gyro update if not possible
        let Some(step) = (J_t * F).try_normalize(zero) else {
            self.update_with_gyro(gyroscope, sample_period);
            return Ok(());
        };

        // Compute rate of change of quaternion
        let q_dot = (q * Quaternion::from_parts(zero, *gyroscope)) * half
            - Quaternion::new(step[0], step[1], step[2], step[3]) * filter_gain;

        // Integrate to yield quaternion
        *self = UnitQuaternion::from_quaternion(q + q_dot * sample_period);

        Ok(())
    }

    fn update_with_gyro(&mut self, gyroscope: &Vector3<T>, sample_period: T) {
        let q = self.as_ref();

        let zero: T = nalgebra::zero();
        let half: T = nalgebra::convert(0.5);

        // Compute rate of change for quaternion
        let q_dot = q * Quaternion::from_parts(zero, *gyroscope) * half;

        // Integrate to yield quaternion
        *self = UnitQuaternion::from_quaternion(q + q_dot * sample_period);
    }
}
