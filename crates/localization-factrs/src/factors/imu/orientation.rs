use std::time::SystemTime;

use factrs::{
    core::{SO3, Vector2, Vector3},
    linalg::{Matrix3, Numeric},
    traits::Variable,
    variables::MatrixLieGroup,
};
use nalgebra::{Matrix2, UnitQuaternion};

use crate::measurements::ImuMeasurement;

pub(crate) fn interpolate_measurement_orientation(
    previous: &ImuMeasurement,
    next: &ImuMeasurement,
    time: SystemTime,
) -> SO3 {
    let interval = next
        .time
        .duration_since(previous.time)
        .expect("IMU measurements must be sorted by time")
        .as_secs_f64();
    if interval == 0.0 {
        return orientation_from_measurement(previous);
    }

    let elapsed = time
        .duration_since(previous.time)
        .expect("interpolation time must be after previous IMU measurement")
        .as_secs_f64();
    let alpha = elapsed / interval;

    geodesic_interpolate(
        orientation_from_measurement(previous),
        orientation_from_measurement(next),
        alpha,
    )
}

pub(super) fn roll_pitch_information_root(roll_pitch_yaw_noise: Matrix3<f64>) -> Matrix2<f64> {
    roll_pitch_yaw_noise
        .fixed_view::<2, 2>(0, 0)
        .into_owned()
        .cholesky()
        .expect("roll/pitch noise covariance must be positive definite")
        .l()
        .try_inverse()
        .expect("roll/pitch lower triangular matrix must be invertible")
}

pub(super) fn relative_yaw_information_root(roll_pitch_yaw_noise: Matrix3<f64>) -> f64 {
    let yaw_variance = roll_pitch_yaw_noise[(2, 2)];
    assert!(yaw_variance > 0.0, "yaw variance must be positive");

    // The relative-yaw residual differences two absolute yaw samples.
    (2.0 * yaw_variance).sqrt().recip()
}

pub(super) fn orientation_from_measurement(measurement: &ImuMeasurement) -> SO3 {
    let rpy = measurement.state.roll_pitch_yaw.inner.cast::<f64>();
    so3_from_euler_angles(rpy.x, rpy.y, rpy.z)
}

pub(super) fn local_up_xy_from_so3<T: Numeric>(rotation: &SO3<T>) -> Vector2<T> {
    let field_up = Vector3::new(T::zero(), T::zero(), T::one());
    let local_up = rotation.inverse().apply(field_up.as_view());
    local_up.fixed_rows::<2>(0).into_owned()
}

pub(super) fn relative_yaw_error<T: Numeric>(
    predicted_start: &SO3<T>,
    predicted_end: &SO3<T>,
    measured_relative_yaw: f64,
) -> T {
    wrap_angle(
        relative_heading_yaw(predicted_start, predicted_end) - T::from(measured_relative_yaw),
    )
}

fn heading_yaw<T: Numeric>(rotation: &SO3<T>) -> T {
    let one = T::one();
    let two = T::from(2.0);

    let w = rotation.w();
    let x = rotation.x();
    let y = rotation.y();
    let z = rotation.z();

    (two * (w * z + x * y)).atan2(one - two * (y * y + z * z))
}

fn wrap_angle<T: Numeric>(angle: T) -> T {
    angle.sin().atan2(angle.cos())
}

pub(super) fn relative_heading_yaw<T: Numeric>(start: &SO3<T>, end: &SO3<T>) -> T {
    wrap_angle(heading_yaw(end) - heading_yaw(start))
}

fn geodesic_interpolate(start: SO3, end: SO3, alpha: f64) -> SO3 {
    let phi = start.inverse().compose(&end).log();
    start.oplus_right((phi * alpha).as_view())
}

#[cfg(test)]
pub(super) fn roll_pitch_yaw_from_so3<T: Numeric>(rotation: &SO3<T>) -> Vector3<T> {
    let one = T::from(1.0);
    let two = T::from(2.0);

    let w = rotation.w();
    let x = rotation.x();
    let y = rotation.y();
    let z = rotation.z();

    let roll = (two * (w * x + y * z)).atan2(one - two * (x * x + y * y));
    let pitch = (two * (w * y - z * x)).asin();
    let yaw = (two * (w * z + x * y)).atan2(one - two * (y * y + z * z));

    Vector3::new(roll, pitch, yaw)
}

pub(super) fn so3_from_euler_angles(roll: f64, pitch: f64, yaw: f64) -> SO3 {
    let quaternion = UnitQuaternion::from_euler_angles(roll, pitch, yaw);
    SO3::from_xyzw(quaternion.i, quaternion.j, quaternion.k, quaternion.w)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use booster::ImuState;
    use linear_algebra::IntoFramed;
    use nalgebra::{Vector3, vector};

    use super::*;

    #[test]
    fn interpolates_orientation_geodesically() {
        let now = SystemTime::now();
        let measurements = [
            measurement(now, 0.0),
            measurement(now + Duration::from_secs(1), 90.0_f64.to_radians()),
        ];

        let orientation = interpolate_measurement_orientation(
            &measurements[0],
            &measurements[1],
            now + Duration::from_millis(500),
        );
        let rpy = roll_pitch_yaw_from_so3(&orientation);

        assert!((rpy.z - 45.0_f64.to_radians()).abs() < 1.0e-7);
    }

    fn measurement(time: SystemTime, yaw: f64) -> ImuMeasurement {
        ImuMeasurement {
            time,
            state: ImuState {
                roll_pitch_yaw: vector![0.0, 0.0, yaw as f32].framed(),
                angular_velocity: Vector3::zeros().framed(),
                linear_acceleration: Vector3::zeros().framed(),
            },
        }
    }
}
