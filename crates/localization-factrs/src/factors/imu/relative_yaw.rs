use factrs::{
    core::SO3,
    linalg::{ForwardProp, Matrix3, Numeric, VectorX},
    traits::Residual,
    variables::SE23,
};

use super::orientation::{relative_heading_yaw, relative_yaw_error, relative_yaw_information_root};

#[derive(Debug, Clone)]
pub(crate) struct RelativeYawFactor {
    measured_relative_yaw: f64,
    information_root: f64,
}

#[factrs::mark]
impl Residual for RelativeYawFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        1
    }

    fn residual<T: Numeric>(&self, (start, end): (SE23<T>, SE23<T>)) -> VectorX<T> {
        let raw_error = relative_yaw_error(start.rot(), end.rot(), self.measured_relative_yaw);

        VectorX::<T>::from_element(1, T::from(self.information_root) * raw_error)
    }
}

impl RelativeYawFactor {
    pub(crate) fn new(
        measured_start_orientation: SO3,
        measured_end_orientation: SO3,
        roll_pitch_yaw_noise: Matrix3<f64>,
    ) -> Self {
        Self {
            measured_relative_yaw: relative_heading_yaw(
                &measured_start_orientation,
                &measured_end_orientation,
            ),
            information_root: relative_yaw_information_root(roll_pitch_yaw_noise),
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
    fn relative_yaw_factor_ignores_constant_yaw_offset() {
        let measured_start = so3_from_euler_angles(0.0, 0.0, 0.1);
        let measured_end = so3_from_euler_angles(0.0, 0.0, 0.6);
        let predicted_start = SE23::from_rot_vel_trans(
            so3_from_euler_angles(0.0, 0.0, 1.2),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let predicted_end = SE23::from_rot_vel_trans(
            so3_from_euler_angles(0.0, 0.0, 1.7),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let factor = RelativeYawFactor::new(measured_start, measured_end, Matrix3::identity());

        let residual = factor.residual((predicted_start, predicted_end));

        assert!(residual.norm() < 1.0e-12);
    }

    #[test]
    fn relative_yaw_factor_uses_heading_yaw_not_tangent_yaw() {
        let measured_start = so3_from_euler_angles(0.3, -0.2, 0.1);
        let measured_end = so3_from_euler_angles(-0.4, 0.2, 0.6);
        let predicted_start = SE23::from_rot_vel_trans(
            so3_from_euler_angles(-0.7, 0.3, 1.0),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let predicted_end = SE23::from_rot_vel_trans(
            so3_from_euler_angles(0.6, -0.4, 1.5),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let factor = RelativeYawFactor::new(measured_start, measured_end, Matrix3::identity());

        let residual = factor.residual((predicted_start, predicted_end));

        assert!(residual[0].abs() < 1.0e-12);
    }

    #[test]
    fn relative_yaw_factor_wraps_yaw_delta_residual() {
        let measured_start = so3_from_euler_angles(0.0, 0.0, 179.0_f64.to_radians());
        let measured_end = so3_from_euler_angles(0.0, 0.0, -179.0_f64.to_radians());
        let predicted = SE23::from_rot_vel_trans(
            so3_from_euler_angles(0.0, 0.0, 179.0_f64.to_radians()),
            Vector3::zeros(),
            Vector3::zeros(),
        );
        let factor = RelativeYawFactor::new(measured_start, measured_end, Matrix3::identity());

        let residual = factor.residual((predicted.clone(), predicted));

        assert!((residual[0] + 2.0_f64.to_radians() / 2.0_f64.sqrt()).abs() < 1.0e-12);
    }

    #[test]
    fn relative_yaw_factor_stays_finite_near_gimbal_lock() {
        let measured_start = so3_from_euler_angles(0.1, 89.9_f64.to_radians(), 0.1);
        let measured_end = so3_from_euler_angles(0.1, 89.9_f64.to_radians(), 0.6);
        let predicted_start =
            SE23::from_rot_vel_trans(measured_start.clone(), Vector3::zeros(), Vector3::zeros());
        let predicted_end =
            SE23::from_rot_vel_trans(measured_end.clone(), Vector3::zeros(), Vector3::zeros());
        let factor = RelativeYawFactor::new(measured_start, measured_end, Matrix3::identity());

        let linearized = factor.residual_jacobian((predicted_start, predicted_end));

        assert!(linearized.value.iter().all(|value| value.is_finite()));
        assert!(linearized.diff.iter().all(|value| value.is_finite()));
        assert!(linearized.value.norm() < 1.0e-12);
    }
}
