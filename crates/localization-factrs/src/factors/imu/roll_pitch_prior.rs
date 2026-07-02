use factrs::{
    core::{SO3, Vector2},
    linalg::{ForwardProp, Matrix3, Numeric, VectorX},
    traits::Residual,
    variables::SE23,
};
use nalgebra::Matrix2;

use super::orientation::{local_up_xy_from_so3, roll_pitch_information_root};

#[derive(Debug, Clone)]
pub(crate) struct RollPitchPriorFactor {
    measured_local_up_xy: Vector2<f64>,
    information_root: Matrix2<f64>,
}

#[factrs::mark]
impl Residual for RollPitchPriorFactor {
    type Input = SE23;
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        2
    }

    fn residual<T: Numeric>(&self, pose: SE23<T>) -> VectorX<T> {
        let raw_error = local_up_xy_from_so3(pose.rot()) - self.measured_local_up_xy.cast::<T>();
        let whitened = self.information_root.cast::<T>() * raw_error;

        VectorX::<T>::from_column_slice(whitened.as_slice())
    }
}

impl RollPitchPriorFactor {
    pub(crate) fn new(measured_orientation: SO3, roll_pitch_yaw_noise: Matrix3<f64>) -> Self {
        Self {
            measured_local_up_xy: local_up_xy_from_so3(&measured_orientation),
            information_root: roll_pitch_information_root(roll_pitch_yaw_noise),
        }
    }
}

#[cfg(test)]
mod tests {
    use factrs::{core::Vector3, traits::Residual, variables::SE23};
    use nalgebra::Matrix3;

    use super::*;
    use crate::factors::imu::orientation::so3_from_euler_angles;

    #[test]
    fn roll_pitch_prior_ignores_yaw_offset() {
        let measurement = so3_from_euler_angles(0.1, -0.2, 1.0);
        let predicted = SE23::from_rot_vel_trans(
            so3_from_euler_angles(0.1, -0.2, -2.0),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let factor = RollPitchPriorFactor::new(measurement, Matrix3::identity());

        let residual = factor.residual(predicted);

        assert!(residual.norm() < 1.0e-12);
    }

    #[test]
    fn roll_pitch_prior_stays_finite_near_gimbal_lock() {
        let measurement = so3_from_euler_angles(0.1, 89.9_f64.to_radians(), 1.0);
        let predicted = SE23::from_rot_vel_trans(
            so3_from_euler_angles(0.1, 89.9_f64.to_radians(), -2.0),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let factor = RollPitchPriorFactor::new(measurement, Matrix3::identity());

        let linearized = factor.residual_jacobian(predicted);

        assert!(linearized.value.iter().all(|value| value.is_finite()));
        assert!(linearized.diff.iter().all(|value| value.is_finite()));
        assert!(linearized.value.norm() < 1.0e-6);
    }
}
