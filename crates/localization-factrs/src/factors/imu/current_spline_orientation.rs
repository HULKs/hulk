use std::time::SystemTime;

use factrs::{
    core::{SO3, Vector2},
    linalg::{ForwardProp, Matrix3, Numeric, VectorX},
    traits::Residual,
    variables::SE23,
};
use nalgebra::Matrix2;

use crate::{
    SE23Spline,
    measurements::ImuMeasurement,
    utils::{interval_dt, tau},
};

use super::orientation::{
    local_up_xy_from_so3, orientation_from_measurement, relative_heading_yaw, relative_yaw_error,
    relative_yaw_information_root, roll_pitch_information_root,
};

#[derive(Debug, Clone)]
pub(crate) struct CurrentSplineOrientationFactor {
    measurement_tau: f64,
    measured_local_up_xy: Vector2<f64>,
    measured_relative_yaw: f64,
    roll_pitch_information_root: Matrix2<f64>,
    yaw_information_root: f64,
    duration: f64,
}

#[factrs::mark]
impl Residual for CurrentSplineOrientationFactor {
    type Input = (SE23, SE23);
    type Differ = ForwardProp;

    fn dim_out(&self) -> usize {
        3
    }

    fn residual<T: Numeric>(&self, (start, end): (SE23<T>, SE23<T>)) -> VectorX<T> {
        let spline = SE23Spline::new(start.clone(), end, T::from(self.duration));
        let current_pose = spline.evaluate(T::from(self.measurement_tau));

        let roll_pitch_error =
            local_up_xy_from_so3(current_pose.rot()) - self.measured_local_up_xy.cast::<T>();
        let whitened_roll_pitch = self.roll_pitch_information_root.cast::<T>() * roll_pitch_error;

        let yaw_error =
            relative_yaw_error(start.rot(), current_pose.rot(), self.measured_relative_yaw);

        let mut residual = VectorX::<T>::zeros(3);
        residual
            .fixed_view_mut::<2, 1>(0, 0)
            .copy_from(&whitened_roll_pitch);
        residual[2] = T::from(self.yaw_information_root) * yaw_error;
        residual
    }
}

impl CurrentSplineOrientationFactor {
    pub(crate) fn new(
        measured_start_orientation: SO3,
        current_measurement: &ImuMeasurement,
        roll_pitch_yaw_noise: Matrix3<f64>,
        start_time: SystemTime,
        end_time: SystemTime,
    ) -> Self {
        let measured_current_orientation = orientation_from_measurement(current_measurement);

        Self {
            measurement_tau: tau::<f64>(start_time, end_time, current_measurement.time),
            measured_local_up_xy: local_up_xy_from_so3(&measured_current_orientation),
            measured_relative_yaw: relative_heading_yaw(
                &measured_start_orientation,
                &measured_current_orientation,
            ),
            roll_pitch_information_root: roll_pitch_information_root(roll_pitch_yaw_noise),
            yaw_information_root: relative_yaw_information_root(roll_pitch_yaw_noise),
            duration: interval_dt::<f64>(start_time, end_time),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use booster::ImuState;
    use factrs::{core::Vector3, traits::Residual, variables::SE23};
    use linear_algebra::IntoFramed;
    use nalgebra::{Matrix3, Vector3 as NaVector3, vector};

    use super::*;
    use crate::factors::imu::orientation::so3_from_euler_angles;

    #[test]
    fn current_spline_orientation_yaw_uses_heading_yaw_not_tangent_yaw() {
        let start_time = SystemTime::UNIX_EPOCH;
        let end_time = start_time + Duration::from_secs(1);
        let measured_start = so3_from_euler_angles(0.3, -0.2, 0.1);
        let current_measurement = measurement(end_time, -0.4, 0.2, 0.6);
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
        let factor = CurrentSplineOrientationFactor::new(
            measured_start,
            &current_measurement,
            Matrix3::identity(),
            start_time,
            end_time,
        );

        let residual = factor.residual((predicted_start, predicted_end));

        assert!(residual[2].abs() < 1.0e-6);
    }

    fn measurement(time: SystemTime, roll: f64, pitch: f64, yaw: f64) -> ImuMeasurement {
        ImuMeasurement {
            time,
            state: ImuState {
                roll_pitch_yaw: vector![roll as f32, pitch as f32, yaw as f32].framed(),
                angular_velocity: NaVector3::zeros().framed(),
                linear_acceleration: NaVector3::zeros().framed(),
            },
        }
    }
}
